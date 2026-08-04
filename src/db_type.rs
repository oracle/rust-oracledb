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
// db_type.rs
//
// Defines the database types supported by the driver.
//-----------------------------------------------------------------------------

use crate::constants;
use std::fmt;

/// Repreents a database type supported by the library.
#[derive(PartialEq)]
pub struct DbType {
    num: u16,
    name: &'static str,
    pub(crate) default_size: u32,
    pub(crate) buffer_size_factor: usize,
    pub(crate) ora_name: &'static str,
    pub(crate) ora_type_num: u16,
    pub(crate) csfrm: u8,
    pub(crate) is_fast: bool,
}

impl DbType {
    /// Returns the instance of the database type corresponding to the Oracle
    /// type number and character set form.
    pub(crate) fn from_ora_type_and_csfrm(
        ora_type_num: u16,
        csfrm: u8,
    ) -> &'static DbType {
        match ora_type_num {
            constants::ORA_TYPE_NUM_BFILE => &DB_TYPE_BFILE,
            constants::ORA_TYPE_NUM_BINARY_DOUBLE => &DB_TYPE_BINARY_DOUBLE,
            constants::ORA_TYPE_NUM_BINARY_FLOAT => &DB_TYPE_BINARY_FLOAT,
            constants::ORA_TYPE_NUM_BINARY_INTEGER => &DB_TYPE_BINARY_INTEGER,
            constants::ORA_TYPE_NUM_BLOB => &DB_TYPE_BLOB,
            constants::ORA_TYPE_NUM_BOOLEAN => &DB_TYPE_BOOLEAN,
            constants::ORA_TYPE_NUM_CHAR => match csfrm {
                constants::CS_FORM_IMPLICIT => &DB_TYPE_CHAR,
                constants::CS_FORM_NCHAR => &DB_TYPE_NCHAR,
                _ => &DB_TYPE_UNKNOWN,
            },
            constants::ORA_TYPE_NUM_CLOB => match csfrm {
                constants::CS_FORM_IMPLICIT => &DB_TYPE_CLOB,
                constants::CS_FORM_NCHAR => &DB_TYPE_NCLOB,
                _ => &DB_TYPE_UNKNOWN,
            },
            constants::ORA_TYPE_NUM_CURSOR => &DB_TYPE_CURSOR,
            constants::ORA_TYPE_NUM_DATE => &DB_TYPE_DATE,
            constants::ORA_TYPE_NUM_INTERVAL_DS => &DB_TYPE_INTERVAL_DS,
            constants::ORA_TYPE_NUM_INTERVAL_YM => &DB_TYPE_INTERVAL_YM,
            constants::ORA_TYPE_NUM_JSON => &DB_TYPE_JSON,
            constants::ORA_TYPE_NUM_LONG => match csfrm {
                constants::CS_FORM_IMPLICIT => &DB_TYPE_LONG,
                constants::CS_FORM_NCHAR => &DB_TYPE_LONG_NVARCHAR,
                _ => &DB_TYPE_UNKNOWN,
            },
            constants::ORA_TYPE_NUM_LONG_RAW => &DB_TYPE_LONG_RAW,
            constants::ORA_TYPE_NUM_NUMBER => &DB_TYPE_NUMBER,
            constants::ORA_TYPE_NUM_OBJECT => &DB_TYPE_OBJECT,
            constants::ORA_TYPE_NUM_RAW => &DB_TYPE_RAW,
            constants::ORA_TYPE_NUM_ROWID => &DB_TYPE_ROWID,
            constants::ORA_TYPE_NUM_TIMESTAMP => &DB_TYPE_TIMESTAMP,
            constants::ORA_TYPE_NUM_TIMESTAMP_LTZ => &DB_TYPE_TIMESTAMP_LTZ,
            constants::ORA_TYPE_NUM_TIMESTAMP_TZ => &DB_TYPE_TIMESTAMP_TZ,
            constants::ORA_TYPE_NUM_UROWID => &DB_TYPE_UROWID,
            constants::ORA_TYPE_NUM_VARCHAR => match csfrm {
                constants::CS_FORM_IMPLICIT => &DB_TYPE_VARCHAR,
                constants::CS_FORM_NCHAR => &DB_TYPE_NVARCHAR,
                _ => &DB_TYPE_UNKNOWN,
            },
            constants::ORA_TYPE_NUM_VECTOR => &DB_TYPE_VECTOR,
            _ => &DB_TYPE_UNKNOWN,
        }
    }

    /// Returns whether the database type contains binary (raw) data. This
    /// method returns true for types [`DB_TYPE_RAW`], [`DB_TYPE_LONG_RAW`] and
    /// [`DB_TYPE_BLOB`] and false for all other types.
    pub fn is_binary_type(&self) -> bool {
        matches!(
            self.ora_type_num,
            constants::ORA_TYPE_NUM_RAW
                | constants::ORA_TYPE_NUM_LONG_RAW
                | constants::ORA_TYPE_NUM_BLOB
        )
    }

    /// Returns whether the database type contains date data. This method
    /// returns true for types [`DB_TYPE_DATE`], [`DB_TYPE_TIMESTAMP`],
    /// [`DB_TYPE_TIMESTAMP_LTZ`] and [`DB_TYPE_TIMESTAMP_TZ`] and false for
    /// all other types.
    pub fn is_date_type(&self) -> bool {
        matches!(
            self.ora_type_num,
            constants::ORA_TYPE_NUM_DATE
                | constants::ORA_TYPE_NUM_TIMESTAMP
                | constants::ORA_TYPE_NUM_TIMESTAMP_LTZ
                | constants::ORA_TYPE_NUM_TIMESTAMP_TZ
        )
    }

    /// Returns whether the database type contains string data. This method
    /// returns true for types [`DB_TYPE_CHAR`], [`DB_TYPE_NCHAR`],
    /// [`DB_TYPE_VARCHAR`], [`DB_TYPE_NVARCHAR`], [`DB_TYPE_LONG`],
    /// [`DB_TYPE_LONG_NVARCHAR`], [`DB_TYPE_ROWID`], [`DB_TYPE_UROWID`],
    /// [`DB_TYPE_CLOB`] and [`DB_TYPE_NCLOB`] and false for all other types.
    pub fn is_string_type(&self) -> bool {
        matches!(
            self.ora_type_num,
            constants::ORA_TYPE_NUM_CHAR
                | constants::ORA_TYPE_NUM_VARCHAR
                | constants::ORA_TYPE_NUM_LONG
                | constants::ORA_TYPE_NUM_ROWID
                | constants::ORA_TYPE_NUM_UROWID
                | constants::ORA_TYPE_NUM_CLOB
        )
    }

    /// Returns the name of the database type.
    pub fn name(&self) -> &str {
        self.name
    }

    /// Returns the number of the database type.
    pub fn num(&self) -> u16 {
        self.num
    }
}

