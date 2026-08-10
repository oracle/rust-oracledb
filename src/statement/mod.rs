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
// statement.rs
//
// Defines the structure representing a statement.
//-----------------------------------------------------------------------------

mod bind_info;
mod cache;
mod holder;
mod options;
mod public;
mod sql_parser;

use std::collections::HashMap;
use std::collections::HashSet;

use crate::bind_params::BindParameters;
use crate::client::Client;
use crate::constants;
use crate::db_value::ToDbValue;
use crate::error::Error;
use crate::metadata::Metadata;
use crate::response::Response;

#[derive(Clone)]
pub(crate) struct CachedStatement {
    sql: String,
    cursor_id: u16,
    no_prefetch: bool,
    requires_define: bool,
    is_query: bool,
    is_plsql: bool,
    is_ddl: bool,
    is_dml: bool,
    is_returning: bool,
    binds: Vec<BindInfo>,
    bind_names: Vec<String>,
    bind_names_set: HashSet<String>,
    binds_changed: bool,
    out_metadata: Vec<Metadata>,
    cache_slot_num: usize,
    is_nested: bool,
    options: StatementOptions,
}

impl CachedStatement {
    /// Adds a bind variable to the statement.
    fn add_bind(&mut self, name: String) {
        let exists = self.bind_names_set.contains(&name);
        if !self.is_plsql || !exists {
            self.binds.push(BindInfo {
                name: name.clone(),
                is_return_bind: self.is_returning,
                metadata: None,
                bind_direction: constants::TTC_BIND_DIR_INPUT,
            });
            if !exists {
                self.bind_names.push(name.clone());
                self.bind_names_set.insert(name);
            }
        }
    }

    /// Determines the statement type.
    fn determine_statement_type(&mut self, keyword: &str) {
        match keyword.to_uppercase().as_str() {
            "DECLARE" | "BEGIN" | "CALL" => {
                self.is_plsql = true;
            }
            "SELECT" | "WITH" => {
                self.is_query = true;
            }
            "INSERT" | "UPDATE" | "DELETE" | "MERGE" => {
                self.is_dml = true;
            }
            "CREATE" | "ALTER" | "DROP" | "GRANT" | "REVOKE" | "ANALYZE"
            | "AUDIT" | "COMMENT" | "TRUNCATE" => {
                self.is_ddl = true;
            }
            _ => {}
        }
    }

    /// Returns the binds defined for the statement.
    pub(crate) fn binds(&self) -> &Vec<BindInfo> {
        &self.binds
    }

    /// Returns a copy of the names of the binds defined for the statement.
    pub(crate) fn bind_names(&self) -> Vec<String> {
        self.bind_names.clone()
    }

    /// Returns a boolean indicating if binds have changed since the last
    /// time the statement was executed.
    pub(crate) fn binds_changed(&self) -> bool {
        self.binds_changed
    }

    /// Returns the cache slot number used. A value of zero indicates that the
    /// statement is not actually cached.
    pub(crate) fn cache_slot_num(&self) -> usize {
        self.cache_slot_num
    }

    /// Checks that the supplied parameters satisfy the binds defined by the
    /// statement.
    pub(crate) fn check_binds(
        &mut self,
        params: &BindParameters,
    ) -> Result<(), Error> {
        params.validate(&mut self.binds)?;
        if !self.is_query {
            self.out_metadata.clear();
        }
        for bind_info in self.binds.iter() {
            if bind_info.is_output_bind() {
                self.out_metadata.push(bind_info.metadata.clone().unwrap());
            }
        }
        Ok(())
    }

