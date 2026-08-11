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
// db_value.rs
//
// Defines the structures and enumeration containing database values.
//-----------------------------------------------------------------------------

use crate::client::Client;
use crate::constants;
use crate::cursor::Cursor;
use crate::db_type::DbType;
use crate::db_type::{
    DB_TYPE_BINARY_DOUBLE, DB_TYPE_BINARY_FLOAT, DB_TYPE_BOOLEAN,
    DB_TYPE_INTERVAL_DS, DB_TYPE_INTERVAL_YM, DB_TYPE_JSON, DB_TYPE_NUMBER,
    DB_TYPE_RAW, DB_TYPE_TIMESTAMP, DB_TYPE_VARCHAR, DB_TYPE_VECTOR,
};
use crate::error::Error;
use crate::json::JsonValue;
use crate::lob::Lob;
use crate::metadata::Metadata;
use crate::ora_type::OracleIntervalDS;
use crate::ora_type::OracleIntervalYM;
use crate::ora_type::OracleNumber;
use crate::ora_type::OracleTimestamp;
use crate::response::Response;
use crate::row::RowData;
use crate::rowid::Rowid;
use crate::statement::CachedStatement;
use crate::utils;
use crate::vector::Vector;
use crate::write_buffer::ToBuf;
use crate::write_buffer::WriteBuffer;

#[derive(Clone)]
pub enum DbValue {
    Array(RowData),
    BinaryDouble(f64),
    BinaryFloat(f32),
    Boolean(bool),
    Bytes(Vec<u8>),
    Cursor(Option<Cursor>),
    IntervalDS(OracleIntervalDS),
    IntervalYM(OracleIntervalYM),
    Json(JsonValue),
    Number(OracleNumber),
    Rowid(Rowid),
    String(String),
    Timestamp(OracleTimestamp),
    Vector(Vector),
    Lob(Lob),
}

impl DbValue {
    fn array_from_response(
        resp: &mut Response,
        client: &Client,
        statement: &CachedStatement,
        db_type: &'static DbType,
    ) -> Result<Option<DbValue>, Error> {
        let num_elements = resp.read_ub4()? as usize;
        let mut array = <RowData>::with_capacity(num_elements);
        for _ in 0..num_elements {
            array.push(DbValue::scalar_from_response(
                resp, client, statement, db_type, false,
            )?);
        }
        Ok(Some(DbValue::Array(array)))
    }

