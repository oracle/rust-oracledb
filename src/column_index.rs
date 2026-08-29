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
// column_index.rs
//
// Defines the trait for resolving column positions by index or name.
//-----------------------------------------------------------------------------

use crate::{Error, Row};

/// A trait for types that can be used to index columns in a [`Row`].
///
/// Both numeric indices (`usize`) and column names (`&str`) implement this trait.
pub trait ColumnIndex {
    /// Resolves the column position within the given row.
    fn resolve(&self, row: &Row) -> Result<usize, Error>;
}

impl ColumnIndex for usize {
    fn resolve(&self, row: &Row) -> Result<usize, Error> {
        if *self < row.column_values.len() {
            Ok(*self)
        } else {
            Err(Error::invalid_column_index(*self))
        }
    }
}

impl ColumnIndex for &str {
    fn resolve(&self, row: &Row) -> Result<usize, Error> {
        if let Some(metadata) = &row.metadata {
            if let Some(index) = metadata
                .iter()
                .position(|col| col.name().eq_ignore_ascii_case(self))
            {
                return Ok(index);
            }
        }
        Err(Error::invalid_column_name(self))
    }
}