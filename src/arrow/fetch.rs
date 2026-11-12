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

use std::collections::HashMap;
use std::sync::Arc;

use arrow_array::builder::ArrayBuilder;
use arrow_array::builder::make_builder;
use arrow_array::builder::{
    BinaryBuilder, BooleanBuilder, Decimal128Builder, Decimal256Builder,
    Float32Builder, Float64Builder, Int8Builder, Int16Builder, Int32Builder,
    Int64Builder, LargeBinaryBuilder, LargeStringBuilder, StringBuilder,
    TimestampMicrosecondBuilder, TimestampMillisecondBuilder,
    TimestampNanosecondBuilder, TimestampSecondBuilder, UInt8Builder,
    UInt16Builder, UInt32Builder, UInt64Builder,
};
use arrow_array::types::{
    Decimal128Type, Decimal256Type, validate_decimal_precision_and_scale,
};
use arrow_array::{RecordBatch, RecordBatchReader};
use arrow_schema::{ArrowError, DataType, Field, Schema, SchemaRef, TimeUnit};

use crate::bind_params::BindParameters;
use crate::constants;
use crate::error::Error;
use crate::metadata::Metadata;
use crate::ora_type::OracleNumber;
use crate::ora_type::OracleTimestamp;
use crate::response::Response;
use crate::row::DbRow;
use crate::row::Row;
use crate::statement::Statement;

/// Streams query results as Arrow record batches.
///
/// Created by
/// [Connection::query_arrow_batches()](crate::Connection::query_arrow_batches)
/// or [Statement::query_arrow_batches()].
///
/// Implements Arrow's [RecordBatchReader] interface, which exposes a schema
/// and yields one [RecordBatch] at a time. Use it to process query results
/// incrementally instead of collecting the full result into one batch.
///
/// The schema is available before iteration. Use
/// [crate::StatementBuilder::arrow_batch_size()] to control output batch size.
/// The default output batch size is 1,024 rows. Errors are returned as
/// [ArrowError::ExternalError] containing the original [Error].
pub struct ArrowBatchReader<'params> {
    statement: Statement,
    params: BindParameters<'params>,
    param_index: usize,
    pending_rows: std::vec::IntoIter<DbRow>,
    column_info: Arc<Vec<Metadata>>,
    creator: RecordBatchCreator,
    last_row: Option<DbRow>,
    end_of_fetch: bool,
}

impl<'params> ArrowBatchReader<'params> {
    /// Fills an output batch with fetched rows. There are three possibilities
    /// for how output batches are determined:
    ///     - all rows from all inputs are placed in one batch
    ///     - all rows from one input are placed in one batch
    ///     - rows from all inputs are split into chunks corresponding to the
    ///       specified batch size
    fn get_next_batch(
        &mut self,
        single_batch: bool,
    ) -> Result<RecordBatch, Error> {
        let mut num_rows = 0;
        let batch_size = if single_batch {
            usize::MAX
        } else {
            self.statement.options().arrow_batch_size()
        };
        while num_rows < batch_size {
            // consume all of the rows that are in the buffer, up to the
            // maximum specified
            for row in self.pending_rows.by_ref().take(batch_size - num_rows) {
                self.creator.add_row(&Row::new(&self.column_info, row))?;
                num_rows += 1;
            }

            // if no rows remain, attempt to get more rows, either from the
            // current input or from the next input, if one is available
            if self.pending_rows.len() == 0 {
                let response = if !self.end_of_fetch {
                    self.statement.fetch(self.last_row.take())?
                } else if self.param_index < self.params.num_rows() {
                    let response = self.statement.get_execute_response(
                        self.params.slice(self.param_index, 1),
                        false,
                    )?;
                    self.param_index += 1;
                    response
                } else {
                    break;
                };
                self.process_response(response);
            }
        }
        self.creator.create_batch()
    }

    /// Returns whether any more batches remain. This is true if there are
    /// pending rows, pending fetches or pending inputs.
    fn has_more_batches(&self) -> bool {
        self.pending_rows.len() > 0
            || !self.end_of_fetch
            || self.param_index < self.params.num_rows()
    }