    fn scalar_from_response(
        resp: &mut Response,
        client: &Client,
        statement: &CachedStatement,
        db_type: &'static DbType,
        in_fetch: bool,
    ) -> Result<Option<DbValue>, Error> {
        let mut value = match db_type.ora_type_num {
            constants::ORA_TYPE_NUM_BINARY_DOUBLE => {
                Ok(resp.read_value::<f64>()?.map(DbValue::BinaryDouble))
            }
            constants::ORA_TYPE_NUM_VECTOR => {
                Ok(resp.read_value_lob::<Vector>()?.map(DbValue::Vector))
            }
            constants::ORA_TYPE_NUM_BINARY_FLOAT => {
                Ok(resp.read_value::<f32>()?.map(DbValue::BinaryFloat))
            }
            constants::ORA_TYPE_NUM_BOOLEAN => {
                Ok(resp.read_value::<bool>()?.map(DbValue::Boolean))
            }
            constants::ORA_TYPE_NUM_DATE
            | constants::ORA_TYPE_NUM_TIMESTAMP
            | constants::ORA_TYPE_NUM_TIMESTAMP_LTZ
            | constants::ORA_TYPE_NUM_TIMESTAMP_TZ => Ok(resp
                .read_value::<OracleTimestamp>()?
                .map(DbValue::Timestamp)),
            constants::ORA_TYPE_NUM_INTERVAL_DS => Ok(resp
                .read_value::<OracleIntervalDS>()?
                .map(DbValue::IntervalDS)),
            constants::ORA_TYPE_NUM_INTERVAL_YM => Ok(resp
                .read_value::<OracleIntervalYM>()?
                .map(DbValue::IntervalYM)),
            constants::ORA_TYPE_NUM_NUMBER => {
                Ok(resp.read_value::<OracleNumber>()?.map(DbValue::Number))
            }
            constants::ORA_TYPE_NUM_CHAR
            | constants::ORA_TYPE_NUM_LONG
            | constants::ORA_TYPE_NUM_VARCHAR => {
                let bytes = resp.read_bytes_with_length()?;
                if bytes.is_empty() {
                    Ok(None)
                } else if db_type.csfrm == constants::CS_FORM_NCHAR {
                    let value = utils::utf16be_bytes_to_string(&bytes)?;
                    Ok(Some(DbValue::String(value)))
                } else {
                    let value = std::str::from_utf8(&bytes)?;
                    Ok(Some(DbValue::String(value.to_string())))
                }
            }
            constants::ORA_TYPE_NUM_RAW | constants::ORA_TYPE_NUM_LONG_RAW => {
                let bytes = resp.read_bytes_with_length()?;
                if bytes.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(DbValue::Bytes(bytes.to_vec())))
                }
            }
            constants::ORA_TYPE_NUM_ROWID => {
                match resp.read_short_length()? {
                    0 => Ok(None),
                    _ => Ok(Some(DbValue::Rowid(Rowid::deserialize(resp)?))),
                }
            }
            constants::ORA_TYPE_NUM_JSON => {
                Ok(resp.read_value_lob::<JsonValue>()?.map(DbValue::Json))
            }
            constants::ORA_TYPE_NUM_CLOB | constants::ORA_TYPE_NUM_BLOB => {
                Ok(Lob::from_resp(resp, db_type)?.map(DbValue::Lob))
            }
            constants::ORA_TYPE_NUM_CURSOR => Ok(Cursor::from_resp(
                resp,
                client,
                in_fetch,
                statement.options(),
            )?
            .map(|v| DbValue::Cursor(Some(v)))),
            _ => Err(Error::unsupported_db_type(db_type)),
        }?;
        if !in_fetch {
            let max_num_bytes = resp.read_sb4()? as isize;
            if max_num_bytes < 0
                && db_type.ora_type_num == constants::ORA_TYPE_NUM_BOOLEAN
            {
                value = None;
            } else if max_num_bytes != 0
                && let Some(internal_value) = value
            {
                let actual_num_bytes = match internal_value {
                    DbValue::String(v) => v.len(),
                    DbValue::Bytes(v) => v.len(),
                    _ => 0,
                };
                return Err(Error::column_truncated(
                    actual_num_bytes,
                    max_num_bytes as usize,
                ));
            }
        } else if db_type.ora_type_num == constants::ORA_TYPE_NUM_LONG
            || db_type.ora_type_num == constants::ORA_TYPE_NUM_LONG_RAW
        {
            let _null_indicator = resp.read_sb4()?;
            let _return_code = resp.read_ub4()?;
        }
        Ok(value)
    }

    pub(crate) fn from_response(
        resp: &mut Response,
        client: &Client,
        statement: &CachedStatement,
        metadata: &Metadata,
        in_fetch: bool,
        column_num: usize,
    ) -> Result<Option<DbValue>, Error> {
        if in_fetch && metadata.is_null_by_describe() {
            return Ok(None);
        } else if resp.is_duplicate_data(column_num) {
            let last_row = resp.get_last_row_fetched();
            return Ok(last_row[column_num].clone());
        }

        if metadata.is_array() {
            DbValue::array_from_response(
                resp,
                client,
                statement,
                metadata.db_type(),
            )
        } else {
            DbValue::scalar_from_response(
                resp,
                client,
                statement,
                metadata.db_type(),
                in_fetch,
            )
        }
    }

    /// Returns the type of the stored value.
    pub(crate) fn type_name(&self) -> &str {
        match self {
            DbValue::Array(_) => "array",
            DbValue::BinaryDouble(_) => "binary_double",
            DbValue::BinaryFloat(_) => "binary_float",
            DbValue::Boolean(_) => "boolean",
            DbValue::Bytes(_) => "raw",
            DbValue::Cursor(_) => "cursor",
            DbValue::IntervalDS(_) => "interval day to second",
            DbValue::IntervalYM(_) => "interval year to month",
            DbValue::Json(_) => "json",
            DbValue::Number(_) => "number",
            DbValue::Rowid(_) => "rowid",
            DbValue::String(_) => "string",
            DbValue::Timestamp(_) => "timestamp",
            DbValue::Vector(_) => "vector",
            DbValue::Lob(_) => "lob",
        }
    }
}