impl fmt::Display for DbType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<DbType {}>", self.name)
    }
}

impl fmt::Debug for DbType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub const DB_TYPE_BFILE: DbType = DbType {
    num: constants::DB_TYPE_NUM_BFILE,
    name: "DB_TYPE_BFILE",
    default_size: 0,
    buffer_size_factor: 4000,
    ora_name: "BFILE",
    ora_type_num: constants::ORA_TYPE_NUM_BFILE,
    csfrm: constants::CS_FORM_NONE,
    is_fast: false,
};

pub const DB_TYPE_BINARY_DOUBLE: DbType = DbType {
    num: constants::DB_TYPE_NUM_BINARY_DOUBLE,
    name: "DB_TYPE_BINARY_DOUBLE",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_BINARY_DOUBLE,
    ora_name: "BINARY_DOUBLE",
    ora_type_num: constants::ORA_TYPE_NUM_BINARY_DOUBLE,
    csfrm: constants::CS_FORM_NONE,
    is_fast: true,
};

pub const DB_TYPE_BINARY_FLOAT: DbType = DbType {
    num: constants::DB_TYPE_NUM_BINARY_FLOAT,
    name: "DB_TYPE_BINARY_FLOAT",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_BINARY_FLOAT,
    ora_name: "BINARY_FLOAT",
    ora_type_num: constants::ORA_TYPE_NUM_BINARY_FLOAT,
    csfrm: constants::CS_FORM_NONE,
    is_fast: true,
};

