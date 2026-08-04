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
// mod.rs
//
// Submodule for connections.
//-----------------------------------------------------------------------------

mod conn_impl;

pub(crate) use conn_impl::ConnImpl;
pub(crate) use conn_impl::ConnImplStatus;

use crate::bind_params::BindParameters;
use crate::config::Config;
use crate::cursor::Cursor;
use crate::db_value::ToDbValue;
use crate::end_user_security_context::EndUserSecurityContext;
use crate::error::Error;
use crate::exec_result::ExecResult;
use crate::ora_version::OracleVersion;
use crate::pool::PoolContentsRef;
use crate::row::Row;
use crate::statement::Statement;

/// Represents a connection to the database. This can be either a standalone
/// connection created by calling [connect()](`crate::connect`) or a pooled
/// connection created by calling [Pool::acquire()](`crate::Pool::acquire`)
pub struct Connection {
    conn_impl: Option<ConnImpl>,
    pool_contents_ref: Option<PoolContentsRef>,
}

impl Connection {
    /// Returns an immutable reference to the implementation object or an error
    /// if the implementation object has been closed.
    fn get_impl(&self) -> Result<&ConnImpl, Error> {
        if let Some(conn_impl) = &self.conn_impl {
            Ok(conn_impl)
        } else {
            Err(Error::not_connected())
        }
    }

    /// Closes the connection and makes it unsable now instead of when the
    /// connection is dropped.
    pub fn close(&mut self) -> Result<(), Error> {
        if let Some(pool_contents_ref) = self.pool_contents_ref.take() {
            let mut pool_contents = pool_contents_ref.lock().unwrap();
            let mut conn_impl = self.conn_impl.take().unwrap();
            conn_impl.clear_end_user_security_context()?;
            conn_impl.set_returned_to_pool();
            pool_contents.return_connection(conn_impl);
            Ok(())
        } else if let Some(mut conn_impl) = self.conn_impl.take() {
            conn_impl.close()
        } else {
            Err(Error::not_connected())
        }
    }

    /// Establishes a connection to the database and returns it.
    pub(crate) fn connect(config: Config) -> Result<Connection, Error> {
        let conn_impl = ConnImpl::connect(config)?;
        Ok(Connection {
            conn_impl: Some(conn_impl),
            pool_contents_ref: None,
        })
    }

    /// Creates a connection from a pooled connection.
    pub(crate) fn create_pooled(
        conn_impl: ConnImpl,
        pool_contents_ref: &PoolContentsRef,
    ) -> Connection {
        Connection {
            conn_impl: Some(conn_impl),
            pool_contents_ref: Some(pool_contents_ref.clone()),
        }
    }

    /// Returns the "call timeout" value currently in effect by the connection
    /// or an error if the connection is closed. A value of None means that
    /// there is no limit to the amount of time the client will wait for a
    /// database response and is the initial default value.
    pub fn call_timeout(&self) -> Result<Option<std::time::Duration>, Error> {
        self.get_impl()?.get_call_timeout()
    }

    /// Changes the password of the logged on user.
    pub fn change_password(
        &self,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), Error> {
        self.get_impl()?.change_password(old_password, new_password)
    }

    /// Clears the Deep Data Security context from this connection.
    pub fn clear_end_user_security_context(&self) -> Result<(), Error> {
        self.get_impl()?.clear_end_user_security_context()
    }

    /// Commits any pending transactions.
    pub fn commit(&self) -> Result<(), Error> {
        self.get_impl()?.commit()
    }

    /// Returns the domain of the database.
    pub fn db_domain(&self) -> Result<&str, Error> {
        Ok(self.get_impl()?.get_db_domain())
    }

    /// Returns the name of the database.
    pub fn db_name(&self) -> Result<&str, Error> {
        Ok(self.get_impl()?.get_db_name())
    }

    /// Executes a SQL statement against the database.
    pub fn execute(
        &self,
        sql: &str,
        params: &[&dyn ToDbValue],
    ) -> Result<ExecResult, Error> {
        self.get_impl()?.execute(sql, params)
    }

