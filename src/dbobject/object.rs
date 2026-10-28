//-----------------------------------------------------------------------------
// Copyright (c) 2026, Oracle and/or its affiliates.
//
// This software is dual-licensed to you under the Universal Permissive License
// (UPL) 1.0 as shown at https://oss.oracle.com/licenses/upl and Apache License
// 2.0 as shown at http://www.apache.org/licenses/LICENSE-2.0. You may choose
// either license.
//
// If you elect to accept the software under the Apache License, Version 2.0,
// the following applies:
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//-----------------------------------------------------------------------------

//-----------------------------------------------------------------------------
// object.rs
//
// Retains database object images and decodes them when taken from a row.
//-----------------------------------------------------------------------------

use std::sync::Arc;

use super::DbObjectType;
use crate::client::Client;
use crate::client::ClientRef;
use crate::column_index::ColumnIndex;
use crate::constants;
use crate::db_value::{DbValue, FromDbValue};
use crate::error::Error;
use crate::lob::{Lob, PendingLobData};
use crate::metadata::Metadata;
use crate::read_buffer::FromBuf;
use crate::read_buffer::ReadBuffer;
use crate::response::Response;
use crate::row::{ColumnData, DbRow, Row};

const TTC_OBJ_ATOMIC_NULL: u8 = 253;
const TTC_OBJ_IS_DEGENERATE: u8 = 0x10;
const TTC_OBJ_NO_PREFIX_SEG: u8 = 0x04;
const TTC_OBJ_PLSQL_INDEX_TABLE: u8 = 1;

/// A fetched database collection.
pub struct DbCollection {
    object_type: Arc<DbObjectType>,
    values: DbRow,
}

/// A fetched database object.
pub struct DbObject {
    object_type: Arc<DbObjectType>,
    data: Row,
}

/// A simple enumeration to simplify decoding of images which may contain a
/// database object or a collection.
enum DbObjectOrCollection {
    DbCollection(DbCollection),
    DbObject(DbObject),
}

/// Holds an object or collection image until its type has been resolved.
pub(crate) struct PendingDbObject {
    bytes: Vec<u8>,
}

impl DbCollection {
    /// Creates a new database collection.
    fn new(object_type: &Arc<DbObjectType>, values: DbRow) -> Self {
        Self { object_type: Arc::clone(object_type), values }
    }

    /// Transforms an owned DbCollection into a vector of the requested type.
    fn transform_owned<'a, T>(self) -> Result<Vec<T>, Error>
    where
        T: FromDbValue<'a>,
    {
        self.values.transform_owned()
    }

    /// Returns the type describing this collection.
    pub fn object_type(&self) -> &Arc<DbObjectType> {
        &self.object_type
    }
}

impl DbObject {
    /// Creates a new database object.
    fn new(object_type: &Arc<DbObjectType>, data: Row) -> Self {
        Self { object_type: Arc::clone(object_type), data }
    }

    /// Returns an object attribute by name or zero-based position.
    pub fn get<'a, T: FromDbValue<'a>>(
        &'a self,
        index: impl ColumnIndex,
    ) -> Result<T, Error> {
        self.data.get(index)
    }

    /// Returns the type describing this value.
    pub fn object_type(&self) -> &Arc<DbObjectType> {
        &self.object_type
    }

    /// Transfers an object attribute to the caller, replacing it with NULL.
    pub fn take<'a, T: FromDbValue<'a>>(
        &'a mut self,
        index: impl ColumnIndex,
    ) -> Result<T, Error> {
        self.data.take(index)
    }
}

impl PendingDbObject {
    /// Reads the TTC envelope and retains the packed image.
    pub(crate) fn from_response(
        resp: &mut Response,
    ) -> Result<Option<Self>, Error> {
        let _type_oid = resp.read_bytes_with_double_length()?;
        let _instance_oid = resp.read_bytes_with_double_length()?;
        let _snapshot = resp.read_bytes_with_double_length()?;
        let _version = resp.read_ub2()?;
        let size = resp.read_ub4()?;
        let _flags = resp.read_ub2()?;
        if size == 0 {
            Ok(None)
        } else {
            Ok(Some(Self {
                bytes: resp.read_bytes_with_length()?.into_owned(),
            }))
        }
    }

    /// Attempt to decode a database object and return it upon success.
    pub(crate) fn try_into_db_value(
        self,
        object_type: &Arc<DbObjectType>,
        client: &Client,
        client_ref: &ClientRef,
    ) -> Result<DbValue, Error> {
        let mut buf = DbObjectReadBuffer::new(&self.bytes, client, client_ref);
        match buf.decode_image(object_type)? {
            DbObjectOrCollection::DbCollection(coll) => {
                Ok(DbValue::DbCollection(coll))
            }
            DbObjectOrCollection::DbObject(obj) => Ok(DbValue::DbObject(obj)),
        }
    }
}

impl<T> TryFrom<DbCollection> for Vec<T>
where
    for<'a> T: FromDbValue<'a>,
{
    type Error = Error;

    fn try_from(source: DbCollection) -> Result<Self, Self::Error> {
        source.transform_owned()
    }
}

