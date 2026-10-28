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
// metadata.rs
//
// Defines the structure representing column/bind metadata.
//-----------------------------------------------------------------------------

use std::sync::Arc;

use crate::client::Client;
use crate::constants;
use crate::db_type::DbType;
use crate::dbobject::{DbObjectType, DbObjectTypeIdentity};
use crate::error::Error;
use crate::response::Response;
use crate::vector::VectorStorageFormat;
use crate::write_buffer::WriteBuffer;

// vector metadata flags
const VECTOR_META_FLAG_FLEXIBLE_DIM: u8 = 0x01;
const VECTOR_META_FLAG_SPARSE_VECTOR: u8 = 0x02;

/// An object type awaiting discovery or its shared type handle.
#[derive(Clone, Debug)]
enum MetadataObjectType {
    Pending(DbObjectTypeIdentity),
    Resolved(Arc<DbObjectType>),
}

/// Represents the metadata of data fetched from the database or data that will
/// be sent to the database (columns, bind variables, database object
/// attributes, database collection elements).
#[derive(Clone, Debug)]
pub struct Metadata {
    name: String,
    db_type: &'static DbType,
    null_by_describe: bool,
    nullable: bool,
    precision: u8,
    scale: i8,
    max_size: u32,
    is_array: bool,
    vector_storage_format: Option<VectorStorageFormat>,
    vector_dimensions: u32,
    vector_flags: u8,
    object_type: Option<MetadataObjectType>,
    domain: Option<(String, String)>,
    annotations: Vec<(String, String)>,
}

impl Metadata {
    /// Returns a new structure with the given values.
    fn new(
        name: String,
        db_type: &'static DbType,
        max_size: usize,
        precision: u8,
        scale: i8,
        nullable: bool,
        is_array: bool,
    ) -> Self {
        let actual_max_size: u32 = if max_size == 0 {
            db_type.default_size
        } else {
            max_size.try_into().unwrap()
        };
        Self {
            name,
            db_type,
            nullable,
            null_by_describe: false,
            precision,
            scale,
            max_size: actual_max_size,
            is_array,
            vector_storage_format: None,
            vector_dimensions: 0,
            vector_flags: 0,
            object_type: None,
            domain: None,
            annotations: Vec::new(),
        }
    }

    /// Returns a new structure suitable for defining CLOB/BLOB as
    /// string/bytes.
    pub(crate) fn define_metadata(&self) -> Metadata {
        let mut metadata = self.clone();
        metadata.max_size = 0;
        metadata.db_type = match self.db_type {
            crate::DB_TYPE_BLOB => crate::DB_TYPE_LONG_RAW,
            crate::DB_TYPE_CLOB => crate::DB_TYPE_LONG,
            crate::DB_TYPE_NCLOB => crate::DB_TYPE_LONG_NVARCHAR,
            _ => metadata.db_type,
        };
        metadata
    }

