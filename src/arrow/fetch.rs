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
// fetch.rs
//
// Defines support for fetching query results as Arrow record batches.
//-----------------------------------------------------------------------------

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use arrow_array::builder::ArrayBuilder;
use arrow_array::builder::make_builder;
use arrow_array::builder::{
    BinaryBuilder, BooleanBuilder, Decimal128Builder, Float32Builder,
    Float64Builder, Int8Builder, Int16Builder, Int32Builder, Int64Builder,
    LargeBinaryBuilder, LargeStringBuilder, StringBuilder,
    TimestampMicrosecondBuilder, TimestampMillisecondBuilder,
    TimestampNanosecondBuilder, TimestampSecondBuilder, UInt8Builder,
    UInt16Builder, UInt32Builder, UInt64Builder,
};
use arrow_array::types::{
    Decimal128Type, validate_decimal_precision_and_scale,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef, TimeUnit};

use crate::bind_params::BindParameters;
use crate::client::Client;
use crate::client::ClientRef;
use crate::constants;
use crate::error::Error;
use crate::metadata::Metadata;
use crate::ora_type::OracleNumber;
use crate::ora_type::OracleTimestamp;
use crate::row::DbRow;
use crate::row::Row;
use crate::statement::CachedStatement;

/// Returns the default schema to use for fetching data in the Arrow format.
fn default_schema(columns: &[Metadata]) -> Result<Schema, Error> {
    let mut fields: Vec<Field> = Vec::with_capacity(columns.len());
    for col in columns {
        let data_type = default_type(col)?;
        let metadata =
            HashMap::from([("oracle.type-name".to_string(), col.data_type())]);
        fields.push(
            Field::new(col.name(), data_type, col.nullable())
                .with_metadata(metadata),
        );
    }
    Ok(Schema::new(fields))
}

/// Returns the default type to use for fetching data in the Arrow format.
fn default_type(col: &Metadata) -> Result<DataType, Error> {
    let db_type = col.db_type();
    if db_type == crate::DB_TYPE_LONG
        || db_type == crate::DB_TYPE_LONG_NVARCHAR
    {
        Ok(DataType::LargeUtf8)
    } else if db_type == crate::DB_TYPE_LONG_RAW {
        Ok(DataType::LargeBinary)
    } else if db_type.is_string_type() {
        Ok(DataType::Utf8)
    } else if db_type.is_binary_type() {
        Ok(DataType::Binary)
    } else if col.db_type().is_date_type() {
        let unit = match col.scale() {
            0 => TimeUnit::Second,
            1..=3 => TimeUnit::Millisecond,
            4..=6 => TimeUnit::Microsecond,
            _ => TimeUnit::Nanosecond,
        };
        Ok(DataType::Timestamp(unit, None))
    } else {
        match col.db_type().ora_type_num {
            constants::ORA_TYPE_NUM_BINARY_DOUBLE => Ok(DataType::Float64),
            constants::ORA_TYPE_NUM_BINARY_FLOAT => Ok(DataType::Float32),
            constants::ORA_TYPE_NUM_BOOLEAN => Ok(DataType::Boolean),
            constants::ORA_TYPE_NUM_NUMBER => {
                let precision = col.precision();
                let scale = col.scale();
                // Arrow decimal values require a fixed precision and scale.
                // Oracle uses a precision of 0 to indicate that the precision
                // and scale of the stored numbers may vary. Arrow decimal
                // values also require that the absolute value of the scale
                // does not exceed precision. If these constraints are not met
                // then Float64 is used instead.
                if precision == 0 || scale.unsigned_abs() > precision {
                    Ok(DataType::Float64)
                } else if scale != 0 || precision > 18 {
                    Ok(DataType::Decimal128(precision, scale))
                } else if precision <= 2 {
                    Ok(DataType::Int8)
                } else if precision <= 4 {
                    Ok(DataType::Int16)
                } else if precision <= 9 {
                    Ok(DataType::Int32)
                } else {
                    Ok(DataType::Int64)
                }
            }
            _ => Err(Error::unsupported_db_type(col.db_type())),
        }
    }
}

