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
// public.rs
//
// Defines the structure representing a statement and various options used
// publicly.
//-----------------------------------------------------------------------------

#[cfg(feature = "arrow")]
use crate::arrow;
use crate::bind_params::BindParameters;
use crate::client::ClientRef;
use crate::cursor::Cursor;
use crate::db_value::ToDbValue;
use crate::error::Error;
use crate::exec_result::ExecResult;
use crate::row::Row;
use crate::statement::StatementHolder;
use crate::statement::StatementOptions;

/// Represents SQL statements that can be executed with various options.
pub struct Statement<'sql> {
    client_ref: ClientRef,
    sql: &'sql str,
    options: StatementOptions,
    cache_statement: bool,
}

impl<'sql> Statement<'sql> {
    fn holder(&self) -> Result<StatementHolder, Error> {
        let mut client = self.client_ref.lock().unwrap();
        let statement = client.get_statement(
            self.sql,
            self.cache_statement,
            &self.options,
        )?;
        Ok(StatementHolder::new(self.client_ref.clone(), statement))
    }

    pub(crate) fn new(client_ref: &ClientRef, sql: &'sql str) -> Self {
        Self {
            client_ref: client_ref.clone(),
            sql,
            options: StatementOptions::new(),
            cache_statement: true,
        }
    }

    /// Returns a vector of the bind names used by the statement.
    pub fn bind_names(&self) -> Result<Vec<String>, Error> {
        let mut client = self.client_ref.lock().unwrap();
        let statement = client.get_statement(
            self.sql,
            self.cache_statement,
            &self.options,
        )?;
        let names = statement.bind_names();
        client.return_statement(&statement);
        Ok(names)
    }

    /// Executes the statement with the given parameters and returns an
    /// ExecResult structure. The statement that is executed may not be a
    /// query.
    pub fn execute(
        &self,
        params: &[&dyn ToDbValue],
    ) -> Result<ExecResult, Error> {
        let mut holder = self.holder()?;
        let mut response = holder.execute(params)?;
        Ok(ExecResult::new(&mut response))
    }

    /// Executes a SQL statement against the database multiple times in one
    /// round trip.
    pub fn execute_batch(
        &self,
        params: BindParameters,
    ) -> Result<ExecResult, Error> {
        let mut holder = self.holder()?;
        let mut response = holder.execute_batch(params)?;
        Ok(ExecResult::new(&mut response))
    }

    /// Executes the statement with the given named parameters and returns an
    /// ExecResult structure. The statement that is executed may not be a
    /// query.
    pub fn execute_named(
        &self,
        params: &[(&str, &dyn ToDbValue)],
    ) -> Result<ExecResult, Error> {
        let mut holder = self.holder()?;
        let mut response = holder.execute_named(params)?;
        Ok(ExecResult::new(&mut response))
    }

    /// Specifies that this statement should not be cached.
    pub fn exclude_from_cache(&mut self) -> &mut Self {
        self.cache_statement = false;
        self
    }

    /// Specifies the number of rows that should be fetched at a time from the
    /// database.
    pub fn fetch_array_size(&mut self, value: u32) -> &mut Self {
        self.options.set_fetch_array_size(value);
        self
    }

    /// Specifies that LOB values should be fetched as LOB locators.
    pub fn fetch_lobs(&mut self) -> &mut Self {
        self.options.set_fetch_lobs();
        self
    }

    /// Specifies the number of rows that should be fetched when the statement
    /// is executed.
    pub fn prefetch_rows(&mut self, value: u32) -> &mut Self {
        self.options.set_prefetch_rows(value);
        self
    }

    /// Executes the statement with the given parameters and returns a Cursor
    /// which can be used to iterate over the rows returned by the query. The
    /// statement that is executed must be a query.
    pub fn query(&self, params: &[&dyn ToDbValue]) -> Result<Cursor, Error> {
        let mut holder = self.holder()?;
        let response = holder.execute(params)?;
        let mut cursor = Cursor::new(holder);
        cursor.set_from_initial_response(response);
        Ok(cursor)
    }

    #[cfg(feature = "arrow")]
    /// Performs a query against the database and returns an Arrow RecordBatch
    /// structure containing the data.
    pub fn query_arrow(
        &self,
        params: BindParameters,
    ) -> Result<arrow_array::RecordBatch, Error> {
        arrow::query_single_batch(self.holder()?, params)
    }

    /// Executes the statement with the given parameters and returns a Cursor
    /// which can be used to iterate over the rows returned by the query. The
    /// statement that is executed must be a query.
    pub fn query_named(
        &self,
        params: &[(&str, &dyn ToDbValue)],
    ) -> Result<Cursor, Error> {
        let mut holder = self.holder()?;
        let response = holder.execute_named(params)?;
        let mut cursor = Cursor::new(holder);
        cursor.set_from_initial_response(response);
        Ok(cursor)
    }

    /// Executes the statement with the given parameters and returns the first
    /// row supplied by the database. If no rows are found, a NoDataFound error
    /// is returned instead.
    pub fn query_row(&self, params: &[&dyn ToDbValue]) -> Result<Row, Error> {
        let mut cursor = self.query(params)?;
        if let Some(row) = cursor.next() {
            return row;
        }
        Err(Error::no_data_found())
    }

    /// Executes the statement with the given parameters and returns the first
    /// row supplied by the database. If no rows are found, a NoDataFound error
    /// is returned instead.
    pub fn query_row_named(
        &self,
        params: &[(&str, &dyn ToDbValue)],
    ) -> Result<Row, Error> {
        let mut cursor = self.query_named(params)?;
        if let Some(row) = cursor.next() {
            return row;
        }
        Err(Error::no_data_found())
    }
}
