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
// cursor.rs
//
// Defines the structure representing cursors.
//-----------------------------------------------------------------------------

use std::collections::VecDeque;
use std::sync::Arc;

#[cfg(feature = "arrow")]
use crate::bind_params::BindParameters;
use crate::client::Client;
use crate::error::Error;
use crate::metadata::Metadata;
use crate::response::Response;
use crate::row::{DbRow, Row};
use crate::statement::CachedStatement;
use crate::statement::StatementHolder;
use crate::statement::StatementOptions;

/// Represents a set of rows retrieved from the database by calling
/// [Connection::query()](`crate::Connection::query`) or
/// [Connection::query_named()](`crate::Connection::query_named`). This struct
/// implements the Iterator trait.
pub struct Cursor {
    statement_holder: StatementHolder,
    column_info: Arc<Vec<Metadata>>,
    rows: VecDeque<DbRow>,
    last_row: Option<DbRow>,
    end_of_fetch: bool,
}

impl Cursor {
    /// Fetches more rows from the database.
    fn get_more_rows(&mut self) -> Result<(), Error> {
        let last_row = self.last_row.take();
        self.set_from_response(self.statement_holder.fetch(last_row)?);
        Ok(())
    }

    /// Sets the state of the cursor from the database response.
    fn set_from_response(&mut self, mut response: Response) {
        self.end_of_fetch = response.is_end_of_fetch();
        if let Some(rows) = response.take_rows() {
            self.rows = rows.into();
        } else {
            self.rows.clear();
        }
    }

    /// Executes the SQL statement and processes the initial response.
    #[cfg(feature = "arrow")]
    pub(crate) fn execute(
        &mut self,
        params: BindParameters,
    ) -> Result<(), Error> {
        let response = self.statement_holder.execute_batch(params)?;
        self.set_from_initial_response(response);
        Ok(())
    }

    /// Creates a new cursor from a database response.
    pub(crate) fn from_resp(
        resp: &mut Response,
        client: &Client,
        is_nested: bool,
        options: &StatementOptions,
    ) -> Result<Option<Self>, Error> {
        let mut statement =
            CachedStatement::create_empty(String::new(), is_nested, options);
        let _length = resp.read_u8()?;
        statement.populate_from_describe_info(client, resp)?;
        statement.set_cursor_id(resp.read_ub2()?);
        let holder = StatementHolder::new(resp.get_client_ref(), statement);
        Ok(Some(Self::new(holder)))
    }

    /// Creates a new cursor.
    pub(crate) fn new(statement_holder: StatementHolder) -> Self {
        let metadata = statement_holder.out_metadata().to_vec();
        Self {
            statement_holder,
            column_info: Arc::new(metadata),
            rows: VecDeque::<DbRow>::new(),
            last_row: None,
            end_of_fetch: false,
        }
    }

    /// Sets the state of the cursor from the initial database response.
    pub(crate) fn set_from_initial_response(&mut self, response: Response) {
        self.rows.clear();
        self.last_row = None;
        self.set_from_response(response);
    }

    /// Returns the columns associated with the cursor. If the cursor does not
    /// reference a query, the vector will be empty.
    pub fn columns(&self) -> &Vec<Metadata> {
        self.statement_holder.out_metadata()
    }

    /// Returns the sql associated with the cursor.
    pub fn sql(&self) -> &str {
        self.statement_holder.sql()
    }
}

impl Iterator for Cursor {
    type Item = Result<Row, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rows.is_empty()
            && !self.end_of_fetch
            && let Err(e) = self.get_more_rows()
        {
            return Some(Err(e));
        }
        if let Some(row) = self.rows.pop_front() {
            if self.rows.is_empty() {
                self.last_row = Some(row.clone());
            }
            Some(Ok(Row::new(&self.column_info, row)))
        } else {
            None
        }
    }
}
