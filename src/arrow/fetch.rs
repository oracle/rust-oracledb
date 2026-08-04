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

use arrow_array::builder::ArrayBuilder;
use arrow_array::builder::make_builder;
use arrow_array::builder::{
    BinaryBuilder, BooleanBuilder, Decimal128Builder, Float32Builder,
    Float64Builder, Int8Builder, Int16Builder, Int32Builder, Int64Builder,
    StringBuilder, TimestampMicrosecondBuilder, UInt8Builder, UInt16Builder,
    UInt32Builder, UInt64Builder,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef, TimeUnit};

use crate::bind_params::BindParameters;
use crate::constants;
use crate::cursor::Cursor;
use crate::error::Error;
use crate::metadata::Metadata;
use crate::ora_type::OracleNumber;
use crate::ora_type::OracleTimestamp;
use crate::row::Row;
use crate::statement::StatementHolder;

/// Returns the default schema to use for fetching data in the Arrow format.
fn default_schema(columns: &[Metadata]) -> Result<Schema, Error> {
    let mut fields: Vec<Field> = Vec::with_capacity(columns.len());
    for col in columns {
        let data_type = default_type(col)?;
        fields.push(Field::new(col.name(), data_type, col.nullable()));
    }
    Ok(Schema::new(fields))
}

/// Returns the default type to use for fetching data in the Arrow format.
fn default_type(col: &Metadata) -> Result<DataType, Error> {
    if col.db_type().is_string_type() {
        Ok(DataType::Utf8)
    } else if col.db_type().is_binary_type() {
        Ok(DataType::Binary)
    } else if col.db_type().is_date_type() {
        Ok(DataType::Timestamp(TimeUnit::Microsecond, None))
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
    holder: StatementHolder,
    params: BindParameters,
) -> Result<arrow_array::RecordBatch, Error> {
    // perform first query to determine the schema
    let mut cursor = Cursor::new(holder);
    cursor.execute(params.slice(0, 1))?;
    let mut creator = RecordBatchCreator::new(cursor.columns())?;
    for row in cursor.by_ref() {
        creator.add_row(&row?)?;
    }

    // perform any other queries and continue appending to the RecordBatch
    for index in 1..params.num_rows() {
        cursor.execute(params.slice(index, 1))?;
        for row in cursor.by_ref() {
            creator.add_row(&row?)?;
        }
    }
    creator.create_batch()
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
            DataType::Binary | DataType::LargeBinary => {
                builder
                    .as_any_mut()
                    .downcast_mut::<BinaryBuilder>()
                    .unwrap()
                    .append_option(row.get::<Option<Vec<u8>>>(column_index)?);
            }
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
            DataType::Timestamp(TimeUnit::Microsecond, _) => {
                let value = row
                    .get::<Option<OracleTimestamp>>(column_index)?
                    .map(|v| v.to_timestamp_micros())
                    .transpose()?;
                builder
                    .as_any_mut()
                    .downcast_mut::<TimestampMicrosecondBuilder>()
                    .unwrap()
                    .append_option(value)
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
            DataType::Utf8 | DataType::LargeUtf8 => {
                builder
                    .as_any_mut()
                    .downcast_mut::<StringBuilder>()
                    .unwrap()
                    .append_option(row.get::<Option<String>>(column_index)?);
            }
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
        Ok(arrow_array::RecordBatch::try_new(
            self.schema.clone(),
            arrays,
        )?)
    }

    /// Creates a new structure for creating a RecordBatch
    fn new(columns: &[Metadata]) -> Result<Self, Error> {
        let schema = default_schema(columns)?;
        let builders = schema
            .fields()
            .iter()
            .map(|f| make_builder(f.data_type(), 0))
            .collect();
        Ok(Self {
            schema: schema.into(),
            builders,
        })
    }
}