    /// Returns a new structure from the metadata returned by the database.
    pub(crate) fn from_response(
        resp: &mut Response,
        client: &Client,
    ) -> Result<Self, Error> {
        let mut metadata = Self {
            name: String::new(),
            db_type: crate::DB_TYPE_UNKNOWN,
            nullable: false,
            precision: 0,
            scale: 0,
            null_by_describe: false,
            max_size: 0,
            is_array: false,
            vector_storage_format: None,
            vector_dimensions: 0,
            vector_flags: 0,
            object_type: None,
            domain: None,
            annotations: Vec::new(),
        };
        let ora_type_num: u16 = resp.read_u8()?.into();
        resp.read_u8()?; // flags
        metadata.precision = resp.read_u8()?;
        metadata.scale = resp.read_i8()?;
        let buffer_size = resp.read_ub4()?;
        let _max_num_array_elements = resp.read_ub4()?;
        let _cont_flags = resp.read_ub8()?;
        let _oid = resp.read_bytes_with_double_length()?;
        let _version = resp.read_ub2()?;
        let _charset_id = resp.read_ub2()?;
        let csfrm = resp.read_u8()?;
        metadata.db_type =
            DbType::from_ora_type_and_csfrm(ora_type_num, csfrm);
        metadata.max_size = resp.read_ub4()?;
        if ora_type_num == constants::ORA_TYPE_NUM_RAW {
            metadata.max_size = buffer_size;
        }
        if client.supports_ttc_field_version(constants::TTC_FIELD_VERSION_12_2)
        {
            let _oaccolid = resp.read_ub4()?;
        }
        metadata.nullable = resp.read_u8()? != 0;
        let _v7_name_length = resp.read_u8()?;
        metadata.name = resp.read_utf8_with_double_length()?.to_string();
        let obj_schema = resp.read_utf8_with_double_length()?.into_owned();
        let obj_name = resp.read_utf8_with_double_length()?.into_owned();
        if metadata.db_type == crate::DB_TYPE_OBJECT {
            metadata.set_object_type_identity(DbObjectTypeIdentity {
                schema: obj_schema,
                package_name: None,
                name: obj_name,
            });
        }
        let _column_position = resp.read_ub2()?;
        let _uds_flags = resp.read_ub4()?;
        if client.supports_ttc_field_version(constants::TTC_FIELD_VERSION_23_1)
        {
            let schema = resp.read_utf8_with_double_length()?.to_string();
            let name = resp.read_utf8_with_double_length()?.to_string();
            if !schema.is_empty() && !name.is_empty() {
                metadata.domain = Some((schema, name));
            }
        }
        if client.supports_ttc_field_version(
            constants::TTC_FIELD_VERSION_23_1_EXT_3,
        ) && resp.read_ub4()? > 0
        {
            resp.read_u8()?;
            let num_annotations = resp.read_ub4()?;
            resp.read_u8()?;
            for _ in 0..num_annotations {
                let name = resp.read_utf8_with_double_length()?.to_string();
                let value = resp.read_utf8_with_double_length()?.to_string();
                metadata.annotations.push((name, value));
                let _flags = resp.read_ub4()?;
            }
            let _flags = resp.read_ub4()?;
        }
        if client.supports_ttc_field_version(constants::TTC_FIELD_VERSION_23_4)
        {
            metadata.vector_dimensions = resp.read_ub4()?;
            metadata.vector_storage_format = match resp.read_u8()? {
                0 => None,
                v => Some(VectorStorageFormat::try_from(v)?),
            };
            metadata.vector_flags = resp.read_u8()?;
        }
        metadata.null_by_describe = match ora_type_num {
            constants::ORA_TYPE_NUM_LONG
            | constants::ORA_TYPE_NUM_LONG_RAW
            | constants::ORA_TYPE_NUM_UROWID => false,
            _ => buffer_size == 0,
        };
        Ok(metadata)
    }

    /// Returns true if the column is null by describe.
    pub(crate) fn is_null_by_describe(&self) -> bool {
        self.null_by_describe
    }

    /// Creates metadata for an object attribute or collection element.
    pub(crate) fn new_object_field(
        name: String,
        db_type: &'static DbType,
        max_size: u32,
        precision: u8,
        scale: i8,
    ) -> Self {
        Self::new(
            name,
            db_type,
            max_size as usize,
            precision,
            scale,
            true,
            false,
        )
    }

    /// Returns a new scalar with the given type and maximum size.
    pub(crate) fn new_scalar(
        name: &str,
        db_type: &'static DbType,
        max_size: usize,
    ) -> Self {
        Self::new(name.into(), db_type, max_size, 0, 0, false, false)
    }

    /// Returns a boolean indicating if the metadata requires a define. This is
    /// true for all LOBs (including JSON and VECTOR which are value based
    /// LOBs).
    pub(crate) fn requires_define(&self) -> bool {
        matches!(
            self.db_type.ora_type_num,
            constants::ORA_TYPE_NUM_BLOB
                | constants::ORA_TYPE_NUM_CLOB
                | constants::ORA_TYPE_NUM_BFILE
                | constants::ORA_TYPE_NUM_JSON
                | constants::ORA_TYPE_NUM_VECTOR
        )
    }

    /// Replaces a pending identity with its discovered object type.
    pub(crate) fn resolve_object_type(
        &mut self,
        lookup: impl FnOnce(
            &DbObjectTypeIdentity,
        ) -> Result<Arc<DbObjectType>, Error>,
    ) -> Result<(), Error> {
        if let Some(MetadataObjectType::Pending(identity)) = &self.object_type
        {
            self.object_type =
                Some(MetadataObjectType::Resolved(lookup(identity)?));
        }
        Ok(())
    }

    /// Sets the flag indicating that the structure refers to an array.
    pub(crate) fn set_is_array(&mut self, is_array: bool) {
        self.is_array = is_array;
    }

    /// Updates object field details while preserving its name and other
    /// metadata.
    pub(crate) fn set_object_field_details(
        &mut self,
        db_type: &'static DbType,
        max_size: u32,
        precision: u8,
        scale: i8,
    ) {
        self.db_type = db_type;
        self.max_size =
            if max_size == 0 { db_type.default_size } else { max_size };
        self.precision = precision;
        self.scale = scale;
    }