pub const DB_TYPE_BINARY_INTEGER: DbType = DbType {
    num: constants::DB_TYPE_NUM_BINARY_INTEGER,
    name: "DB_TYPE_BINARY_INTEGER",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_NUMBER,
    ora_name: "BINARY_INTEGER",
    ora_type_num: constants::ORA_TYPE_NUM_BINARY_INTEGER,
    csfrm: constants::CS_FORM_NONE,
    is_fast: true,
};

pub const DB_TYPE_BLOB: DbType = DbType {
    num: constants::DB_TYPE_NUM_BLOB,
    name: "DB_TYPE_BLOB",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_LOB,
    ora_name: "BLOB",
    ora_type_num: constants::ORA_TYPE_NUM_BLOB,
    csfrm: constants::CS_FORM_NONE,
    is_fast: false,
};

pub const DB_TYPE_BOOLEAN: DbType = DbType {
    num: constants::DB_TYPE_NUM_BOOLEAN,
    name: "DB_TYPE_BOOLEAN",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_BOOLEAN,
    ora_name: "BOOLEAN",
    ora_type_num: constants::ORA_TYPE_NUM_BOOLEAN,
    csfrm: constants::CS_FORM_NONE,
    is_fast: true,
};

pub const DB_TYPE_CHAR: DbType = DbType {
    num: constants::DB_TYPE_NUM_CHAR,
    name: "DB_TYPE_CHAR",
    default_size: 2000,
    buffer_size_factor: 4,
    ora_name: "CHAR",
    ora_type_num: constants::ORA_TYPE_NUM_CHAR,
    csfrm: constants::CS_FORM_IMPLICIT,
    is_fast: true,
};

pub const DB_TYPE_CLOB: DbType = DbType {
    num: constants::DB_TYPE_NUM_CLOB,
    name: "DB_TYPE_CLOB",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_LOB,
    ora_name: "CLOB",
    ora_type_num: constants::ORA_TYPE_NUM_CLOB,
    csfrm: constants::CS_FORM_IMPLICIT,
    is_fast: false,
};

pub const DB_TYPE_CURSOR: DbType = DbType {
    num: constants::DB_TYPE_NUM_CURSOR,
    name: "DB_TYPE_CURSOR",
    default_size: 0,
    buffer_size_factor: 4,
    ora_name: "CURSOR",
    ora_type_num: constants::ORA_TYPE_NUM_CURSOR,
    csfrm: constants::CS_FORM_NONE,
    is_fast: false,
};

pub const DB_TYPE_DATE: DbType = DbType {
    num: constants::DB_TYPE_NUM_DATE,
    name: "DB_TYPE_DATE",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_DATE,
    ora_name: "DATE",
    ora_type_num: constants::ORA_TYPE_NUM_DATE,
    csfrm: constants::CS_FORM_NONE,
    is_fast: true,
};

pub const DB_TYPE_INTERVAL_DS: DbType = DbType {
    num: constants::DB_TYPE_NUM_INTERVAL_DS,
    name: "DB_TYPE_INTERVAL_DS",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_INTERVAL_DS,
    ora_name: "INTERVAL DAY TO SECOND",
    ora_type_num: constants::ORA_TYPE_NUM_INTERVAL_DS,
    csfrm: constants::CS_FORM_NONE,
    is_fast: true,
};

pub const DB_TYPE_INTERVAL_YM: DbType = DbType {
    num: constants::DB_TYPE_NUM_INTERVAL_YM,
    name: "DB_TYPE_INTERVAL_YM",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_INTERVAL_YM,
    ora_name: "INTERVAL YEAR TO MONTH",
    ora_type_num: constants::ORA_TYPE_NUM_INTERVAL_YM,
    csfrm: constants::CS_FORM_NONE,
    is_fast: true,
};

