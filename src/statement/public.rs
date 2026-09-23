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

use super::CachedStatement;

#[cfg(feature = "arrow")]
use crate::arrow;
use crate::bind_params::BindParameters;
use crate::client::ClientRef;
use crate::cursor::Cursor;
use crate::db_value::ToDbValue;
use crate::error::Error;
use crate::exec_result::ExecBatchResult;
use crate::exec_result::ExecResult;
use crate::metadata::Metadata;
use crate::response::Response;
use crate::row::DbRow;
use crate::row::Row;

/// Represents SQL statements that can be executed with various options.
pub struct Statement {
    client_ref: ClientRef,
    statement: CachedStatement,
}

impl Statement {
    /// Gets the response to a fetch.
    pub(crate) fn fetch(
        &self,
        last_row: Option<DbRow>,
    ) -> Result<Response, Error> {
        self.client_ref.lock()?.fetch(
            &self.statement,
            &self.client_ref,
            last_row,
        )
    }

    /// Gets the response to the execution of a statement. At this point binds
    /// have been checked and transformed (if needed) into the sequence
    /// required by the server.
    pub(crate) fn get_execute_response(
        &mut self,
        params: BindParameters,
        parse_only: bool,
    ) -> Result<Response, Error> {
        self.client_ref.lock()?.execute(
            &mut self.statement,
            &self.client_ref,
            params,
            parse_only,
        )
    }

    /// Creates a new public facing statement from the internal cached
    /// statement.
    pub(crate) fn new(
        client_ref: ClientRef,
        statement: CachedStatement,
    ) -> Self {
        Self {
            client_ref,
            statement,
        }
    }

    /// Returns the bind names used by the statement.
    pub fn bind_names(&self) -> &[String] {
        self.statement.bind_names()
    }

    /// Ensures that the statement is fully parsed by the database. If the
    /// statement has not been fully parsed by the database, a round trip will
    /// be performed to ask the database to parse the statement. Note that DDL
    /// statements will also result in execution.
    pub fn ensure_fully_parsed(&mut self) -> Result<(), Error> {
        if !self.is_fully_parsed() {
            self.get_execute_response(BindParameters::default(), true)?;
        }
        Ok(())
    }

    /// Executes the statement with the given parameters and returns an
    /// ExecResult structure. The statement that is executed may not be a
    /// query.
    pub fn execute(
        &mut self,
        params: &[&dyn ToDbValue],
    ) -> Result<ExecResult, Error> {
        let binding = [params];
        let params: BindParameters = binding.as_slice().into();
        self.statement.check_binds(&params)?;
        let mut response = self.get_execute_response(params, false)?;
        Ok(ExecResult::new(&self.statement, &mut response))
    }

    /// Executes a SQL statement against the database multiple times in one
    /// round trip.
    pub fn execute_batch(
        &mut self,
        params: BindParameters,
    ) -> Result<ExecBatchResult, Error> {
        self.statement.check_binds(&params)?;
        let num_execs = params.num_rows();
        let mut response = self.get_execute_response(params, false)?;
        Ok(ExecBatchResult::new(
            &self.statement,
            num_execs,
            &mut response,
        ))
    }

    /// Executes the statement with the given named parameters and returns an
    /// ExecResult structure. The statement that is executed may not be a
    /// query.
    pub fn execute_named(
        &mut self,
        named_params: &[(&str, &dyn ToDbValue)],
    ) -> Result<ExecResult, Error> {
        let checked_params = self.statement.check_named_binds(named_params)?;
        let binding = [checked_params.as_slice()];
        let params: BindParameters = binding.as_slice().into();
        self.statement.check_binds(&params)?;
        let mut response = self.get_execute_response(params, false)?;
        Ok(ExecResult::new(&self.statement, &mut response))
    }

    /// Returns whether or not the statement is a DDL statement.
    pub fn is_ddl(&self) -> bool {
        self.statement.is_ddl()
    }

    /// Returns whether or not the statement is a DML statement.
    pub fn is_dml(&self) -> bool {
        self.statement.is_dml()
    }

    /// Returns whether or not the statement is a DML returning statement.
    pub fn is_dml_returning(&self) -> bool {
        self.statement.is_dml_returning()
    }

    /// Returns whether or not the statement has been fully parsed by the
    /// database.
    pub fn is_fully_parsed(&self) -> bool {
        self.statement.has_cursor()
    }

    /// Returns whether or not the statement is a PL/SQL statement.
    pub fn is_plsql(&self) -> bool {
        self.statement.is_plsql()
    }

    /// Returns whether or not the statement is a query.
    pub fn is_query(&self) -> bool {
        self.statement.is_query()
    }

    /// Returns the metadata of the data that is returned by the statement.
    /// This includes OUT binds for PL/SQL statements, returned data for DML
    /// RETURNING statements and column data for queries. Note that until the
    /// statement is fully parsed, this information will not be known and an
    /// empty slice will be returned.
    pub fn out_metadata(&self) -> &[Metadata] {
        self.statement.out_metadata()
    }

    /// Executes the statement with the given parameters and returns a Cursor
    /// which can be used to iterate over the rows returned by the query. The
    /// statement that is executed must be a query.
    pub fn query(
        mut self,
        params: &[&dyn ToDbValue],
    ) -> Result<Cursor, Error> {
        let binding = [params];
        let bind_params: BindParameters = binding.as_slice().into();
        self.statement.check_binds(&bind_params)?;
        let response = self.get_execute_response(bind_params, false)?;
        let mut cursor = Cursor::new(self);
        cursor.set_from_initial_response(response);
        Ok(cursor)
    }

    #[cfg(feature = "arrow")]
    /// Performs a query against the database and returns an Arrow RecordBatch
    /// structure containing the data.
    pub fn query_arrow(
        mut self,
        params: BindParameters,
    ) -> Result<arrow_array::RecordBatch, Error> {
        self.statement.check_binds(&params)?;
        let mut client = self.client_ref.lock()?;
        arrow::query_single_batch(
            &mut client,
            &mut self.statement,
            &self.client_ref,
            params,
        )
    }

    /// Executes the statement with the given parameters and returns a Cursor
    /// which can be used to iterate over the rows returned by the query. The
    /// statement that is executed must be a query.
    pub fn query_named(
        mut self,
        named_params: &[(&str, &dyn ToDbValue)],
    ) -> Result<Cursor, Error> {
        let checked_params = self.statement.check_named_binds(named_params)?;
        let binding = [checked_params.as_slice()];
        let params: BindParameters = binding.as_slice().into();
        self.statement.check_binds(&params)?;
        let response = self.get_execute_response(params, false)?;
        let mut cursor = Cursor::new(self);
        cursor.set_from_initial_response(response);
        Ok(cursor)
    }

    /// Executes the statement with the given parameters and returns the first
    /// row supplied by the database. If no rows are found, a NoDataFound error
    /// is returned instead.
    pub fn query_row(self, params: &[&dyn ToDbValue]) -> Result<Row, Error> {
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
        self,
        params: &[(&str, &dyn ToDbValue)],
    ) -> Result<Row, Error> {
        let mut cursor = self.query_named(params)?;
        if let Some(row) = cursor.next() {
            return row;
        }
        Err(Error::no_data_found())
    }

    /// Returns the SQL associated with the statement.
    pub fn sql(&self) -> &str {
        self.statement.sql()
    }
}

impl Drop for Statement {
    fn drop(&mut self) {
        if let Ok(mut client) = self.client_ref.lock() {
            client.return_statement(&self.statement);
        }
    }
}