    /// Attaches the shared object type handle.
    pub(crate) fn set_object_type(&mut self, typ: Arc<DbObjectType>) {
        self.object_type = Some(MetadataObjectType::Resolved(typ));
    }

    /// Records a pending object type, treating SYS.XMLTYPE as a scalar
    /// datatype.
    pub(crate) fn set_object_type_identity(
        &mut self,
        identity: DbObjectTypeIdentity,
    ) {
        if identity.is_xmltype() {
            self.db_type = crate::DB_TYPE_XMLTYPE;
            self.object_type = None;
        } else {
            self.db_type = crate::DB_TYPE_OBJECT;
            self.object_type = Some(MetadataObjectType::Pending(identity));
        }
    }

    /// Returns whether any values retrieved by the database need to go into
    /// the pending values vector for processing after deserialization has
    /// completed.
    pub(crate) fn should_defer_value(&self) -> bool {
        !self.null_by_describe
            && matches!(
                self.db_type.ora_type_num,
                constants::ORA_TYPE_NUM_CLOB
                    | constants::ORA_TYPE_NUM_BLOB
                    | constants::ORA_TYPE_NUM_BFILE
                    | constants::ORA_TYPE_NUM_CURSOR
                    | constants::ORA_TYPE_NUM_OBJECT
            )
    }

    /// Writes the metadata to the buffer.
    pub(crate) fn write_to_buf(&self, buf: &mut WriteBuffer, client: &Client) {
        let mut cont_flag: u64 = 0;
        let mut lob_prefetch_length: u32 = 0;
        match self.db_type.ora_type_num {
            // For LOB locators the protocol expects the LOB prefetch flag
            // to be set even when no prefetching is requested (length = 0).
            constants::ORA_TYPE_NUM_BLOB | constants::ORA_TYPE_NUM_CLOB => {
                cont_flag = 0x2000000; // LOB prefetch flag
            }
            // Value-based LOBs (JSON/VECTOR) use this flag and include a
            // prefetched value payload (length = buffer_size()).
            constants::ORA_TYPE_NUM_JSON | constants::ORA_TYPE_NUM_VECTOR => {
                cont_flag = 0x2000000; // LOB prefetch flag
                lob_prefetch_length = self.buffer_size();
            }
            _ => {}
        };

        buf.write_u8(self.db_type.ora_type_num.try_into().unwrap());
        buf.write_u8(constants::TTC_BIND_FLAG_USE_INDICATORS);
        buf.write_u8(0); // precision must always be zero
        buf.write_u8(0); // scale must always be zero
        buf.write_ub4(self.buffer_size());
        buf.write_ub4(0); // max number of elements
        buf.write_ub8(cont_flag);
        if let Some(MetadataObjectType::Resolved(typ)) = &self.object_type {
            buf.write_bytes_with_double_length(Some(typ.oid()));
            buf.write_ub4(typ.version().into());
        } else {
            buf.write_ub4(0); // OID
            buf.write_ub2(0); // version
        }
        if self.db_type.csfrm != 0 {
            buf.write_ub2(constants::CHARSET_ID_UTF8);
        } else {
            buf.write_ub2(0);
        }
        buf.write_u8(self.db_type.csfrm);
        buf.write_ub4(lob_prefetch_length);
        if client.supports_ttc_field_version(constants::TTC_FIELD_VERSION_12_2)
        {
            buf.write_ub4(0); // oaccolid
        }
    }

    /// Returns the annotations of the column as a vector of name and value
    /// pairs, in the order the database sent them. The slice is empty if the
    /// column has no annotations.
    pub fn annotations(&self) -> &[(String, String)] {
        &self.annotations
    }

    /// Returns the buffer size.
    pub fn buffer_size(&self) -> u32 {
        let buffer_size_factor: u32 =
            self.db_type.buffer_size_factor.try_into().unwrap();
        if self.db_type.default_size == 0 {
            buffer_size_factor
        } else {
            self.max_size * buffer_size_factor
        }
    }