pub const DB_TYPE_JSON: DbType = DbType {
    num: constants::DB_TYPE_NUM_JSON,
    name: "DB_TYPE_JSON",
    default_size: 0,
    buffer_size_factor: 32 * 1024 * 1024,
    ora_name: "JSON",
    ora_type_num: constants::ORA_TYPE_NUM_JSON,
    csfrm: constants::CS_FORM_NONE,
    is_fast: false,
};

pub const DB_TYPE_LONG: DbType = DbType {
    num: constants::DB_TYPE_NUM_LONG_VARCHAR,
    name: "DB_TYPE_LONG",
    default_size: 0,
    buffer_size_factor: 2147483647,
    ora_name: "LONG",
    ora_type_num: constants::ORA_TYPE_NUM_LONG,
    csfrm: constants::CS_FORM_IMPLICIT,
    is_fast: false,
};

pub const DB_TYPE_LONG_NVARCHAR: DbType = DbType {
    num: constants::DB_TYPE_NUM_LONG_NVARCHAR,
    name: "DB_TYPE_LONG_NVARCHAR",
    default_size: 0,
    buffer_size_factor: 2147483647,
    ora_name: "LONG NVARCHAR",
    ora_type_num: constants::ORA_TYPE_NUM_LONG,
    csfrm: constants::CS_FORM_NCHAR,
    is_fast: false,
};

pub const DB_TYPE_LONG_RAW: DbType = DbType {
    num: constants::DB_TYPE_NUM_LONG_RAW,
    name: "DB_TYPE_LONG_RAW",
    default_size: 0,
    buffer_size_factor: 2147483647,
    ora_name: "LONG RAW",
    ora_type_num: constants::ORA_TYPE_NUM_LONG_RAW,
    csfrm: constants::CS_FORM_NONE,
    is_fast: false,
};

pub const DB_TYPE_NCHAR: DbType = DbType {
    num: constants::DB_TYPE_NUM_NCHAR,
    name: "DB_TYPE_NCHAR",
    default_size: 2000,
    buffer_size_factor: 4,
    ora_name: "NCHAR",
    ora_type_num: constants::ORA_TYPE_NUM_CHAR,
    csfrm: constants::CS_FORM_NCHAR,
    is_fast: true,
};

pub const DB_TYPE_NCLOB: DbType = DbType {
    num: constants::DB_TYPE_NUM_NCLOB,
    name: "DB_TYPE_NCLOB",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_LOB,
    ora_name: "NCLOB",
    ora_type_num: constants::ORA_TYPE_NUM_CLOB,
    csfrm: constants::CS_FORM_NCHAR,
    is_fast: false,
};

pub const DB_TYPE_NUMBER: DbType = DbType {
    num: constants::DB_TYPE_NUM_NUMBER,
    name: "DB_TYPE_NUMBER",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_NUMBER,
    ora_name: "NUMBER",
    ora_type_num: constants::ORA_TYPE_NUM_NUMBER,
    csfrm: constants::CS_FORM_NONE,
    is_fast: true,
};

pub const DB_TYPE_NVARCHAR: DbType = DbType {
    num: constants::DB_TYPE_NUM_NVARCHAR,
    name: "DB_TYPE_NVARCHAR",
    default_size: 4000,
    buffer_size_factor: 4,
    ora_name: "NVARCHAR2",
    ora_type_num: constants::ORA_TYPE_NUM_VARCHAR,
    csfrm: constants::CS_FORM_NCHAR,
    is_fast: true,
};

pub const DB_TYPE_OBJECT: DbType = DbType {
    num: constants::DB_TYPE_NUM_OBJECT,
    name: "DB_TYPE_OBJECT",
    default_size: 0,
    buffer_size_factor: 0,
    ora_name: "OBJECT",
    ora_type_num: constants::ORA_TYPE_NUM_OBJECT,
    csfrm: constants::CS_FORM_NONE,
    is_fast: false,
};

pub const DB_TYPE_RAW: DbType = DbType {
    num: constants::DB_TYPE_NUM_RAW,
    name: "DB_TYPE_RAW",
    default_size: 4000,
    buffer_size_factor: 1,
    ora_name: "RAW",
    ora_type_num: constants::ORA_TYPE_NUM_RAW,
    csfrm: constants::CS_FORM_NONE,
    is_fast: true,
};