impl<'a> FromDbValue<'a> for DbObject {
    fn from_db_value(data: ColumnData<'a>) -> Result<Self, Error> {
        match data {
            ColumnData::Owned(Some(DbValue::DbObject(obj))) => Ok(obj),
            _ => Err(Self::unsupported_conversion(data)),
        }
    }
}

impl<'a> FromDbValue<'a> for DbCollection {
    fn from_db_value(data: ColumnData<'a>) -> Result<Self, Error> {
        match data {
            ColumnData::Owned(Some(DbValue::DbCollection(coll))) => Ok(coll),
            _ => Err(Self::unsupported_conversion(data)),
        }
    }
}

struct DbObjectReadBuffer<'a> {
    buf: ReadBuffer,
    client: &'a Client,
    client_ref: &'a ClientRef,
}

impl DbObjectReadBuffer<'_> {
    /// Decodes the body of a database object or collection and returns a
    /// vector of database values adn a vector of indices (for collections).
    fn decode_body(
        &mut self,
        object_type: &Arc<DbObjectType>,
    ) -> Result<DbObjectOrCollection, Error> {
        let mut values = Vec::new();
        if object_type.is_collection() {
            let collection_type = object_type.collection_type();
            let metadata = object_type.element_metadata().unwrap();
            let _collection_flags = self.buf.read_u8()?;
            let num_elements = self.read_length()?;
            for _ in 0..num_elements {
                if collection_type == Some(TTC_OBJ_PLSQL_INDEX_TABLE) {
                    let _index = self.buf.read_u32be()?;
                }
                values.push(self.decode_value(object_type, metadata)?);
            }
            Ok(DbObjectOrCollection::DbCollection(DbCollection::new(
                object_type,
                DbRow::new(values),
            )))
        } else {
            for metadata in object_type.attributes() {
                values.push(self.decode_value(object_type, metadata)?);
            }
            let row = Row::new(
                &Arc::new(object_type.attributes().to_vec()),
                DbRow::new(values),
            );
            Ok(DbObjectOrCollection::DbObject(DbObject::new(object_type, row)))
        }
    }

    /// Decodes a database object or collection image and returns a vector of
    /// database values and a vector of indices (for collections).
    fn decode_image(
        &mut self,
        object_type: &Arc<DbObjectType>,
    ) -> Result<DbObjectOrCollection, Error> {
        let flags = self.buf.read_u8()?;
        let _image_version = self.buf.read_u8()?;
        let _image_size = self.read_length()?;
        if flags & TTC_OBJ_IS_DEGENERATE != 0 {
            return Err(Error::not_implemented(
                "database object stored in a LOB".into(),
            ));
        }
        if flags & TTC_OBJ_NO_PREFIX_SEG == 0 {
            let length = self.read_length()?;
            let _prefix_seg = self.buf.read_bytes(length)?;
        }
        self.decode_body(object_type)
    }

    /// Decodes a single value from the image.
    fn decode_value(
        &mut self,
        object_type: &Arc<DbObjectType>,
        metadata: &Metadata,
    ) -> Result<Option<DbValue>, Error> {
        let client = self.client;
        let client_ref = self.client_ref;
        let marker = self.peek_u8()?;
        let typ = metadata.db_type();
        if typ == crate::DB_TYPE_OBJECT {
            let child_object_type = metadata
                .object_type()?
                .ok_or_else(Error::db_object_type_unavailable)?;
            let has_image = object_type.is_collection()
                || child_object_type.is_collection();
            if marker == TTC_OBJ_ATOMIC_NULL
                || (has_image
                    && marker == constants::TTC_NULL_LENGTH_INDICATOR)
            {
                self.buf.read_u8()?;
                return Ok(None);
            }
            let result = if has_image {
                let mut child_buf = DbObjectReadBuffer::new(
                    self.read_bytes_with_length()?,
                    client,
                    client_ref,
                );
                child_buf.decode_image(&child_object_type)?
            } else {
                self.decode_body(&child_object_type)?
            };
            return match result {
                DbObjectOrCollection::DbCollection(coll) => {
                    Ok(Some(DbValue::DbCollection(coll)))
                }
                DbObjectOrCollection::DbObject(obj) => {
                    Ok(Some(DbValue::DbObject(obj)))
                }
            };
        }
        if marker == 0 || marker == constants::TTC_NULL_LENGTH_INDICATOR {
            self.buf.read_u8()?;
            return Ok(None);
        }
        let bytes = self.read_bytes_with_length()?;
        validate_scalar_length(typ, bytes.len())?;
        let scalar = match typ {
            crate::DB_TYPE_CHAR
            | crate::DB_TYPE_VARCHAR
            | crate::DB_TYPE_NCHAR
            | crate::DB_TYPE_NVARCHAR => {
                let charset = {
                    if typ.csfrm == constants::CS_FORM_NCHAR {
                        client.get_ncharset_id()
                    } else {
                        client.get_charset_id()
                    }
                };
                let value = match charset {
                    crate::constants::CHARSET_ID_UTF8 => {
                        String::from_utf8(bytes.to_vec())?
                    }
                    crate::constants::CHARSET_ID_UTF16 => {
                        crate::utils::utf16be_bytes_to_string(bytes)?
                    }
                    _ => {
                        return Err(Error::not_implemented(format!(
                            "object character set {charset}"
                        )));
                    }
                };
                DbValue::String(value)
            }
            crate::DB_TYPE_RAW => DbValue::Bytes(bytes.to_vec()),
            crate::DB_TYPE_NUMBER => {
                DbValue::Number(crate::OracleNumber::from_buf(bytes))
            }
            crate::DB_TYPE_BINARY_INTEGER => {
                let value =
                    i32::from_be_bytes(bytes.to_vec().try_into().unwrap());
                DbValue::Number(value.to_string().parse()?)
            }
            crate::DB_TYPE_BINARY_FLOAT => {
                DbValue::BinaryFloat(f32::from_buf(bytes))
            }
            crate::DB_TYPE_BINARY_DOUBLE => {
                DbValue::BinaryDouble(f64::from_buf(bytes))
            }
            crate::DB_TYPE_BOOLEAN => DbValue::Boolean(bool::from_buf(bytes)),
            crate::DB_TYPE_INTERVAL_DS => {
                DbValue::IntervalDS(crate::OracleIntervalDS::from_buf(bytes))
            }
            crate::DB_TYPE_INTERVAL_YM => {
                DbValue::IntervalYM(crate::OracleIntervalYM::from_buf(bytes))
            }
            value if value.is_date_type() => {
                if bytes.len() == 13 && bytes[11] & 0x80 != 0 {
                    return Err(Error::not_implemented(
                        "object timestamp with a time zone region".into(),
                    ));
                }
                DbValue::Timestamp(crate::OracleTimestamp::from_buf(bytes))
            }
            crate::DB_TYPE_CLOB
            | crate::DB_TYPE_NCLOB
            | crate::DB_TYPE_BLOB
            | crate::DB_TYPE_BFILE => DbValue::Lob(Lob::new(
                client_ref.clone(),
                PendingLobData::from_locator(typ, bytes.to_vec()),
            )),
            _ => return Err(Error::unsupported_db_type(typ)),
        };
        Ok(Some(scalar))
    }

    /// Creates a new buffer for reading database object and collections.
    fn new<'a>(
        data: &[u8],
        client: &'a Client,
        client_ref: &'a ClientRef,
    ) -> DbObjectReadBuffer<'a> {
        DbObjectReadBuffer {
            buf: ReadBuffer::from_bytes(data),
            client,
            client_ref,
        }
    }

    /// Peeks at the next byte without advancing the reader.
    fn peek_u8(&mut self) -> Result<u8, Error> {
        let pos = self.buf.get_pos();
        let value = self.buf.read_u8()?;
        self.buf.set_pos(pos);
        Ok(value)
    }

    /// Reads bytes prefixed by an object-image length.
    fn read_bytes_with_length(&mut self) -> Result<&[u8], Error> {
        let length = self.read_length()?;
        self.buf.read_bytes(length)
    }

    /// Reads a short or extended object-image length.
    fn read_length(&mut self) -> Result<usize, Error> {
        let length = self.buf.read_u8()?;
        if length == crate::constants::TTC_LONG_LENGTH_INDICATOR {
            Ok(self.buf.read_u32be()? as usize)
        } else {
            Ok(length as usize)
        }
    }
}

