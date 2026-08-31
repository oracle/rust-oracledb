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
// Defines the trait for resolving column positions by index or name, allowing
// Row::get() to accept both usize and &str.
//-----------------------------------------------------------------------------

use crate::error::Error;
use crate::metadata::Metadata;

/// Trait for types which can be used to index columns in a [Row](crate::Row)
pub trait ColumnIndex {
    fn resolve(&self, column_info: &[Metadata]) -> Result<usize, Error>;
}

impl ColumnIndex for usize {
    fn resolve(&self, _column_info: &[Metadata]) -> Result<usize, Error> {
        Ok(*self)
    }
}

impl ColumnIndex for &str {
    fn resolve(&self, column_info: &[Metadata]) -> Result<usize, Error> {
        for (idx, info) in column_info.iter().enumerate() {
            if info.name().eq_ignore_ascii_case(self) {
                return Ok(idx);
            }
        }
        Err(Error::invalid_column_name(self))
    }
}
