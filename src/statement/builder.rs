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
// builder.rs
//
// Builds a statement from SQL statement text and various options.
//-----------------------------------------------------------------------------

use super::Statement;
use super::StatementOptions;

use crate::client::ClientRef;
use crate::error::Error;

pub struct StatementBuilder<'sql> {
    client_ref: ClientRef,
    sql: &'sql str,
    options: StatementOptions,
    cache_statement: bool,
}

impl<'sql> StatementBuilder<'sql> {
    /// Creates a new builder with default options.
    pub(crate) fn new(client_ref: &ClientRef, sql: &'sql str) -> Self {
        Self {
            client_ref: client_ref.clone(),
            sql,
            options: StatementOptions::new(),
            cache_statement: true,
        }
    }

    /// Builds the statement from the SQL and the options.
    pub fn build(self) -> Result<Statement, Error> {
        let statement = {
            self.client_ref.lock()?.get_statement(
                self.sql,
                self.cache_statement,
                &self.options,
            )?
        };
        Ok(Statement::new(self.client_ref, statement))
    }

    /// Specifies that this statement should not be cached.
    pub fn exclude_from_cache(mut self) -> Self {
        self.cache_statement = false;
        self
    }

    /// Specifies the number of rows that should be fetched at a time from the
    /// database.
    pub fn fetch_array_size(mut self, value: u32) -> Self {
        self.options.set_fetch_array_size(value);
        self
    }

    /// Specifies that LOB values should be fetched as LOB locators.
    pub fn fetch_lobs(mut self) -> Self {
        self.options.set_fetch_lobs();
        self
    }

    /// Specifies the number of rows that should be fetched when the statement
    /// is executed.
    pub fn prefetch_rows(mut self, value: u32) -> Self {
        self.options.set_prefetch_rows(value);
        self
    }
}