/// Performs a query with one or more sets of parameters and returns a single
/// RecordBatch containing all of the data.
pub(crate) fn query_single_batch(
    client: &mut Client,
    statement: &mut CachedStatement,
    client_ref: &ClientRef,
    params: BindParameters,
) -> Result<arrow_array::RecordBatch, Error> {
    // perform first query to determine the schema
    let mut last_row: Option<DbRow> = None;
    let mut response =
        client.execute(statement, client_ref, params.slice(0, 1), false)?;
    let metadata = statement.out_metadata().to_vec();
    let schema = if let Some(schema) = statement.options().requested_schema() {
        if schema.fields().len() != metadata.len() {
            return Err(Error::wrong_arrow_schema_length(
                schema.fields().len(),
                metadata.len(),
            ));
        }
        for (column, field) in metadata.iter().zip(schema.fields()) {
            validate_fetch_type(column, field)?;
        }
        schema.clone()
    } else {
        Arc::new(default_schema(&metadata)?)
    };
    let mut creator = RecordBatchCreator::new(schema)?;
    let column_info = Arc::new(metadata);
    for index in 0..params.num_rows() {
        if index > 0 {
            response = client.execute(
                statement,
                client_ref,
                params.slice(index, 1),
                false,
            )?;
        }
        loop {
            if let Some(rows) = response.take_rows() {
                let mut rows: VecDeque<DbRow> = rows.into();
                while let Some(row) = rows.pop_front() {
                    if rows.is_empty() {
                        last_row = Some(row.clone());
                    }
                    creator.add_row(&Row::new(&column_info, row))?;
                }
            }
            if response.is_end_of_fetch() {
                break;
            }
            response = client.fetch(statement, client_ref, last_row.take())?;
        }
    }
    creator.create_batch()
}

/// Validates that an Oracle result column supports the requested Arrow type.
fn validate_fetch_type(column: &Metadata, field: &Field) -> Result<(), Error> {
    let data_type = field.data_type();
    if let DataType::Decimal128(precision, scale) = data_type {
        validate_decimal_precision_and_scale::<Decimal128Type>(
            *precision, *scale,
        )?;
    }
    let db_type = column.db_type();
    let is_compatible = if db_type == crate::DB_TYPE_NUMBER {
        matches!(
            data_type,
            DataType::Decimal128(_, _)
                | DataType::Float32
                | DataType::Float64
                | DataType::Int8
                | DataType::Int16
                | DataType::Int32
                | DataType::Int64
                | DataType::UInt8
                | DataType::UInt16
                | DataType::UInt32
                | DataType::UInt64
                | DataType::Utf8
                | DataType::LargeUtf8
        )
    } else if db_type.is_binary_type() {
        matches!(data_type, DataType::Binary | DataType::LargeBinary)
    } else if db_type == crate::DB_TYPE_BOOLEAN {
        matches!(data_type, DataType::Boolean)
    } else if db_type.is_date_type() {
        matches!(data_type, DataType::Timestamp(_, _))
    } else if db_type.is_string_type() {
        matches!(data_type, DataType::Utf8 | DataType::LargeUtf8)
    } else if db_type == crate::DB_TYPE_BINARY_FLOAT {
        matches!(data_type, DataType::Float32)
    } else if db_type == crate::DB_TYPE_BINARY_DOUBLE {
        matches!(data_type, DataType::Float64)
    } else {
        false
    };
    if is_compatible {
        Ok(())
    } else {
        Err(Error::unsupported_conversion(
            &format!("{} column {}", column.data_type(), field.name()),
            &data_type.to_string(),
        ))
    }
}

/// Structure used for creating a record batch from the results of a fetch.
struct RecordBatchCreator {
    schema: SchemaRef,
    builders: Vec<Box<dyn ArrayBuilder>>,
}

impl RecordBatchCreator {
    /// Adds a row to the RecordBatch is being created.
    fn add_row(&mut self, row: &Row) -> Result<(), Error> {
        let fields = self.schema.fields();
        for (column_index, field) in fields.into_iter().enumerate() {
            let builder = &mut self.builders[column_index];
            Self::add_value(row, field, builder, column_index)?;
        }
        Ok(())
    }

