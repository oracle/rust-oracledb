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
// row.rs
//
// Defines the structure containing row data.
//-----------------------------------------------------------------------------

use std::sync::Arc;

use crate::column_index::ColumnIndex;
use crate::db_value::{DbValue, FromDbValue};
use crate::error::Error;
use crate::metadata::Metadata;

pub(crate) type RowData = Vec<Option<DbValue>>;

pub enum ColumnData<'a> {
    Borrowed(&'a Option<DbValue>),
    Owned(Option<DbValue>),
}

pub struct Row {
    column_info: Arc<Vec<Metadata>>,
    column_values: RowData,
}

impl Row {
    /// Creates a new row from the set of column values.
    pub(crate) fn new(
        column_info: &Arc<Vec<Metadata>>,
        column_values: RowData,
    ) -> Self {
        Self {
            column_info: Arc::clone(column_info),
            column_values,
        }
    }

    /// Returns the value at the given column index, converted to the requested
    /// type. If a reference type is supplied, a reference is provided to the
    /// internal data, if possible. If an owned type is supplied, a copy of
    /// the internal data is made.
    pub fn get<'a, T>(
        &'a self,
        col_index: impl ColumnIndex,
    ) -> Result<T, Error>
    where
        T: FromDbValue<'a>,
    {
        let index = col_index.resolve(&self.column_info)?;
        let value_opt = self
            .column_values
            .get(index)
            .ok_or(Error::invalid_column_index(index))?;
        <T>::from_db_value(ColumnData::Borrowed(value_opt))
    }

    /// Returns the array at the given column index as a vector.
    pub fn get_array<'a, T>(
        &'a self,
        col_index: impl ColumnIndex,
    ) -> Result<Vec<T>, Error>
    where
        T: FromDbValue<'a>,
    {
        let index = col_index.resolve(&self.column_info)?;
        let value_opt = self
            .column_values
            .get(index)
            .ok_or(Error::invalid_column_index(index))?;
        <T>::from_db_value_array(ColumnData::Borrowed(value_opt))
    }

    /// Returns the value at the given column index, converted to the requested
    /// type. Ownership of the data that was stored in the row at the given
    /// column index is transferred to the caller. If this is attempted with a
    /// reference type, an error will take place.
    pub fn take<'a, T>(
        &'a mut self,
        col_index: impl ColumnIndex,
    ) -> Result<T, Error>
    where
        T: FromDbValue<'a>,
    {
        let index = col_index.resolve(&self.column_info)?;
        let value_opt = self
            .column_values
            .get_mut(index)
            .ok_or(Error::invalid_column_index(index))?;
        <T>::from_db_value(ColumnData::Owned(value_opt.take()))
    }

    /// Returns the array at the given column index as a vector.
    pub fn take_array<'a, T>(
        &'a mut self,
        col_index: impl ColumnIndex,
    ) -> Result<Vec<T>, Error>
    where
        T: FromDbValue<'a>,
    {
        let index = col_index.resolve(&self.column_info)?;
        let value_opt = self
            .column_values
            .get_mut(index)
            .ok_or(Error::invalid_column_index(index))?;
        <T>::from_db_value_array(ColumnData::Owned(value_opt.take()))
    }
}
