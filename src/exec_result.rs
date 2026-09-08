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
// exec_result.rs
//
// Defines the structures representing execution results.
//-----------------------------------------------------------------------------

use std::sync::Arc;

use crate::metadata::Metadata;
use crate::response::Response;
use crate::row::{DbRow, Row};
use crate::statement::CachedStatement;

/// Represents the result returned by the database when calling
/// [Connection::execute()](`crate::Connection::execute()`),
/// [Connection::execute_named()](`crate::Connection::execute_named()`).
pub struct ExecResult {
    column_info: Arc<Vec<Metadata>>,
    out_bind_data: Option<DbRow>,
    returned_data: Option<Vec<DbRow>>,
    rows_affected: u64,
}

impl ExecResult {
    pub(crate) fn new(
        statement: &CachedStatement,
        resp: &mut Response,
    ) -> Self {
        let mut out_bind_data: Option<DbRow> = None;
        let mut returned_data: Option<Vec<DbRow>> = None;
        if let Some(rows) = resp.take_rows()
            && !rows.is_empty()
        {
            let row = rows.into_iter().next().unwrap();
            if statement.is_plsql() {
                out_bind_data = Some(row)
            } else if statement.is_dml_returning() {
                returned_data = Some(row.transpose_arrays())
            }
        }
        Self {
            column_info: Arc::new(statement.out_metadata().to_vec()),
            out_bind_data,
            returned_data,
            rows_affected: resp.get_rowcount(),
        }
    }

    /// Returns the number of rows affected by the execution of the statement.
    pub fn rows_affected(&self) -> u64 {
        self.rows_affected
    }

    /// Returns data returned by the database as OUT variables for PL/SQL. This
    /// transfers ownership of the data to the caller.
    pub fn out_bind_data(&mut self) -> Row {
        if let Some(row) = self.out_bind_data.take() {
            Row::new(&self.column_info, row)
        } else {
            Row::new_empty()
        }
    }

    /// Returns data returned by the database for DML RETURNING statements.
    /// This transfers ownership of the data to the caller.
    pub fn returned_data(&mut self) -> Vec<Row> {
        if let Some(rows) = self.returned_data.take() {
            rows.into_iter()
                .map(|r| Row::new(&self.column_info, r))
                .collect()
        } else {
            Vec::new()
        }
    }
}

/// Represents the result returned by the database when calling
/// [Connection::execute_batch()](`crate::Connection::execute_batch()`).
pub struct ExecBatchResult {
    column_info: Arc<Vec<Metadata>>,
    out_bind_data: Option<Vec<DbRow>>,
    returned_data: Option<Vec<Vec<DbRow>>>,
    num_execs: usize,
    rows_affected: u64,
}

impl ExecBatchResult {
    pub(crate) fn new(
        statement: &CachedStatement,
        num_execs: usize,
        resp: &mut Response,
    ) -> Self {
        let mut out_bind_data: Option<Vec<DbRow>> = None;
        let mut returned_data: Option<Vec<Vec<DbRow>>> = None;
        if let Some(rows) = resp.take_rows()
            && !rows.is_empty()
        {
            if statement.is_plsql() {
                out_bind_data = Some(rows);
            } else if statement.is_dml_returning() {
                returned_data = Some(
                    rows.into_iter().map(|r| r.transpose_arrays()).collect(),
                );
            }
        }
        Self {
            column_info: Arc::new(statement.out_metadata().to_vec()),
            out_bind_data,
            returned_data,
            num_execs,
            rows_affected: resp.get_rowcount(),
        }
    }

    /// Returns the number of rows affected by the execution of the statement.
    pub fn rows_affected(&self) -> u64 {
        self.rows_affected
    }

    /// Returns data returned by the database as OUT variables in PL/SQL. This
    /// transfers ownership of the returned data to the caller.
    pub fn out_bind_data(&mut self) -> Vec<Row> {
        if let Some(rows) = self.out_bind_data.take() {
            rows.into_iter()
                .map(|r| Row::new(&self.column_info, r))
                .collect()
        } else {
            std::iter::repeat_with(Row::new_empty)
                .take(self.num_execs)
                .collect()
        }
    }

    /// Returns data returned by the database for DML RETURNING statements.
    /// This transfers ownership of the data to the caller.
    pub fn returned_data(&mut self) -> Vec<Vec<Row>> {
        if let Some(rows_of_rows) = self.returned_data.take() {
            rows_of_rows
                .into_iter()
                .map(|rows| {
                    rows.into_iter()
                        .map(|r| Row::new(&self.column_info, r))
                        .collect()
                })
                .collect()
        } else {
            std::iter::repeat_with(Vec::<Row>::new)
                .take(self.num_execs)
                .collect()
        }
    }
}