    /// Adds a value from the row to the Arrow array.
    fn add_value(
        row: &Row,
        field: &Field,
        builder: &mut Box<dyn ArrayBuilder>,
        column_index: usize,
    ) -> Result<(), Error> {
        match field.data_type() {
            DataType::Binary => builder
                .as_any_mut()
                .downcast_mut::<BinaryBuilder>()
                .unwrap()
                .append_option(row.get::<Option<Vec<u8>>>(column_index)?),
            DataType::LargeBinary => builder
                .as_any_mut()
                .downcast_mut::<LargeBinaryBuilder>()
                .unwrap()
                .append_option(row.get::<Option<Vec<u8>>>(column_index)?),
            DataType::Boolean => {
                builder
                    .as_any_mut()
                    .downcast_mut::<BooleanBuilder>()
                    .unwrap()
                    .append_option(row.get::<Option<bool>>(column_index)?);
            }
            DataType::Decimal128(precision, scale) => {
                let value = row
                    .get::<Option<OracleNumber>>(column_index)?
                    .map(|v| v.as_decimal128(*precision, *scale))
                    .transpose()?;
                builder
                    .as_any_mut()
                    .downcast_mut::<Decimal128Builder>()
                    .unwrap()
                    .append_option(value);
            }
            DataType::Float32 => {
                builder
                    .as_any_mut()
                    .downcast_mut::<Float32Builder>()
                    .unwrap()
                    .append_option(row.get::<Option<f32>>(column_index)?);
            }
            DataType::Float64 => {
                builder
                    .as_any_mut()
                    .downcast_mut::<Float64Builder>()
                    .unwrap()
                    .append_option(row.get::<Option<f64>>(column_index)?);
            }
            DataType::Int8 => {
                builder
                    .as_any_mut()
                    .downcast_mut::<Int8Builder>()
                    .unwrap()
                    .append_option(row.get::<Option<i8>>(column_index)?);
            }
            DataType::Int16 => {
                builder
                    .as_any_mut()
                    .downcast_mut::<Int16Builder>()
                    .unwrap()
                    .append_option(row.get::<Option<i16>>(column_index)?);
            }
            DataType::Int32 => {
                builder
                    .as_any_mut()
                    .downcast_mut::<Int32Builder>()
                    .unwrap()
                    .append_option(row.get::<Option<i32>>(column_index)?);
            }
            DataType::Int64 => {
                builder
                    .as_any_mut()
                    .downcast_mut::<Int64Builder>()
                    .unwrap()
                    .append_option(row.get::<Option<i64>>(column_index)?);
            }
            DataType::Timestamp(time_unit, _) => {
                let value = row
                    .get::<Option<OracleTimestamp>>(column_index)?
                    .map(|v| v.to_timestamp(*time_unit))
                    .transpose()?;
                match time_unit {
                    TimeUnit::Second => builder
                        .as_any_mut()
                        .downcast_mut::<TimestampSecondBuilder>()
                        .unwrap()
                        .append_option(value),
                    TimeUnit::Millisecond => builder
                        .as_any_mut()
                        .downcast_mut::<TimestampMillisecondBuilder>()
                        .unwrap()
                        .append_option(value),
                    TimeUnit::Microsecond => builder
                        .as_any_mut()
                        .downcast_mut::<TimestampMicrosecondBuilder>()
                        .unwrap()
                        .append_option(value),
                    TimeUnit::Nanosecond => builder
                        .as_any_mut()
                        .downcast_mut::<TimestampNanosecondBuilder>()
                        .unwrap()
                        .append_option(value),
                }
            }
            DataType::UInt8 => {
                builder
                    .as_any_mut()
                    .downcast_mut::<UInt8Builder>()
                    .unwrap()
                    .append_option(row.get::<Option<u8>>(column_index)?);
            }
            DataType::UInt16 => {
                builder
                    .as_any_mut()
                    .downcast_mut::<UInt16Builder>()
                    .unwrap()
                    .append_option(row.get::<Option<u16>>(column_index)?);
            }
            DataType::UInt32 => {
                builder
                    .as_any_mut()
                    .downcast_mut::<UInt32Builder>()
                    .unwrap()
                    .append_option(row.get::<Option<u32>>(column_index)?);
            }
            DataType::UInt64 => {
                builder
                    .as_any_mut()
                    .downcast_mut::<UInt64Builder>()
                    .unwrap()
                    .append_option(row.get::<Option<u64>>(column_index)?);
            }
            DataType::Utf8 => builder
                .as_any_mut()
                .downcast_mut::<StringBuilder>()
                .unwrap()
                .append_option(row.get::<Option<String>>(column_index)?),
            DataType::LargeUtf8 => builder
                .as_any_mut()
                .downcast_mut::<LargeStringBuilder>()
                .unwrap()
                .append_option(row.get::<Option<String>>(column_index)?),
            _ => {
                return Err(Error::unsupported_arrow_type(
                    field.data_type().to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Creates the RecordBatch and returns it.
    fn create_batch(&mut self) -> Result<arrow_array::RecordBatch, Error> {
        let arrays: Vec<arrow_array::ArrayRef> =
            self.builders.iter_mut().map(|b| b.finish()).collect();
        Ok(arrow_array::RecordBatch::try_new(self.schema.clone(), arrays)?)
    }

    /// Creates a new structure for creating a RecordBatch
    fn new(schema: SchemaRef) -> Result<Self, Error> {
        let builders = schema
            .fields()
            .iter()
            .map(|f| make_builder(f.data_type(), 0))
            .collect();
        Ok(Self { schema, builders })
    }
}