    /// Returns a string representation of the data type as would be found in
    /// a create table statement.
    pub fn data_type(&self) -> String {
        match self.db_type.ora_type_num {
            constants::ORA_TYPE_NUM_CHAR
            | constants::ORA_TYPE_NUM_VARCHAR
            | constants::ORA_TYPE_NUM_RAW => {
                format!("{}({})", self.db_type.ora_name, self.max_size)
            }
            constants::ORA_TYPE_NUM_INTERVAL_DS => {
                format!(
                    "INTERVAL DAY({}) TO SECOND({})",
                    self.precision, self.scale
                )
            }
            constants::ORA_TYPE_NUM_INTERVAL_YM => {
                format!("INTERVAL YEAR({}) TO MONTH", self.precision)
            }
            constants::ORA_TYPE_NUM_NUMBER => {
                match (self.precision, self.scale) {
                    (0, _) => self.db_type.ora_name.to_string(),
                    (p, 0) => format!("NUMBER({})", p),
                    (p, s) => format!("NUMBER({},{})", p, s),
                }
            }
            constants::ORA_TYPE_NUM_TIMESTAMP => {
                format!("TIMESTAMP({})", self.scale)
            }
            constants::ORA_TYPE_NUM_TIMESTAMP_LTZ => {
                format!("TIMESTAMP({}) WITH LOCAL TIME ZONE", self.scale)
            }
            constants::ORA_TYPE_NUM_TIMESTAMP_TZ => {
                format!("TIMESTAMP({}) WITH TIME ZONE", self.scale)
            }
            constants::ORA_TYPE_NUM_VECTOR => {
                match (self.vector_dimensions, &self.vector_storage_format) {
                    (0, None) => self.db_type.ora_name.to_string(),
                    (d, None) => format!("VECTOR({})", d),
                    (0, Some(f)) => format!("VECTOR(*,{})", f.name()),
                    (d, Some(f)) => format!("VECTOR({},{})", d, f.name()),
                }
            }
            _ => self.db_type.ora_name.to_string(),
        }
    }

    /// Returns the database type of the data.
    pub fn db_type(&self) -> &'static DbType {
        self.db_type
    }

    /// Returns the domain of the data, if applicable, as a 2-tuple containing
    /// the schema and the name of the domain.
    pub fn domain(&self) -> Option<(&str, &str)> {
        self.domain.as_ref().map(|(a, b)| (a.as_str(), b.as_str()))
    }

    /// Returns whether the data refers to an array.
    pub fn is_array(&self) -> bool {
        self.is_array
    }

    /// Returns whether the data refers to sparse vectors.
    pub fn is_sparse_vector(&self) -> bool {
        if self.db_type == crate::DB_TYPE_VECTOR {
            self.vector_flags & VECTOR_META_FLAG_SPARSE_VECTOR != 0
        } else {
            false
        }
    }

    /// Returns the maximum size of RAW, (N)CHAR and (N)VARCHAR2 data.
    pub fn max_size(&self) -> u32 {
        self.max_size
    }

    /// Returns the name of the data.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns whether the data allows nulls or not.
    pub fn nullable(&self) -> bool {
        self.nullable
    }

    /// Returns the database object type, or None for a field without a
    /// database object type. An error is raised if discovery is incomplete.
    pub fn object_type(&self) -> Result<Option<Arc<DbObjectType>>, Error> {
        match &self.object_type {
            Some(MetadataObjectType::Resolved(typ)) => {
                Ok(Some(Arc::clone(typ)))
            }
            Some(MetadataObjectType::Pending(_)) => {
                Err(Error::db_object_type_unavailable())
            }
            None => Ok(None),
        }
    }

    /// Returns the precision of the data. This is used for the precision of
    /// NUMBER, the precision for the number of days in INTERVAL DAY TO SECOND
    /// and the precision of the number of years in INTERVAL YEAR TO MONTH. For
    /// all other types this value is always zero.
    pub fn precision(&self) -> u8 {
        self.precision
    }

    /// Returns the scale of the data. This is used for the scale of NUMBER
    /// and the fractional seconds precision of TIMESTAMP, TIMESTAMP WITH LOCAL
    /// TIMEZONE, TIMESTAMP WITH TIMEZONE and INTERVAL DAY TO SECOND data. For
    /// all other types this value is always zero.
    pub fn scale(&self) -> i8 {
        self.scale
    }

    /// Returns the number of dimensions used for vectors. If the data does not
    /// refer to a vector or the vector data is flexible, None is returned.
    pub fn vector_dimensions(&self) -> Option<usize> {
        if self.db_type == crate::DB_TYPE_VECTOR
            && self.vector_flags & VECTOR_META_FLAG_FLEXIBLE_DIM != 0
        {
            Some(self.vector_dimensions as usize)
        } else {
            None
        }
    }

    /// Returns the storage format used for vectors. If the data does not
    /// refer to a vector or the vector data is flexible, None is returned.
    pub fn vector_storage_format(&self) -> Option<VectorStorageFormat> {
        self.vector_storage_format.clone()
    }
}