/// Trait which transforms database values from the format required by the
/// database to ones usable by applications.
pub trait FromDbValue {
    fn from_db_value(db_value: &Option<DbValue>) -> Result<Self, Error>
    where
        Self: Sized;
}

impl FromDbValue for bool {
    fn from_db_value(db_value_opt: &Option<DbValue>) -> Result<bool, Error> {
        if let Some(db_value) = db_value_opt {
            match db_value {
                DbValue::Boolean(value) => Ok(*value),
                _ => Err(Error::unsupported_conversion(
                    db_value.type_name(),
                    "bool",
                )),
            }
        } else {
            Err(Error::value_was_null())
        }
    }
}

impl FromDbValue for f32 {
    fn from_db_value(db_value_opt: &Option<DbValue>) -> Result<f32, Error> {
        if let Some(db_value) = db_value_opt {
            match db_value {
                DbValue::BinaryFloat(value) => Ok(*value),
                DbValue::Number(value) => value
                    .to_string()
                    .parse::<f32>()
                    .map_err(|e| Error::unexpected_error(Box::new(e))),
                _ => Err(Error::unsupported_conversion(
                    db_value.type_name(),
                    "f32",
                )),
            }
        } else {
            Err(Error::value_was_null())
        }
    }
}

impl FromDbValue for f64 {
    fn from_db_value(db_value_opt: &Option<DbValue>) -> Result<f64, Error> {
        if let Some(db_value) = db_value_opt {
            match db_value {
                DbValue::BinaryDouble(value) => Ok(*value),
                DbValue::Number(value) => value
                    .to_string()
                    .parse::<f64>()
                    .map_err(|e| Error::unexpected_error(Box::new(e))),
                _ => Err(Error::unsupported_conversion(
                    db_value.type_name(),
                    "f64",
                )),
            }
        } else {
            Err(Error::value_was_null())
        }
    }
}

impl FromDbValue for JsonValue {
    fn from_db_value(db_value_opt: &Option<DbValue>) -> Result<Self, Error> {
        if let Some(db_value) = db_value_opt {
            match db_value {
                DbValue::Json(value) => Ok(value.clone()),
                _ => Err(Error::unsupported_conversion(
                    db_value.type_name(),
                    "JsonValue",
                )),
            }
        } else {
            Err(Error::value_was_null())
        }
    }
}

impl FromDbValue for OracleIntervalDS {
    fn from_db_value(
        db_value_opt: &Option<DbValue>,
    ) -> Result<OracleIntervalDS, Error> {
        if let Some(db_value) = db_value_opt {
            match db_value {
                DbValue::IntervalDS(value) => Ok(*value),
                _ => Err(Error::unsupported_conversion(
                    db_value.type_name(),
                    "OracleIntervalDS",
                )),
            }
        } else {
            Err(Error::value_was_null())
        }
    }
}

impl FromDbValue for OracleIntervalYM {
    fn from_db_value(
        db_value_opt: &Option<DbValue>,
    ) -> Result<OracleIntervalYM, Error> {
        if let Some(db_value) = db_value_opt {
            match db_value {
                DbValue::IntervalYM(value) => Ok(*value),
                _ => Err(Error::unsupported_conversion(
                    db_value.type_name(),
                    "OracleIntervalYM",
                )),
            }
        } else {
            Err(Error::value_was_null())
        }
    }
}

impl FromDbValue for OracleNumber {
    fn from_db_value(
        db_value_opt: &Option<DbValue>,
    ) -> Result<OracleNumber, Error> {
        if let Some(db_value) = db_value_opt {
            match db_value {
                DbValue::Number(value) => Ok(*value),
                _ => Err(Error::unsupported_conversion(
                    db_value.type_name(),
                    "OracleNumber",
                )),
            }
        } else {
            Err(Error::value_was_null())
        }
    }
}

