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

use crate::cursor::Cursor;
use crate::db_value::{DbValue, FromDbValue};
use crate::error::Error;

pub(crate) type RowData = Vec<Option<DbValue>>;

#[derive(Clone)]
pub struct Row {
    column_values: RowData,
}

impl Row {
    /// Creates a new row from the set of column values.
    pub(crate) fn new(column_values: RowData) -> Row {
        Row { column_values }
    }

    /// Returns the value at the given column index.
    pub fn get<T>(&self, index: usize) -> Result<T, Error>
    where
        T: FromDbValue,
    {
        if index < self.column_values.len() {
            <T>::from_db_value(&self.column_values[index])
        } else {
            Err(Error::invalid_column_index(index))
        }
    }

    /// Returns a cursor from the given column index. Ownership is transferred
    /// from the row to the caller.
    pub fn get_cursor(&mut self, index: usize) -> Result<Cursor, Error> {
        if index < self.column_values.len() {
            if let Some(db_value) = &mut self.column_values[index] {
                match db_value {
                    DbValue::Cursor(cursor_opt) => {
                        Ok(cursor_opt.take().ok_or(Error::value_was_null())?)
                    }
                    _ => Err(Error::unsupported_conversion(
                        db_value.type_name(),
                        "cursor",
                    )),
                }
            } else {
                Err(Error::value_was_null())
            }
        } else {
            Err(Error::invalid_column_index(index))
        }
    }
}