pub const DB_TYPE_ROWID: DbType = DbType {
    num: constants::DB_TYPE_NUM_ROWID,
    name: "DB_TYPE_ROWID",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_ROWID,
    ora_name: "ROWID",
    ora_type_num: constants::ORA_TYPE_NUM_ROWID,
    csfrm: constants::CS_FORM_NONE,
    is_fast: false,
};

pub const DB_TYPE_TIMESTAMP: DbType = DbType {
    num: constants::DB_TYPE_NUM_TIMESTAMP,
    name: "DB_TYPE_TIMESTAMP",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_TIMESTAMP,
    ora_name: "TIMESTAMP",
    ora_type_num: constants::ORA_TYPE_NUM_TIMESTAMP,
    csfrm: constants::CS_FORM_NONE,
    is_fast: true,
};

pub const DB_TYPE_TIMESTAMP_LTZ: DbType = DbType {
    num: constants::DB_TYPE_NUM_TIMESTAMP_LTZ,
    name: "DB_TYPE_TIMESTAMP_LTZ",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_TIMESTAMP,
    ora_name: "TIMESTAMP WITH LOCAL TIME ZONE",
    ora_type_num: constants::ORA_TYPE_NUM_TIMESTAMP_LTZ,
    csfrm: constants::CS_FORM_NONE,
    is_fast: true,
};

pub const DB_TYPE_TIMESTAMP_TZ: DbType = DbType {
    num: constants::DB_TYPE_NUM_TIMESTAMP_TZ,
    name: "DB_TYPE_TIMESTAMP_TZ",
    default_size: 0,
    buffer_size_factor: constants::ORA_TYPE_SIZE_TIMESTAMP_TZ,
    ora_name: "TIMESTAMP WITH TIME ZONE",
    ora_type_num: constants::ORA_TYPE_NUM_TIMESTAMP_TZ,
    csfrm: constants::CS_FORM_NONE,
    is_fast: true,
};

pub const DB_TYPE_UNKNOWN: DbType = DbType {
    num: constants::DB_TYPE_NUM_UNKNOWN,
    name: "DB_TYPE_UNKNOWN",
    default_size: 0,
    buffer_size_factor: 0,
    ora_name: "UNKNOWN",
    ora_type_num: 0,
    csfrm: constants::CS_FORM_NONE,
    is_fast: false,
};

pub const DB_TYPE_UROWID: DbType = DbType {
    num: constants::DB_TYPE_NUM_UROWID,
    name: "DB_TYPE_UROWID",
    default_size: 0,
    buffer_size_factor: 0,
    ora_name: "UROWID",
    ora_type_num: constants::ORA_TYPE_NUM_UROWID,
    csfrm: constants::CS_FORM_NONE,
    is_fast: false,
};

pub const DB_TYPE_VARCHAR: DbType = DbType {
    num: constants::DB_TYPE_NUM_VARCHAR,
    name: "DB_TYPE_VARCHAR",
    default_size: 4000,
    buffer_size_factor: 4,
    ora_name: "VARCHAR2",
    ora_type_num: constants::ORA_TYPE_NUM_VARCHAR,
    csfrm: constants::CS_FORM_IMPLICIT,
    is_fast: true,
};

pub const DB_TYPE_VECTOR: DbType = DbType {
    num: constants::DB_TYPE_NUM_VECTOR,
    name: "DB_TYPE_VECTOR",
    default_size: 0,
    buffer_size_factor: 1024 * 1024,
    ora_name: "VECTOR",
    ora_type_num: constants::ORA_TYPE_NUM_VECTOR,
    csfrm: constants::CS_FORM_NONE,
    is_fast: false,
};

pub const DB_TYPE_XMLTYPE: DbType = DbType {
    num: constants::DB_TYPE_NUM_XMLTYPE,
    name: "DB_TYPE_XMLTYPE",
    default_size: 0,
    buffer_size_factor: 2147483647,
    ora_name: "XMLTYPE",
    ora_type_num: constants::ORA_TYPE_NUM_OBJECT,
    csfrm: constants::CS_FORM_IMPLICIT,
    is_fast: false,
};