impl FromDbValue for OracleTimestamp {
    fn from_db_value(
        db_value_opt: &Option<DbValue>,
    ) -> Result<OracleTimestamp, Error> {
        if let Some(db_value) = db_value_opt {
            match db_value {
                DbValue::Timestamp(value) => Ok(*value),
                _ => Err(Error::unsupported_conversion(
                    db_value.type_name(),
                    "OracleTimestamp",
                )),
            }
        } else {
            Err(Error::value_was_null())
        }
    }
}

impl FromDbValue for String {
    fn from_db_value(db_value_opt: &Option<DbValue>) -> Result<String, Error> {
        if let Some(db_value) = db_value_opt {
            match db_value {
                DbValue::String(val) => Ok(val.into()),
                DbValue::Rowid(val) => Ok(val.to_string()),
                _ => Err(Error::unsupported_conversion(
                    db_value.type_name(),
                    "String",
                )),
            }
        } else {
            Err(Error::value_was_null())
        }
    }
}

impl FromDbValue for Vec<u8> {
    fn from_db_value(
        db_value_opt: &Option<DbValue>,
    ) -> Result<Vec<u8>, Error> {
        if let Some(db_value) = db_value_opt {
            match db_value {
                DbValue::Bytes(val) => Ok(val.to_vec()),
                _ => Err(Error::unsupported_conversion(
                    db_value.type_name(),
                    "Bytes",
                )),
            }
        } else {
            Err(Error::value_was_null())
        }
    }
}

impl FromDbValue for Lob {
    fn from_db_value(db_value_opt: &Option<DbValue>) -> Result<Lob, Error> {
        if let Some(db_value) = db_value_opt {
            match db_value {
                DbValue::Lob(val) => Ok(val.clone()),
                _ => Err(Error::unsupported_conversion(
                    db_value.type_name(),
                    "Lob",
                )),
            }
        } else {
            Err(Error::value_was_null())
        }
    }
}
impl FromDbValue for Vector {
    fn from_db_value(db_value_opt: &Option<DbValue>) -> Result<Self, Error> {
        if let Some(db_value) = db_value_opt {
            match db_value {
                DbValue::Vector(v) => Ok(v.clone()),
                _ => Err(Error::unsupported_conversion(
                    db_value.type_name(),
                    "Vector",
                )),
            }
        } else {
            Err(Error::value_was_null())
        }
    }
}

trait ArrayElementType: FromDbValue {}

impl ArrayElementType for bool {}
impl ArrayElementType for f32 {}
impl ArrayElementType for f64 {}
impl ArrayElementType for OracleIntervalDS {}
impl ArrayElementType for OracleIntervalYM {}
impl ArrayElementType for OracleNumber {}
impl ArrayElementType for OracleTimestamp {}
impl ArrayElementType for String {}
impl ArrayElementType for Vec<u8> {}

impl<T> FromDbValue for Vec<T>
where
    T: ArrayElementType,
{
    fn from_db_value(db_value_opt: &Option<DbValue>) -> Result<Vec<T>, Error> {
        if let Some(db_value) = db_value_opt {
            match db_value {
                DbValue::Array(db_array) => {
                    let mut array = Vec::<T>::with_capacity(db_array.len());
                    for element_value in db_array {
                        array.push(<T>::from_db_value(element_value)?);
                    }
                    Ok(array)
                }
                _ => Err(Error::unsupported_conversion(
                    db_value.type_name(),
                    std::any::type_name::<T>(),
                )),
            }
        } else {
            Err(Error::value_was_null())
        }
    }
}

impl<T> FromDbValue for Option<T>
where
    T: FromDbValue,
{
    fn from_db_value(
        db_value_opt: &Option<DbValue>,
    ) -> Result<Option<T>, Error> {
        if db_value_opt.is_none() {
            Ok(None)
        } else {
            let result = <T>::from_db_value(db_value_opt)?;
            Ok(Some(result))
        }
    }
}

/// Trait which transforms values used by applications into the format required
/// by the database.
pub trait ToDbValue: ToBuf {
    fn db_type(&self) -> &'static DbType;
    fn max_size(&self) -> usize {
        0
    }
}

