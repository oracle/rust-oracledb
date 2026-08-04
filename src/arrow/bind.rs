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
// bind.rs
//
// Defines support for binding Arrow record batches to SQL statements.
//-----------------------------------------------------------------------------

use arrow_array::cast::AsArray;
use arrow_array::types::{
    Date32Type, Date64Type, Decimal128Type, Float32Type, Float64Type,
    Int8Type, Int16Type, Int32Type, Int64Type, TimestampMicrosecondType,
    TimestampMillisecondType, TimestampNanosecondType, TimestampSecondType,
    UInt8Type, UInt16Type, UInt32Type, UInt64Type,
};
use arrow_schema::DataType;
use arrow_schema::TimeUnit;

use crate::db_type;
use crate::db_value::ToDbValue;
use crate::error::Error;
use crate::metadata::Metadata;
use crate::ora_type::OracleNumber;
use crate::ora_type::OracleTimestamp;
use crate::write_buffer::WriteBuffer;

/// Utility function that returns the database data type to use for the given
/// Arrow column.
pub(crate) fn column_db_type(
    column: &std::sync::Arc<dyn arrow_array::Array>,
) -> Result<&'static db_type::DbType, Error> {
    match column.data_type() {
        DataType::Boolean => Ok(&db_type::DB_TYPE_BOOLEAN),
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64
        | DataType::Decimal128(_, _) => Ok(&db_type::DB_TYPE_NUMBER),
        DataType::Float32 => Ok(&db_type::DB_TYPE_BINARY_FLOAT),
        DataType::Float64 => Ok(&db_type::DB_TYPE_BINARY_DOUBLE),
        DataType::Date32 | DataType::Date64 | DataType::Timestamp(_, _) => {
            Ok(&db_type::DB_TYPE_TIMESTAMP)
        }
        DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View => {
            Ok(&db_type::DB_TYPE_VARCHAR)
        }
        DataType::Binary | DataType::LargeBinary | DataType::BinaryView => {
            Ok(&db_type::DB_TYPE_RAW)
        }
        _ => Err(Error::unsupported_arrow_type(
            column.data_type().to_string(),
        )),
    }
}

/// Utility function that returns the maximum size of the data found within the
/// Arrow column.
pub(crate) fn column_max_size(
    column: &std::sync::Arc<dyn arrow_array::Array>,
) -> usize {
    macro_rules! max_size {
        ($e:expr) => {
            $e.iter()
                .flatten()
                .map(|item| item.len())
                .max()
                .unwrap_or(0)
        };
    }
    match column.data_type() {
        DataType::Utf8 => max_size!(column.as_string::<i32>()),
        DataType::LargeUtf8 => max_size!(column.as_string::<i64>()),
        DataType::Utf8View => max_size!(column.as_string_view()),
        DataType::Binary => max_size!(column.as_binary::<i32>()),
        DataType::LargeBinary => max_size!(column.as_binary::<i64>()),
        DataType::BinaryView => max_size!(column.as_binary_view()),
        _ => 0,
    }
}

/// Utility function that writes the value found at the specified row index in
/// the Arrow array to the write buffer
pub(crate) fn write_to_buf(
    column: &std::sync::Arc<dyn arrow_array::Array>,
    row_index: usize,
    metadata: &Metadata,
    buf: &mut WriteBuffer,
) {
    let is_null = column.is_null(row_index);
    let value: &dyn ToDbValue = match column.data_type() {
        DataType::Boolean => {
            &(!is_null).then(|| column.as_boolean().value(row_index))
        }
        DataType::Int8 => &(!is_null)
            .then(|| column.as_primitive::<Int8Type>().value(row_index)),
        DataType::Int16 => &(!is_null)
            .then(|| column.as_primitive::<Int16Type>().value(row_index)),
        DataType::Int32 => &(!is_null)
            .then(|| column.as_primitive::<Int32Type>().value(row_index)),
        DataType::Int64 => &(!is_null)
            .then(|| column.as_primitive::<Int64Type>().value(row_index)),
        DataType::UInt8 => &(!is_null)
            .then(|| column.as_primitive::<UInt8Type>().value(row_index)),
        DataType::UInt16 => &(!is_null)
            .then(|| column.as_primitive::<UInt16Type>().value(row_index)),
        DataType::UInt32 => &(!is_null)
            .then(|| column.as_primitive::<UInt32Type>().value(row_index)),
        DataType::UInt64 => &(!is_null)
            .then(|| column.as_primitive::<UInt64Type>().value(row_index)),
        DataType::Float32 => &(!is_null)
            .then(|| column.as_primitive::<Float32Type>().value(row_index)),
        DataType::Float64 => &(!is_null)
            .then(|| column.as_primitive::<Float64Type>().value(row_index)),
        DataType::Decimal128(_, _) => &(!is_null).then(|| {
            let str_val = column
                .as_primitive::<Decimal128Type>()
                .value_as_string(row_index);
            let num: OracleNumber = str_val.parse().unwrap();
            num
        }),
        DataType::Date32 => &(!is_null).then(|| {
            OracleTimestamp::from(
                column
                    .as_primitive::<Date32Type>()
                    .value_as_datetime(row_index)
                    .unwrap(),
            )
        }),
        DataType::Date64 => &(!is_null).then(|| {
            OracleTimestamp::from(
                column
                    .as_primitive::<Date64Type>()
                    .value_as_datetime(row_index)
                    .unwrap(),
            )
        }),
        DataType::Timestamp(time_unit, _) => &(!is_null).then(|| {
            let ts = match time_unit {
                TimeUnit::Second => column
                    .as_primitive::<TimestampSecondType>()
                    .value_as_datetime(row_index),
                TimeUnit::Millisecond => column
                    .as_primitive::<TimestampMillisecondType>()
                    .value_as_datetime(row_index),
                TimeUnit::Microsecond => column
                    .as_primitive::<TimestampMicrosecondType>()
                    .value_as_datetime(row_index),
                TimeUnit::Nanosecond => column
                    .as_primitive::<TimestampNanosecondType>()
                    .value_as_datetime(row_index),
            }
            .unwrap();
            OracleTimestamp::from(ts)
        }),
        DataType::Utf8 => {
            &(!is_null).then(|| column.as_string::<i32>().value(row_index))
        }
        DataType::LargeUtf8 => {
            &(!is_null).then(|| column.as_string::<i64>().value(row_index))
        }
        DataType::Binary => {
            &(!is_null).then(|| column.as_binary::<i32>().value(row_index))
        }
        DataType::LargeBinary => {
            &(!is_null).then(|| column.as_binary::<i64>().value(row_index))
        }
        _ => todo!(),
    };
    value.to_buf(buf, metadata.db_type(), true);
}
