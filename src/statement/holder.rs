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
// holder.rs
//
// Defines the structure holding a statement for a period of time.
//-----------------------------------------------------------------------------

use crate::bind_params::BindParameters;
use crate::client::ClientRef;
use crate::db_value::ToDbValue;
use crate::error::Error;
use crate::messages::ExecuteMessage;
use crate::messages::FetchMessage;
use crate::metadata::Metadata;
use crate::response::Response;
use crate::row::RowData;
use crate::statement::CachedStatement;

#[derive(Clone)]
pub(crate) struct StatementHolder {
    client_ref: ClientRef,
    statement: CachedStatement,
}

impl StatementHolder {
    /// Returns the metadata of the data that is being returned by the
    /// statement. This will be fetch metadata or the metadata of PL/SQL out
    /// variables or DML returning out variables.
    pub(crate) fn out_metadata(&self) -> &Vec<Metadata> {
        self.statement.out_metadata()
    }

    /// Base execution of a statement. At this point binds have been
    /// checked and transformed (if needed) into the sequence required by the
    /// server.
    fn base_execute(
        &mut self,
        params: BindParameters,
    ) -> Result<Response, Error> {
        let mut client = self.client_ref.lock().unwrap();
        let mut message = ExecuteMessage::new(&mut self.statement, params);
        let response = client
            .process_message_with_ref(&mut message, Some(&self.client_ref))?;
        if self.statement.requires_define() {
            self.statement.clear_requires_define();
        }
        Ok(response)
    }

    /// Executes a statement with the given parameters.
    pub(crate) fn execute(
        &mut self,
        params: &[&dyn ToDbValue],
    ) -> Result<Response, Error> {
        let binding = [params];
        let params: BindParameters = binding.as_slice().into();
        self.statement.check_binds(&params)?;
        self.base_execute(params)
    }

    /// Executes a SQL statement against the database multiple times in one
    /// round trip. Statements that require single execution are performed once
    /// first before subsequent iterations are performed as a batch.
    pub fn execute_batch(
        &mut self,
        params: BindParameters,
    ) -> Result<Response, Error> {
        self.statement.check_binds(&params)?;
        if params.num_rows() > 1 && self.statement.requires_single_execute() {
            let mut initial_resp = self.base_execute(params.slice(0, 1))?;
            let mut final_resp =
                self.base_execute(params.slice(1, params.num_rows() - 1))?;
            final_resp.transfer_info(&mut initial_resp);
            Ok(final_resp)
        } else {
            self.base_execute(params)
        }
    }

    /// Executes a statement with the given named parameters.
    pub(crate) fn execute_named(
        &mut self,
        named_params: &[(&str, &dyn ToDbValue)],
    ) -> Result<Response, Error> {
        let checked_params = self.statement.check_named_binds(named_params)?;
        let binding = [checked_params.as_slice()];
        let params: BindParameters = binding.as_slice().into();
        self.statement.check_binds(&params)?;
        self.base_execute(params)
    }

    /// Fetches another set of rows from the database.
    pub(crate) fn fetch(
        &self,
        last_row: Option<RowData>,
    ) -> Result<Response, Error> {
        let mut message = FetchMessage::new(&self.statement, last_row);
        self.client_ref
            .lock()
            .unwrap()
            .process_message_with_ref(&mut message, Some(&self.client_ref))
    }

    /// Creates a new statement holder used for performing the actual
    /// interaction with the database.
    pub(crate) fn new(
        client_ref: ClientRef,
        statement: CachedStatement,
    ) -> StatementHolder {
        StatementHolder {
            client_ref,
            statement,
        }
    }

    /// Returns the SQL associated with the statement that is held.
    pub(crate) fn sql(&self) -> &str {
        self.statement.sql()
    }
}

impl Drop for StatementHolder {
    fn drop(&mut self) {
        let mut client = self.client_ref.lock().unwrap();
        client.return_statement(&self.statement);
    }
}