    /// Checks that the supplied named parameters satisfy the binds defined by
    /// the statement.
    pub(crate) fn check_named_binds<'a>(
        &mut self,
        params: &[(&str, &'a dyn ToDbValue)],
    ) -> Result<Vec<&'a dyn ToDbValue>, Error> {
        // first, build a hash map containing the binds and their values since
        // the order in which they are supplied may differ from the order in
        // which they are supplied in the statement; in addition, PL/SQL
        // statements only allow one bind per name whereas SQL statements allow
        // multiple binds per name and the bind must be repeated when being
        // sent to the server
        let mut bind_map: HashMap<String, &dyn ToDbValue> = HashMap::new();
        for (name, value) in params.iter() {
            let normalized_name =
                if name.starts_with('"') && name.ends_with('"') {
                    name[1..name.len() - 1].to_string()
                } else {
                    name.to_uppercase()
                };
            bind_map.insert(normalized_name, *value);
        }

        // second, check that the number of binds matches, and if not, raise an
        // appropriate exception
        if bind_map.len() != self.bind_names.len() {
            for name in bind_map.keys() {
                if !self.bind_names_set.contains(name) {
                    return Err(Error::invalid_bind_name(name));
                }
            }
        }

        // finally, scan the binds and ensure that a named bind has been
        // supplied for each of them
        let mut checked_binds: Vec<&dyn ToDbValue> = Vec::new();
        for bind_info in self.binds.iter_mut() {
            if let Some(param) = bind_map.get(&bind_info.name) {
                checked_binds.push(*param);
            } else {
                return Err(Error::missing_bind_value(&bind_info.name));
            }
        }

        Ok(checked_binds)
    }

    /// Clears the database cursor id.
    pub(crate) fn clear_cursor(&mut self) {
        self.cursor_id = 0;
    }

    /// Clears the flag indicating that a define is required.
    pub(crate) fn clear_requires_define(&mut self) {
        self.requires_define = false;
    }

    /// Creates a clone of the statement with the specified options.
    pub(crate) fn clone_with_options(
        &self,
        options: &StatementOptions,
    ) -> Self {
        let mut cloned = self.clone();
        cloned.options = options.clone();
        cloned
    }

    /// Creates a new empty statement with the specified SQL and options.
    pub(crate) fn create_empty(
        sql: String,
        is_nested: bool,
        options: &StatementOptions,
    ) -> Self {
        let is_query = sql.is_empty();
        Self {
            sql,
            cursor_id: 0,
            no_prefetch: false,
            requires_define: false,
            is_query,
            is_plsql: false,
            is_ddl: false,
            is_dml: false,
            is_returning: false,
            binds: Vec::new(),
            bind_names: Vec::new(),
            bind_names_set: HashSet::new(),
            binds_changed: false,
            out_metadata: Vec::new(),
            cache_slot_num: 0,
            is_nested,
            options: options.clone(),
        }
    }

    /// Returns the id of the database cursor.
    pub(crate) fn cursor_id(&self) -> u16 {
        self.cursor_id
    }

    /// Returns whether or not the statement has any bind variables.
    pub(crate) fn has_binds(&self) -> bool {
        !self.binds.is_empty()
    }

    /// Returns whether or not a database cursor is associated with the
    /// statement.
    pub(crate) fn has_cursor(&self) -> bool {
        self.cursor_id != 0
    }

    /// Returns whether or not the statement has input binds.
    pub(crate) fn has_input_binds(&self) -> bool {
        for bind_info in &self.binds {
            if bind_info.is_input_bind() {
                return true;
            }
        }
        false
    }

    /// Returns whether or not the statement is in the statement cache.
    pub(crate) fn is_cached(&self) -> bool {
        self.cache_slot_num != 0
    }

    /// Returns whether or not the statement is a DDL statement.
    pub(crate) fn is_ddl(&self) -> bool {
        self.is_ddl
    }

    /// Returns whether or not the statement is a nested statement.
    pub(crate) fn is_nested(&self) -> bool {
        self.is_nested
    }

    /// Returns whether or not the statement is a PL/SQL statement.
    pub(crate) fn is_plsql(&self) -> bool {
        self.is_plsql
    }