    /// Executes a SQL statement against the database multiple times in one
    /// round trip.
    pub fn execute_batch<'a>(
        &self,
        sql: &str,
        params: impl Into<BindParameters<'a>>,
    ) -> Result<ExecResult, Error> {
        self.get_impl()?.execute_batch(sql, params.into())
    }

    /// Executes a SQL statement against the database using named parameters.
    pub fn execute_named(
        &self,
        sql: &str,
        params: &[(&str, &dyn ToDbValue)],
    ) -> Result<ExecResult, Error> {
        self.get_impl()?.execute_named(sql, params)
    }

    /// Returns the instance name used to connect to the database.
    pub fn instance_name(&self) -> Result<&str, Error> {
        Ok(self.get_impl()?.get_instance_name())
    }

    /// Returns the last warning that was returned by the database.
    pub fn last_warning(&self) -> Result<Option<String>, Error> {
        Ok(self.get_impl()?.get_last_warning())
    }

    /// Returns the maximum number of bytes allowed to be used in identifiers.
    pub fn max_identifier_length(&self) -> Result<usize, Error> {
        Ok(self.get_impl()?.get_max_identifier_length())
    }

    /// Returns the maximum number of open cursors allowed by the database.
    pub fn max_open_cursors(&self) -> Result<usize, Error> {
        Ok(self.get_impl()?.get_max_open_cursors())
    }

    /// Pings the database.
    pub fn ping(&self) -> Result<(), Error> {
        self.get_impl()?.ping()
    }

    /// Executes a query against the database.
    pub fn query(
        &self,
        sql: &str,
        params: &[&dyn ToDbValue],
    ) -> Result<Cursor, Error> {
        self.get_impl()?.query(sql, params)
    }

    #[cfg(feature = "arrow")]
    /// Executes a query against the database and returns a single Arrow
    /// RecordBatch containing the results of the query.
    pub fn query_arrow<'a>(
        &self,
        sql: &str,
        params: impl Into<BindParameters<'a>>,
    ) -> Result<arrow_array::RecordBatch, Error> {
        self.get_impl()?.query_arrow(sql, params.into())
    }

    /// Executes a query against the database using named parameters.
    pub fn query_named(
        &self,
        sql: &str,
        params: &[(&str, &dyn ToDbValue)],
    ) -> Result<Cursor, Error> {
        self.get_impl()?.query_named(sql, params)
    }

    /// Executes a query against the database.
    pub fn query_row(
        &self,
        sql: &str,
        params: &[&dyn ToDbValue],
    ) -> Result<Row, Error> {
        self.get_impl()?.query_row(sql, params)
    }

    /// Executes a query against the database using named parameters.
    pub fn query_row_named(
        &self,
        sql: &str,
        params: &[(&str, &dyn ToDbValue)],
    ) -> Result<Row, Error> {
        self.get_impl()?.query_row_named(sql, params)
    }

    /// Rolls back any pending transactions.
    pub fn rollback(&self) -> Result<(), Error> {
        self.get_impl()?.rollback()
    }

    /// Returns the serial number of the connection to the database.
    pub fn serial_num(&self) -> Result<usize, Error> {
        Ok(self.get_impl()?.get_serial_num())
    }

    /// Returns the service name used to connect to the database.
    pub fn service_name(&self) -> Result<&str, Error> {
        Ok(self.get_impl()?.get_service_name())
    }

    /// Returns the session id of the connection to the database.
    pub fn session_id(&self) -> Result<usize, Error> {
        Ok(self.get_impl()?.get_session_id())
    }

    /// Sets the action associated with the connection. This is the same as
    /// calling dbms_application_info.set_action() but without executing a
    /// statement. The value is piggybacked to the database with the next
    /// network round trip.
    pub fn set_action(&self, action: &str) -> Result<(), Error> {
        self.get_impl()?.set_pending_action(action);
        Ok(())
    }

    /// Sets the "call timeout" value which is the length of time that is
    /// permitted for the database response to be returned to a request sent by
    /// the client. If None, there is no limit and the client will wait
    /// indefinitely.
    pub fn set_call_timeout(
        &self,
        duration: Option<std::time::Duration>,
    ) -> Result<(), Error> {
        self.get_impl()?.set_call_timeout(duration)
    }

    /// Sets the client identifier associated with the connection. This is the
    /// same as calling dbms_application_info.set_client_identifier() but
    /// without executing a statement. The value is piggybacked to the database
    /// with the next network round trip.
    pub fn set_client_identifier(
        &self,
        client_identifier: &str,
    ) -> Result<(), Error> {
        self.get_impl()?
            .set_pending_client_identifier(client_identifier);
        Ok(())
    }

    /// Sets the client info associated with the connection. This is the same
    /// as calling dbms_application_info.set_client_info() but without
    /// executing a statement. The value is piggybacked to the database with
    /// the next network round trip.
    pub fn set_client_info(&self, client_info: &str) -> Result<(), Error> {
        self.get_impl()?.set_pending_client_info(client_info);
        Ok(())
    }

    /// Sets the Deep Data Security context for subsequent round trips on this
    /// connection.
    pub fn set_end_user_security_context(
        &self,
        context: EndUserSecurityContext,
    ) -> Result<(), Error> {
        self.get_impl()?.set_end_user_security_context(context)
    }

    /// Sets the database operation to be monitored in the database. This is
    /// the same as calling dbms_sql_monitor.begin_operation() but without
    /// executing a statement. The value is piggybacked to the database with
    /// the next network round trip.
    pub fn set_db_op(&self, db_op: &str) -> Result<(), Error> {
        self.get_impl()?.set_pending_db_op(db_op);
        Ok(())
    }

    /// Sets the module associated with the connection. This is the same as
    /// calling dbms_application_info.set_module() but without executing a
    /// statement. The value is piggybacked to the database with the next
    /// network round trip.
    pub fn set_module(&self, db_op: &str) -> Result<(), Error> {
        self.get_impl()?.set_pending_module(db_op);
        Ok(())
    }

    /// Creates a Statement structure which can be used to specify various
    /// statement options.
    pub fn statement<'sql>(
        &self,
        sql: &'sql str,
    ) -> Result<Statement<'sql>, Error> {
        Ok(self.get_impl()?.statement(sql))
    }

    /// Returns the version of the database.
    pub fn version(&self) -> Result<OracleVersion, Error> {
        Ok(self.get_impl()?.get_server_version())
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
