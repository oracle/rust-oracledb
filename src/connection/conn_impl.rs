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
// conn_impl.rs
//
// Defines the structure that implements connections. This is independent of
// the connection structure in order to allow for connections that are closed
// and for pooling connections.
//-----------------------------------------------------------------------------

use std::time::Duration;
use std::time::Instant;

use crate::bind_params::BindParameters;
use crate::client::{Client, ClientRef};
use crate::config::Config;
use crate::cursor::Cursor;
use crate::db_info::DbInfo;
use crate::db_value::ToDbValue;
use crate::end_user_security_context::EndUserSecurityContext;
use crate::error::Error;
use crate::exec_result::ExecResult;
use crate::messages::CommitMessage;
use crate::messages::PingMessage;
use crate::messages::RollbackMessage;
use crate::ora_version::OracleVersion;
use crate::row::Row;
use crate::statement::Statement;

pub(crate) struct ConnImpl {
    client_ref: ClientRef,
    db_info: DbInfo,
    returned_to_pool: Instant,
}

pub(crate) enum ConnImplStatus {
    Healthy,
    RequiresPing,
    RequiresClose,
}

impl ConnImpl {
    /// Changes the password of the logged on user.
    pub(crate) fn change_password(
        &self,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), Error> {
        let mut client = self.client_ref.lock().unwrap();
        client.change_password(old_password, new_password)
    }

    /// Clears Deep Data Security state from the underlying client/session.
    pub(crate) fn clear_end_user_security_context(&self) -> Result<(), Error> {
        self.client_ref
            .lock()
            .unwrap()
            .clear_end_user_security_context();
        Ok(())
    }

    /// Closes the connection and makes it unsable now instead of when the
    /// connection is dropped.
    pub(crate) fn close(&mut self) -> Result<(), Error> {
        self.clear_end_user_security_context()?;
        self.rollback()?;
        self.client_ref.lock().unwrap().close()
    }

    /// Establishes a connection to the database and returns it.
    pub(crate) fn connect(config: Config) -> Result<ConnImpl, Error> {
        config.validate()?;
        let mut client = Client::new(config);
        let db_info = client.connect()?;
        let client_ref = std::sync::Arc::new(std::sync::Mutex::new(client));
        Ok(ConnImpl {
            client_ref,
            db_info,
            returned_to_pool: Instant::now(),
        })
    }

    /// Returns the current status of the connection.
    pub(crate) fn get_status(
        &self,
        ping_interval_opt: Option<Duration>,
    ) -> ConnImplStatus {
        if self.client_ref.lock().unwrap().requires_close() {
            ConnImplStatus::RequiresClose
        } else if let Some(ping_interval) = ping_interval_opt
            && self.returned_to_pool.elapsed() >= ping_interval
        {
            ConnImplStatus::RequiresPing
        } else {
            ConnImplStatus::Healthy
        }
    }

    /// Sets Deep Data Security state on the underlying client/session.
    pub(crate) fn set_end_user_security_context(
        &self,
        context: EndUserSecurityContext,
    ) -> Result<(), Error> {
        self.client_ref
            .lock()
            .unwrap()
            .set_end_user_security_context(context)
    }

    /// Sets the returned to pool instant which is used in pool management.
    pub(crate) fn set_returned_to_pool(&mut self) {
        self.returned_to_pool = Instant::now();
    }

    /// Commits any pending transactions.
    pub fn commit(&self) -> Result<(), Error> {
        let mut message = CommitMessage::new();
        let mut client = self.client_ref.lock().unwrap();
        client.process_message(&mut message)?;
        Ok(())
    }

    /// Executes a SQL statement against the database.
    pub fn execute(
        &self,
        sql: &str,
        params: &[&dyn ToDbValue],
    ) -> Result<ExecResult, Error> {
        self.statement(sql).execute(params)
    }

    /// Executes a SQL statement against the database multiple times in one
    /// round trip.
    pub fn execute_batch(
        &self,
        sql: &str,
        params: BindParameters,
    ) -> Result<ExecResult, Error> {
        self.statement(sql).execute_batch(params)
    }

    /// Executes a SQL statement against the database using named parameters.
    pub fn execute_named(
        &self,
        sql: &str,
        params: &[(&str, &dyn ToDbValue)],
    ) -> Result<ExecResult, Error> {
        self.statement(sql).execute_named(params)
    }

    /// Returns the call timeout configured on the connection.
    pub fn get_call_timeout(&self) -> Result<Option<Duration>, Error> {
        self.client_ref.lock().unwrap().get_call_timeout()
    }

    /// Returns the domain of the database.
    pub fn get_db_domain(&self) -> &str {
        self.db_info.get_db_domain()
    }

    /// Returns the name of the database.
    pub fn get_db_name(&self) -> &str {
        self.db_info.get_db_name()
    }

    /// Returns the instance name used to connect to the database.
    pub fn get_instance_name(&self) -> &str {
        self.db_info.get_instance_name()
    }

    /// Returns the last warning returned by the database.
    pub fn get_last_warning(&self) -> Option<String> {
        self.client_ref.lock().unwrap().get_last_warning()
    }