    /// Processes a response from the database.
    fn process_response(&mut self, mut response: Response) {
        self.end_of_fetch = response.is_end_of_fetch();
        if let Some(rows) = response.take_rows() {
            if self.end_of_fetch {
                self.last_row = None;
            } else {
                self.last_row = rows.last().cloned();
            }
            self.pending_rows = rows.into_iter();
        }
    }

    /// Collects all responses into one batch without concatenating arrays.
    pub(crate) fn into_single_batch(mut self) -> Result<RecordBatch, Error> {
        self.get_next_batch(true)
    }

    /// Executes the first set of parameters and determines the schema.
    pub(crate) fn new(
        mut statement: Statement,
        params: BindParameters<'params>,
    ) -> Result<Self, Error> {
        let response =
            statement.get_execute_response(params.slice(0, 1), false)?;
        let column_info = Arc::new(statement.out_metadata().to_vec());
        let schema = if let Some(schema) =
            statement.options().requested_schema()
        {
            if schema.fields().len() != column_info.len() {
                return Err(Error::wrong_arrow_schema_length(
                    schema.fields().len(),
                    column_info.len(),
                ));
            }
            for (column, field) in column_info.iter().zip(schema.fields()) {
                validate_fetch_type(column, field)?;
            }
            schema.clone()
        } else {
            Arc::new(default_schema(&column_info)?)
        };
        let creator = RecordBatchCreator::new(schema)?;
        let mut reader = Self {
            statement,
            params,
            param_index: 1,
            pending_rows: Vec::new().into_iter(),
            column_info,
            creator,
            last_row: None,
            end_of_fetch: false,
        };
        reader.process_response(response);
        Ok(reader)
    }

    /// Returns the database column metadata, available before the first batch.
    pub fn columns(&self) -> &[Metadata] {
        &self.column_info
    }

    /// Returns the Arrow schema, available before the first batch.
    pub fn schema(&self) -> SchemaRef {
        self.creator.schema.clone()
    }
}

impl Iterator for ArrowBatchReader<'_> {
    type Item = Result<RecordBatch, ArrowError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.has_more_batches() {
            Some(self.get_next_batch(false).map_err(|error| {
                self.end_of_fetch = true;
                self.param_index = self.params.num_rows();
                ArrowError::ExternalError(Box::new(error))
            }))
        } else {
            None
        }
    }
}

impl RecordBatchReader for ArrowBatchReader<'_> {
    fn schema(&self) -> SchemaRef {
        ArrowBatchReader::schema(self)
    }
}

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
        let timezone = match db_type.ora_type_num {
            constants::ORA_TYPE_NUM_TIMESTAMP_TZ
            | constants::ORA_TYPE_NUM_TIMESTAMP_LTZ => Some("UTC".into()),
            _ => None,
        };
        Ok(DataType::Timestamp(unit, timezone))
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

/// Validates that an Oracle result column supports the requested Arrow type.
fn validate_fetch_type(column: &Metadata, field: &Field) -> Result<(), Error> {
    let data_type = field.data_type();
    if let DataType::Decimal128(precision, scale) = data_type {
        validate_decimal_precision_and_scale::<Decimal128Type>(
            *precision, *scale,
        )?;
    }
    if let DataType::Decimal256(precision, scale) = data_type {
        validate_decimal_precision_and_scale::<Decimal256Type>(
            *precision, *scale,
        )?;
    }
    let db_type = column.db_type();
    let is_compatible = if db_type == crate::DB_TYPE_NUMBER {
        matches!(
            data_type,
            DataType::Decimal128(_, _)
                | DataType::Decimal256(_, _)
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
            DataType::Decimal256(precision, scale) => {
                let value = row
                    .get::<Option<OracleNumber>>(column_index)?
                    .map(|v| v.as_decimal256(*precision, *scale))
                    .transpose()?;
                builder
                    .as_any_mut()
                    .downcast_mut::<Decimal256Builder>()
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
    fn create_batch(&mut self) -> Result<RecordBatch, Error> {
        let arrays: Vec<arrow_array::ArrayRef> =
            self.builders.iter_mut().map(|b| b.finish()).collect();
        Ok(RecordBatch::try_new(self.schema.clone(), arrays)?)
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
