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

pub struct DbRow {
    column_values: Vec<Option<DbValue>>,
}

impl DbRow {
    /// Clones a column in the row. All values except Cursors and Lobs are
    /// cloned. Cursors and Lobs become null in the cloned row because they may
    /// not be cloned.
    fn clone_value(value: &Option<DbValue>) -> Option<DbValue> {
        match value {
            Some(DbValue::Array(v)) => Some(DbValue::Array(v.clone())),
            Some(DbValue::BinaryDouble(v)) => Some(DbValue::BinaryDouble(*v)),
            Some(DbValue::BinaryFloat(v)) => Some(DbValue::BinaryFloat(*v)),
            Some(DbValue::Boolean(v)) => Some(DbValue::Boolean(*v)),
            Some(DbValue::Bytes(v)) => Some(DbValue::Bytes(v.clone())),
            Some(DbValue::IntervalDS(v)) => Some(DbValue::IntervalDS(*v)),
            Some(DbValue::IntervalYM(v)) => Some(DbValue::IntervalYM(*v)),
            Some(DbValue::Json(v)) => Some(DbValue::Json(v.clone())),
            Some(DbValue::Number(v)) => Some(DbValue::Number(*v)),
            Some(DbValue::Rowid(v)) => Some(DbValue::Rowid(v.clone())),
            Some(DbValue::String(v)) => Some(DbValue::String(v.clone())),
            Some(DbValue::Timestamp(v)) => Some(DbValue::Timestamp(*v)),
            Some(DbValue::Vector(v)) => Some(DbValue::Vector(v.clone())),
            _ => None,
        }
    }

    /// Returns the element at the given index, or an error.
    fn get(&self, index: usize) -> Result<&Option<DbValue>, Error> {
        self.column_values
            .get(index)
            .ok_or(Error::invalid_column_index(index))
    }

    /// Returns the element at the given index, or an error. The value is
    /// removed from the row and a null is substituted.
    fn take(&mut self, index: usize) -> Result<Option<DbValue>, Error> {
        let column_value = self
            .column_values
            .get_mut(index)
            .ok_or(Error::invalid_column_index(index))?;
        Ok(column_value.take())
    }

    /// Clones the value at the specified index.
    pub(crate) fn clone_column(&self, index: usize) -> Option<DbValue> {
        Self::clone_value(&self.column_values[index])
    }

    /// Creates a new database value row from a set of column values
    pub(crate) fn new(column_values: Vec<Option<DbValue>>) -> Self {
        Self { column_values }
    }

    /// Tranforms all the elements in the row into the requested type using
    /// owned values.
    pub(crate) fn transform_owned<'a, T>(self) -> Result<Vec<T>, Error>
    where
        T: FromDbValue<'a>,
    {
        let mut array = Vec::<T>::with_capacity(self.column_values.len());
        for value in self.column_values {
            array.push(<T>::from_db_value(ColumnData::Owned(value))?);
        }
        Ok(array)
    }

    /// Tranforms all the elements in the row into the requested type using
    /// references.
    pub(crate) fn transform_ref<'a, T>(&'a self) -> Result<Vec<T>, Error>
    where
        T: FromDbValue<'a>,
    {
        let mut array = Vec::<T>::with_capacity(self.column_values.len());
        for value in &self.column_values {
            array.push(<T>::from_db_value(ColumnData::Borrowed(value))?);
        }
        Ok(array)
    }
}

impl Clone for DbRow {
    fn clone(&self) -> Self {
        DbRow::new(self.column_values.iter().map(Self::clone_value).collect())
    }
}

pub enum ColumnData<'a> {
    Borrowed(&'a Option<DbValue>),
    Owned(Option<DbValue>),
}

pub struct Row {
    column_info: Arc<Vec<Metadata>>,
    column_values: DbRow,
}

impl Row {
    /// Creates a new row from the set of column values.
    pub(crate) fn new(
        column_info: &Arc<Vec<Metadata>>,
        column_values: DbRow,
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
        let value_opt = self.column_values.get(index)?;
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
        let value_opt = self.column_values.get(index)?;
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
        let value_opt = self.column_values.take(index)?;
        <T>::from_db_value(ColumnData::Owned(value_opt))
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
        let value_opt = self.column_values.take(index)?;
        <T>::from_db_value_array(ColumnData::Owned(value_opt))
    }
}
