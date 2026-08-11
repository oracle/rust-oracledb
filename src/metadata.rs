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

use crate::client::Client;
use crate::constants;
use crate::db_type::DB_TYPE_BLOB;
use crate::db_type::DB_TYPE_CLOB;
use crate::db_type::DB_TYPE_LONG;
use crate::db_type::DB_TYPE_LONG_NVARCHAR;
use crate::db_type::DB_TYPE_LONG_RAW;
use crate::db_type::DB_TYPE_NCLOB;
use crate::db_type::DbType;
use crate::error::Error;
use crate::response::Response;
use crate::write_buffer::WriteBuffer;

/// Represents the metadata of columns fetched from the database.
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
}

impl Metadata {
    /// Returns a new structure with the given values.
    fn new(
        db_type: &'static DbType,
        max_size: usize,
        is_array: bool,
    ) -> Metadata {
        let actual_max_size: u32 = if max_size == 0 {
            db_type.default_size
        } else {
            max_size.try_into().unwrap()
        };
        Metadata {
            name: String::new(),
            db_type,
            nullable: false,
            null_by_describe: false,
            precision: 0,
            scale: 0,
            max_size: actual_max_size,
            is_array,
        }
    }

    /// Returns a new structure suitable for defining CLOB/BLOB as
    /// string/bytes. Currently LOB locators are not supported at all.
    pub(crate) fn define_metadata(&self) -> Metadata {
        match *self.db_type {
            DB_TYPE_BLOB => Metadata::new(&DB_TYPE_LONG_RAW, 0, false),
            DB_TYPE_CLOB => Metadata::new(&DB_TYPE_LONG, 0, false),
            DB_TYPE_NCLOB => Metadata::new(&DB_TYPE_LONG_NVARCHAR, 0, false),
            _ => self.clone(),
        }
    }

    /// Returns a new structure from the metadata returned by the database.
    pub(crate) fn from_response(
        resp: &mut Response,
        client: &Client,
    ) -> Result<Metadata, Error> {
        let ora_type_num: u16 = resp.read_u8()?.into();
        resp.read_u8()?; // flags
        let precision = resp.read_u8()?;
        let scale = resp.read_i8()?;
        let buffer_size = resp.read_ub4()?;
        resp.read_ub4()?; // max number of array elements
        resp.read_ub8()?; // cont flags
        resp.read_bytes_with_double_length()?; // oid
        resp.read_ub2()?; // version
        resp.read_ub2()?; // character set id
        let csfrm = resp.read_u8()?;
        let db_type = DbType::from_ora_type_and_csfrm(ora_type_num, csfrm);
        let mut max_size = resp.read_ub4()?;
        if ora_type_num == constants::ORA_TYPE_NUM_RAW {
            max_size = buffer_size;
        }
        if client.supports_ttc_field_version(constants::TTC_FIELD_VERSION_12_2)
        {
            resp.read_ub4()?; // oaccolid
        }
        let nulls_allowed = resp.read_u8()?;
        resp.read_u8()?; // v7 length of name
        let name = resp.read_utf8_with_double_length()?.to_string();
        let _obj_schema = resp.read_utf8_with_double_length()?;
        let _obj_name = resp.read_utf8_with_double_length()?;
        resp.read_ub2()?; // column position
        let _uds_flags = resp.read_ub4()?;
        if client.supports_ttc_field_version(constants::TTC_FIELD_VERSION_23_1)
        {
            let _domain_schema = resp.read_utf8_with_double_length()?;
            let _domain_name = resp.read_utf8_with_double_length()?;
        }
        if client.supports_ttc_field_version(
            constants::TTC_FIELD_VERSION_23_1_EXT_3,
        ) {
            let num_annotations = resp.read_ub4()?;
            if num_annotations > 0 {
                todo!();
            }
        }
        if client.supports_ttc_field_version(constants::TTC_FIELD_VERSION_23_4)
        {
            let _vector_dimensions = resp.read_ub4()?;
            let _vector_format = resp.read_u8()?;
            let _vector_flags = resp.read_u8()?;
        }

        let null_by_describe = match db_type.ora_type_num {
            constants::ORA_TYPE_NUM_LONG
            | constants::ORA_TYPE_NUM_LONG_RAW
            | constants::ORA_TYPE_NUM_UROWID => false,
            _ => buffer_size == 0,
        };

        Ok(Metadata {
            name,
            db_type,
            nullable: (nulls_allowed != 0),
            precision,
            scale,
            null_by_describe,
            max_size,
            is_array: false,
        })
    }

    /// Returns true if the column is null by describe.
    pub(crate) fn is_null_by_describe(&self) -> bool {
        self.null_by_describe
    }

    /// Returns a new scalar with the given type and maximum size.
    pub(crate) fn new_scalar(
        db_type: &'static DbType,
        max_size: usize,
    ) -> Metadata {
        Metadata::new(db_type, max_size, false)
    }

    /// Returns a boolean indicating if the metadata requires a define. This is
    /// true for all LOBs (including JSON and VECTOR which are value based
    /// LOBs).
    pub(crate) fn requires_define(&self) -> bool {
        matches!(
            self.db_type.ora_type_num,
            constants::ORA_TYPE_NUM_BLOB
                | constants::ORA_TYPE_NUM_CLOB
                | constants::ORA_TYPE_NUM_JSON
                | constants::ORA_TYPE_NUM_VECTOR
        )
    }

    /// Sets the flag indicating that the structure refers to an array.
    pub(crate) fn set_is_array(&mut self, is_array: bool) {
        self.is_array = is_array;
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
        buf.write_ub4(0); // OID
        buf.write_ub2(0); // version
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

    /// Returns the database type of the column.
    pub fn db_type(&self) -> &'static DbType {
        self.db_type
    }

    /// Returns whether the metadata refers to an array.
    pub fn is_array(&self) -> bool {
        self.is_array
    }

    /// Returns the maximum size of RAW, (N)CHAR and (N)VARCHAR2 columns.
    pub fn max_size(&self) -> u32 {
        self.max_size
    }

    /// Returns the name of the column.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns whether the column allows nulls or not.
    pub fn nullable(&self) -> bool {
        self.nullable
    }

    /// Returns the precision of the column.
    pub fn precision(&self) -> u8 {
        self.precision
    }

    /// Returns the scale of the column.
    pub fn scale(&self) -> i8 {
        self.scale
    }
}