impl ToDbValue for String {
    fn db_type(&self) -> &'static DbType {
        &DB_TYPE_VARCHAR
    }

    fn max_size(&self) -> usize {
        self.len()
    }
}

impl ToDbValue for &str {
    fn db_type(&self) -> &'static DbType {
        &DB_TYPE_VARCHAR
    }

    fn max_size(&self) -> usize {
        self.len()
    }
}

impl ToDbValue for Vec<u8> {
    fn db_type(&self) -> &'static DbType {
        &DB_TYPE_RAW
    }

    fn max_size(&self) -> usize {
        self.len()
    }
}

impl ToDbValue for &[u8] {
    fn db_type(&self) -> &'static DbType {
        &DB_TYPE_RAW
    }

    fn max_size(&self) -> usize {
        self.len()
    }
}

impl ToDbValue for bool {
    fn db_type(&self) -> &'static DbType {
        &DB_TYPE_BOOLEAN
    }
}

impl ToDbValue for f32 {
    fn db_type(&self) -> &'static DbType {
        &DB_TYPE_BINARY_FLOAT
    }
}

impl ToDbValue for f64 {
    fn db_type(&self) -> &'static DbType {
        &DB_TYPE_BINARY_DOUBLE
    }
}

macro_rules! impl_traits_for_integers {
    ($($t:ty), *) => {
        $(
            impl From<$t> for OracleNumber {
                fn from(value: $t) -> Self {
                    value.to_string().parse().unwrap()
                }
            }
            impl FromDbValue for $t {
                fn from_db_value(
                    db_value_opt: &Option<DbValue>
                ) -> Result<$t, Error> {
                    if let Some(db_value) = db_value_opt {
                        match db_value {
                            DbValue::Number(value) => {
                                let str_val = value.to_string();
                                match str_val.parse::<$t>() {
                                    Ok(num) => Ok(num),
                                    Err(_) => Err(Error::unsupported_conversion(
                                        db_value.type_name(),
                                        std::any::type_name::<$t>(),
                                    )),
                                }
                            }
                            _ => Err(Error::unsupported_conversion(
                                db_value.type_name(),
                                std::any::type_name::<$t>(),
                            )),
                        }
                    } else {
                        Err(Error::value_was_null())
                    }
                }
            }
            impl ToBuf for $t {
                fn to_buf(
                    &self,
                    buf: &mut WriteBuffer,
                    db_type: &'static DbType,
                    write_length: bool,
                ) {
                    OracleNumber::from(*self).to_buf(buf, db_type, write_length)
                }
            }
            impl ToDbValue for $t {
                fn db_type(&self) -> &'static DbType {
                    &DB_TYPE_NUMBER
                }
            }
        )*
    };
}

impl_traits_for_integers!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

impl ToDbValue for JsonValue {
    fn db_type(&self) -> &'static DbType {
        &DB_TYPE_JSON
    }
}

impl ToDbValue for OracleIntervalDS {
    fn db_type(&self) -> &'static DbType {
        &DB_TYPE_INTERVAL_DS
    }
}

impl ToDbValue for OracleIntervalYM {
    fn db_type(&self) -> &'static DbType {
        &DB_TYPE_INTERVAL_YM
    }
}

impl ToDbValue for OracleNumber {
    fn db_type(&self) -> &'static DbType {
        &DB_TYPE_NUMBER
    }
}

impl ToDbValue for OracleTimestamp {
    fn db_type(&self) -> &'static DbType {
        &DB_TYPE_TIMESTAMP
    }
}

impl ToDbValue for Vector {
    fn db_type(&self) -> &'static DbType {
        &DB_TYPE_VECTOR
    }
}

impl<T> ToDbValue for Option<T>
where
    T: ToDbValue + Default,
{
    fn db_type(&self) -> &'static DbType {
        if let Some(value) = self {
            value.db_type()
        } else {
            <T>::default().db_type()
        }
    }

    fn max_size(&self) -> usize {
        if let Some(value) = self {
            <T>::max_size(value)
        } else {
            0
        }
    }
}
