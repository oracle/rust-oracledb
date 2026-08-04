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
// lib.rs
//
// Main library of oracledb crate.
//-----------------------------------------------------------------------------

#![crate_type = "lib"]
#![crate_name = "oracledb"]
#![forbid(unsafe_code)]

#[cfg(feature = "arrow")]
mod arrow;
mod bind_params;
mod client;
mod config;
mod connection;
mod constants;
mod cursor;
mod db_info;
mod db_type;
mod db_value;
mod end_user_security_context;
mod error;
mod exec_result;
mod json;
mod lob;
mod messages;
mod metadata;
mod ora_type;
mod ora_version;
mod packet;
mod parser;
mod pool;
mod read_buffer;
mod response;
mod row;
mod rowid;
mod secret_value;
mod statement;
mod test_config;
mod transport;
mod utils;
mod vector;
mod write_buffer;

// public structs
pub use crate::config::Config;
pub use crate::config::PoolConfig;
pub use crate::connection::Connection;
pub use crate::cursor::Cursor;
pub use crate::db_type::DbType;
pub use crate::end_user_security_context::EndUserSecurityContext;
pub use crate::end_user_security_context::EndUserSecurityContextBuilder;
pub use crate::error::Error;
pub use crate::exec_result::ExecResult;
pub use crate::lob::Lob;
pub use crate::metadata::Metadata;
pub use crate::ora_version::OracleVersion;
pub use crate::pool::Pool;
pub use crate::statement::Statement;
pub use crate::vector::SparseVector;

// public enums
pub use crate::bind_params::BindParameters;
pub use crate::end_user_security_context::EndUserIdentity;
pub use crate::error::ErrorKind;
pub use crate::json::JsonValue;
pub use crate::vector::Vector;
pub use crate::vector::VectorData;

// public methods
pub use crate::utils::enquote_literal;
pub use crate::utils::enquote_name;
pub use crate::utils::is_qualified_sql_name;
pub use crate::utils::is_simple_sql_name;

// public traits
pub use crate::db_value::FromDbValue;
pub use crate::db_value::ToDbValue;

// Oracle specific types
pub use crate::ora_type::OracleIntervalDS;
pub use crate::ora_type::OracleIntervalYM;
pub use crate::ora_type::OracleNumber;
pub use crate::ora_type::OracleTimestamp;

// database types
/// Describes columns, attributes or array elements in a database that are of
/// type BFILE.
pub use crate::db_type::DB_TYPE_BFILE;

/// Describes columns, attributes or array elements in a database that are of
/// type BINARY_DOUBLE.
pub use crate::db_type::DB_TYPE_BINARY_DOUBLE;

/// Describes columns, attributes or array elements in a database that are of
/// type BINARY_FLOAT.
pub use crate::db_type::DB_TYPE_BINARY_FLOAT;

/// Describes attributes or array elements in a database that are of type
/// BINARY_INTEGER.
pub use crate::db_type::DB_TYPE_BINARY_INTEGER;

/// Describes columns, attributes or array elements in a database that are of
/// type BLOB.
pub use crate::db_type::DB_TYPE_BLOB;

/// Describes columns, attributes or array elements in a database that are of
/// type BOOLEAN. Prior to Oracle Database 26ai, columns could not be of type
/// BOOLEAN and the type could only be used in PL/SQL.
pub use crate::db_type::DB_TYPE_BOOLEAN;

/// Describes columns, attributes or array elements in a database that are of
/// type CHAR.
///
/// Note that these are fixed length string values and behave
/// differently from VARCHAR2.
pub use crate::db_type::DB_TYPE_CHAR;

/// Describes columns, attributes or array elements in a database that are of
/// type CLOB.
pub use crate::db_type::DB_TYPE_CLOB;

/// Describes columns in a database that are of type CURSOR. In PL/SQL, these
/// are known as REF CURSOR.
pub use crate::db_type::DB_TYPE_CURSOR;

/// Describes columns, attributes or array elements in a database that are of
/// type DATE.
pub use crate::db_type::DB_TYPE_DATE;

/// Describes columns, attributes or array elements in a database that are of
/// type INTERVAL DAY TO SECOND.
pub use crate::db_type::DB_TYPE_INTERVAL_DS;

/// Describes columns, attributes or array elements in a database that are of
/// type INTERVAL YEAR TO MONTH.
pub use crate::db_type::DB_TYPE_INTERVAL_YM;

/// Describes columns in a database that are of type JSON (with Oracle Database
/// 21 or later).
pub use crate::db_type::DB_TYPE_JSON;

/// Describes columns, attributes or array elements in a database that are of
/// type LONG.
pub use crate::db_type::DB_TYPE_LONG;

/// This type is used internally to describe columns that are of type
/// [`DB_TYPE_NCLOB`] but are being returned as string data instead of a LOB
/// locator.
pub use crate::db_type::DB_TYPE_LONG_NVARCHAR;

/// Describes columns, attributes or array elements in a database that are of
/// type LONG RAW.
pub use crate::db_type::DB_TYPE_LONG_RAW;

/// Describes columns, attributes or array elements in a database that are of
/// type NCHAR.
///
/// Note that these are fixed length string values and behave
/// differently from NVARCHAR2.
pub use crate::db_type::DB_TYPE_NCHAR;