    /// Returns whether or not the statement is a query.
    pub(crate) fn is_query(&self) -> bool {
        self.is_query
    }

    /// Creates a new statement from the provided SQL.
    pub(crate) fn new(
        sql: &str,
        options: &StatementOptions,
    ) -> Result<Self, Error> {
        let mut statement = Self::create_empty(sql.into(), false, options);
        sql_parser::SqlParser::new(sql).parse(&mut statement)?;
        Ok(statement)
    }

    /// Returns whether prefetching has been disabled.
    pub(crate) fn no_prefetch(&self) -> bool {
        self.no_prefetch
    }

    /// Returns the options associated with the statement.
    pub(crate) fn options(&self) -> &StatementOptions {
        &self.options
    }

    /// Returns a reference to the vector of metadata for out variables.
    pub(crate) fn out_metadata(&self) -> &Vec<Metadata> {
        &self.out_metadata
    }

    /// Populates statement metadata from the describe info returned by the
    /// database.
    pub(crate) fn populate_from_describe_info(
        &mut self,
        client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        resp.read_ub4()?; // max row size
        let num_columns = resp.read_ub4()?;
        if num_columns > 0 {
            resp.read_u8()?;
        }
        self.out_metadata.clear();
        for _ in 0..num_columns {
            let mut metadata = Metadata::from_response(resp, client)?;
            if metadata.requires_define() {
                self.requires_define = true;
                self.no_prefetch = true;
                if !self.options.fetch_lobs() {
                    metadata = metadata.define_metadata();
                }
            }
            self.out_metadata.push(metadata);
        }
        let _current_date = resp.read_bytes_with_double_length()?;
        let _dcbflag = resp.read_ub4()?;
        let _dcbmdbz = resp.read_ub4()?;
        let _dcbmnpr = resp.read_ub4()?;
        let _dcbmxpr = resp.read_ub4()?;
        let _dcbqcky = resp.read_bytes_with_double_length()?;
        Ok(())
    }

    /// Returns whether or not the statement requires a define.
    pub(crate) fn requires_define(&self) -> bool {
        self.requires_define
    }

    /// Returns a boolean indicating if the statement requires a single execute
    /// in order to be processed correctly by the server. If a PL/SQL block has
    /// not been executed before, the determination of input/output binds has
    /// not been completed and so a single execution is required in order to
    /// complete that determination.
    pub(crate) fn requires_single_execute(&self) -> bool {
        self.is_plsql && (self.cursor_id == 0 || self.binds_changed)
    }

    /// Sets the bind directions from the response from the database.
    pub(crate) fn set_bind_directions(
        &mut self,
        resp: &mut Response,
    ) -> Result<(), Error> {
        self.out_metadata.clear();
        for bind_info in self.binds.iter_mut() {
            bind_info.bind_direction = resp.read_u8()?;
            if bind_info.is_output_bind() {
                let metadata = bind_info.metadata.as_ref().unwrap();
                self.out_metadata.push(metadata.clone());
            }
        }
        Ok(())
    }

    /// Sets the cache slot number for the statement.
    pub(crate) fn set_cache_slot_num(&mut self, slot_num: usize) {
        self.cache_slot_num = slot_num;
    }

    /// Sets the database cursor id associated with the statement.
    pub(crate) fn set_cursor_id(&mut self, cursor_id: u16) {
        self.cursor_id = cursor_id;
    }

    /// Returns the SQL associated with the statement.
    pub(crate) fn sql(&self) -> &str {
        &self.sql
    }

    /// Returns the length of the SQL associated with the statement in the
    /// format required by the database.
    pub(crate) fn sql_len(&self) -> u32 {
        self.sql.len().try_into().unwrap()
    }
}

pub(crate) use bind_info::BindInfo;
pub(crate) use cache::StatementCache;
pub(crate) use holder::StatementHolder;
pub(crate) use options::StatementOptions;
pub use public::Statement;
