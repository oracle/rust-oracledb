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
// bind_params.rs
//
// Defines the structure representing bind parameters.
//-----------------------------------------------------------------------------

#[cfg(feature = "arrow")]
use crate::arrow;

use crate::ToDbValue;
use crate::error::Error;
use crate::statement::BindInfo;
use crate::write_buffer::WriteBuffer;

/// Values passed as bind parameters to the database.
pub enum BindParameters<'a> {
    /// A 2D slice of objects that implement the ToDbValue trait
    Slice(&'a [&'a [&'a dyn ToDbValue]]),
    /// An Arrow RecordBatch containing column arrays
    #[cfg(feature = "arrow")]
    Arrow(arrow_array::RecordBatch),
}

impl<'a> Default for BindParameters<'a> {
    fn default() -> Self {
        Self::Slice(&[&[]])
    }
}

impl<'a> From<&'a [&'a [&'a dyn ToDbValue]]> for BindParameters<'a> {
    fn from(params: &'a [&'a [&'a dyn ToDbValue]]) -> Self {
        Self::Slice(params)
    }
}

#[cfg(feature = "arrow")]
impl<'a> From<arrow_array::RecordBatch> for BindParameters<'a> {
    fn from(batch: arrow_array::RecordBatch) -> Self {
        Self::Arrow(batch)
    }
}

impl BindParameters<'_> {
    /// Returns the number of rows of data the bind parameters contain.
    pub(crate) fn num_rows(&self) -> usize {
        match self {
            Self::Slice(params) => params.len(),
            #[cfg(feature = "arrow")]
            Self::Arrow(batch) => batch.num_rows(),
        }
    }

    /// Returns a slice of the bind parameters at the given offset and for the
    /// specified length.
    pub(crate) fn slice(&self, offset: usize, length: usize) -> Self {
        match self {
            Self::Slice(params) => {
                Self::Slice(&params[offset..offset + length])
            }
            #[cfg(feature = "arrow")]
            Self::Arrow(batch) => Self::Arrow(batch.slice(offset, length)),
        }
    }

    /// Validates the bind parameters and updates the metadata associated with
    /// the bind variables for the statement.
    pub(crate) fn validate(
        &self,
        binds: &mut [BindInfo],
    ) -> Result<(), Error> {
        match self {
            Self::Slice(params) => {
                let mut first_row = true;
                for row in *params {
                    if row.len() != binds.len() {
                        return Err(Error::wrong_num_positional_binds(
                            binds.len(),
                            row.len(),
                        ));
                    }
                    for (value, bind_info) in row.iter().zip(binds.iter_mut())
                    {
                        bind_info.check_and_set_metadata(
                            first_row,
                            value.db_type(),
                            value.max_size(),
                        )?;
                    }
                    first_row = false;
                }
            }
            #[cfg(feature = "arrow")]
            Self::Arrow(batch) => {
                if batch.num_columns() != binds.len() {
                    return Err(Error::wrong_num_positional_binds(
                        binds.len(),
                        batch.num_columns(),
                    ));
                }
                for (column, bind_info) in
                    batch.columns().iter().zip(binds.iter_mut())
                {
                    let db_type = arrow::column_db_type(column)?;
                    let max_size = arrow::column_max_size(column);
                    bind_info
                        .check_and_set_metadata(true, db_type, max_size)?;
                }
            }
        }
        Ok(())
    }

    /// Writes the value at the given row and column index to the buffer.
    pub(crate) fn write_to_buf(
        &self,
        row_index: usize,
        column_index: usize,
        bind_info: &BindInfo,
        buf: &mut WriteBuffer,
    ) {
        let metadata = bind_info.metadata.as_ref().unwrap();
        match self {
            Self::Slice(params) => params[row_index][column_index].to_buf(
                buf,
                metadata.db_type(),
                true,
            ),
            #[cfg(feature = "arrow")]
            Self::Arrow(batch) => {
                let column = batch.column(column_index);
                arrow::write_to_buf(column, row_index, metadata, buf);
            }
        }
    }
}