/// Describes columns, attributes or array elements in a database that are of
/// type NCLOB.
pub use crate::db_type::DB_TYPE_NCLOB;

/// Describes columns, attributes or array elements in a database that are of
/// type NUMBER.
pub use crate::db_type::DB_TYPE_NUMBER;

/// Describes columns, attributes or array elements in a database that are of
/// type NVARCHAR2.
pub use crate::db_type::DB_TYPE_NVARCHAR;

/// Describes columns, attributes or array elements in a database that are an
/// instance of a named SQL or PL/SQL type.
pub use crate::db_type::DB_TYPE_OBJECT;

/// Describes columns, attributes or array elements in a database that are of
/// type RAW.
pub use crate::db_type::DB_TYPE_RAW;

/// Describes columns, attributes or array elements in a database that are of
/// type ROWID.
pub use crate::db_type::DB_TYPE_ROWID;

/// Describes columns, attributes or array elements in a database that are of
/// type TIMESTAMP.
pub use crate::db_type::DB_TYPE_TIMESTAMP;

/// Describes columns, attributes or array elements in a database that are of
/// type TIMESTAMP WITH LOCAL TIME ZONE.
pub use crate::db_type::DB_TYPE_TIMESTAMP_LTZ;

/// Describes columns, attributes or array elements in a database that are of
/// type TIMESTAMP WITH TIME ZONE.
pub use crate::db_type::DB_TYPE_TIMESTAMP_TZ;

/// Describes columns, attributes or array elements in a database that are of
/// an unknown type.
pub use crate::db_type::DB_TYPE_UNKNOWN;

/// Describes columns, attributes or array elements in a database that are of
/// type UROWID.
pub use crate::db_type::DB_TYPE_UROWID;

/// Describes columns, attributes or array elements in a database that are of
/// type VARCHAR2.
pub use crate::db_type::DB_TYPE_VARCHAR;

/// Describes columns, attributes or array elements in a database that are of
/// type VECTOR (with Oracle Database 26ai or later).
pub use crate::db_type::DB_TYPE_VECTOR;

/// Describes columns, attributes or array elements in a database that are of
/// type SYS.XMLTYPE.
pub use crate::db_type::DB_TYPE_XMLTYPE;

// authorization modes
/// This constant is used to specify that default authentication is to take
/// place and is the value used if [`Config::set_auth_mode`] is not called.
pub use crate::constants::AUTH_MODE_DEFAULT;

/// This constant is used to specify that SYSASM access is to be acquired.
pub use crate::constants::AUTH_MODE_SYSASM;

/// This constant is used to specify that SYSBACKUP access is to be acquired.
pub use crate::constants::AUTH_MODE_SYSBKP;

/// This constant is used to specify that SYSDBA access is to be acquired.
pub use crate::constants::AUTH_MODE_SYSDBA;

/// This constant is used to specify that SYSDG access is to be acquired.
pub use crate::constants::AUTH_MODE_SYSDGD;

/// This constant is used to specify that SYSKM access is to be acquired.
pub use crate::constants::AUTH_MODE_SYSKMT;

/// This constant is used to specify that SYSOPER access is to be acquired.
pub use crate::constants::AUTH_MODE_SYSOPER;

/// This constant is used to specify that SYSRAC access is to be acquired.
pub use crate::constants::AUTH_MODE_SYSRAC;

// pool purity
/// This constant is used to specify that the session acquired from the pool
/// should be new and not have any prior session state.
pub use crate::constants::PURITY_NEW;

/// This constant is used to specify that the session acquired from the pool
/// need not be new and may have prior session state.
pub use crate::constants::PURITY_SELF;

// methods for getting the test configuration (only intended for running the
// integration tests and the helper binaries for creating/destroying the test
// schemas)
pub use test_config::get_test_config;

/// Establish a standalone connection to the database with the given
/// configuration.
pub fn connect(config: Config) -> Result<Connection, Error> {
    Connection::connect(config)
}

/// Creates a new pool with the given configuration.
pub fn create_pool(config: PoolConfig) -> Result<Pool, Error> {
    Pool::create(config)
}

// User documentation
pub mod guide {
    #![doc = include_str!("../doc/introduction.md")]
    #![doc = include_str!("../doc/connection_handling.md")]
    #![doc = include_str!("../doc/sql_execution.md")]
    #![doc = include_str!("../doc/plsql_execution.md")]
    #![doc = include_str!("../doc/bind.md")]
    #![doc = include_str!("../doc/batch_statement.md")]
    #![doc = include_str!("../doc/txn_management.md")]
    #![doc = include_str!("../doc/tuning.md")]
    #![doc = include_str!("../doc/lob.md")]
    #![doc = include_str!("../doc/json_data_type.md")]
    #![doc = include_str!("../doc/interval_data_type.md")]
    #![doc = include_str!("../doc/vector_data_type.md")]
    #![doc = include_str!("../doc/arrow.md")]
    #![doc = include_str!("../doc/ha.md")]
    #![doc = include_str!("../doc/globalization.md")]
    #![doc = include_str!("../doc/tracing.md")]
    #![doc = include_str!("../doc/appendix_a.md")]
    #![doc = include_str!("../doc/release_notes.md")]
}