    /// Returns the maximum number of bytes allowed to be used in identifiers.
    pub fn get_max_identifier_length(&self) -> usize {
        self.db_info.get_max_identifier_length()
    }

    /// Returns the maximum number of open cursors allowed by the database.
    pub fn get_max_open_cursors(&self) -> usize {
        self.db_info.get_max_open_cursors()
    }

    /// Returns the serial number of the connection to the database.
    pub fn get_serial_num(&self) -> usize {
        self.db_info.get_serial_num()
    }

    /// Returns the version of the database.
    pub fn get_server_version(&self) -> OracleVersion {
        self.db_info.get_server_version()
    }

    /// Returns the service name used to connect to the database.
    pub fn get_service_name(&self) -> &str {
        self.db_info.get_service_name()
    }

    /// Returns the session id of the connection to the database.
    pub fn get_session_id(&self) -> usize {
        self.db_info.get_session_id()
    }

    /// Pings the database.
    pub fn ping(&self) -> Result<(), Error> {
        let mut message = PingMessage::new();
        let mut client = self.client_ref.lock().unwrap();
        client.process_message(&mut message)?;
        Ok(())
    }

    /// Executes a query against the database.
    pub fn query(
        &self,
        sql: &str,
        params: &[&dyn ToDbValue],
    ) -> Result<Cursor, Error> {
        self.statement(sql).query(params)
    }

    #[cfg(feature = "arrow")]
    /// Performs a query against the database and returns an Arrow RecordBatch
    /// structure containing the data.
    pub fn query_arrow(
        &self,
        sql: &str,
        params: BindParameters,
    ) -> Result<arrow_array::RecordBatch, Error> {
        self.statement(sql).query_arrow(params)
    }

    /// Executes a query against the database using named parameters.
    pub fn query_named(
        &self,
        sql: &str,
        params: &[(&str, &dyn ToDbValue)],
    ) -> Result<Cursor, Error> {
        self.statement(sql).query_named(params)
    }

    /// Executes a query against the database.
    pub fn query_row(
        &self,
        sql: &str,
        params: &[&dyn ToDbValue],
    ) -> Result<Row, Error> {
        self.statement(sql)
            .prefetch_rows(1)
            .fetch_array_size(1)
            .query_row(params)
    }

    /// Executes a query against the database using named parameters.
    pub fn query_row_named(
        &self,
        sql: &str,
        params: &[(&str, &dyn ToDbValue)],
    ) -> Result<Row, Error> {
        self.statement(sql)
            .prefetch_rows(1)
            .fetch_array_size(1)
            .query_row_named(params)
    }

    /// Rolls back any pending transactions.
    pub fn rollback(&self) -> Result<(), Error> {
        let mut message = RollbackMessage::new();
        let mut client = self.client_ref.lock().unwrap();
        client.process_message(&mut message)?;
        Ok(())
    }

    /// Sets the call timeout configured on the connection.
    pub fn set_call_timeout(
        &self,
        duration: Option<Duration>,
    ) -> Result<(), Error> {
        self.client_ref.lock().unwrap().set_call_timeout(duration)
    }

    /// Sets the action associated with the connection. This is the same as
    /// calling dbms_application_info.set_action() but without executing a
    /// statement. The value is piggybacked to the database with the next
    /// network round trip.
    pub fn set_pending_action(&self, action: &str) {
        let mut client = self.client_ref.lock().unwrap();
        client.set_pending_action(action);
    }

    /// Sets the client identifier associated with the connection. This is the
    /// same as calling dbms_application_info.set_client_identifier() but
    /// without executing a statement. The value is piggybacked to the database
    /// with the next network round trip.
    pub fn set_pending_client_identifier(&self, client_identifier: &str) {
        let mut client = self.client_ref.lock().unwrap();
        client.set_pending_client_identifier(client_identifier);
    }

    /// Sets the client info associated with the connection. This is the same
    /// as calling dbms_application_info.set_client_info() but without
    /// executing a statement. The value is piggybacked to the database with
    /// the next network round trip.
    pub fn set_pending_client_info(&self, client_info: &str) {
        let mut client = self.client_ref.lock().unwrap();
        client.set_pending_client_info(client_info);
    }

    /// Sets the database operation to be monitored in the database. This is
    /// the same as calling dbms_sql_monitor.begin_operation() but without
    /// executing a statement. The value is piggybacked to the database with
    /// the next network round trip.
    pub fn set_pending_db_op(&self, db_op: &str) {
        let mut client = self.client_ref.lock().unwrap();
        client.set_pending_db_op(db_op);
    }

    /// Sets the module associated with the connection. This is the same as
    /// calling dbms_application_info.set_module() but without executing a
    /// statement. The value is piggybacked to the database with the next
    /// network round trip.
    pub fn set_pending_module(&self, db_op: &str) {
        let mut client = self.client_ref.lock().unwrap();
        client.set_pending_module(db_op);
    }

    /// Creates a Statement structure which can be used to specify
    /// various statement options.
    pub fn statement<'sql>(&self, sql: &'sql str) -> Statement<'sql> {
        Statement::new(&self.client_ref, sql)
    }
}

impl Drop for ConnImpl {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