/// Validates the encoded length of a scalar object value.
fn validate_scalar_length(
    typ: &'static crate::DbType,
    length: usize,
) -> Result<(), Error> {
    use crate::constants;

    let expected = match typ {
        crate::DB_TYPE_BINARY_FLOAT => {
            Some(constants::ORA_TYPE_SIZE_BINARY_FLOAT)
        }
        crate::DB_TYPE_BINARY_INTEGER => {
            Some(constants::ORA_TYPE_SIZE_BINARY_INTEGER)
        }
        crate::DB_TYPE_BOOLEAN => Some(constants::ORA_TYPE_SIZE_BOOLEAN),
        crate::DB_TYPE_BINARY_DOUBLE => {
            Some(constants::ORA_TYPE_SIZE_BINARY_DOUBLE)
        }
        crate::DB_TYPE_INTERVAL_DS => {
            Some(constants::ORA_TYPE_SIZE_INTERVAL_DS)
        }
        crate::DB_TYPE_INTERVAL_YM => {
            Some(constants::ORA_TYPE_SIZE_INTERVAL_YM)
        }
        _ => None,
    };
    let valid_date_length = matches!(
        length,
        constants::ORA_TYPE_SIZE_DATE
            | constants::ORA_TYPE_SIZE_TIMESTAMP
            | constants::ORA_TYPE_SIZE_TIMESTAMP_TZ
    );
    if expected.is_some_and(|size| length != size)
        || (typ == crate::DB_TYPE_NUMBER && length == 0)
        || (typ.is_date_type() && !valid_date_length)
    {
        return Err(Error::unexpected_result());
    }
    Ok(())
}
