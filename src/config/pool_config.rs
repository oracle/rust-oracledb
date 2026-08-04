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
// pool_config.rs
//
// Defines the structure used for defining the configuration used for
// establishing a connection to the database.
//-----------------------------------------------------------------------------

use std::time::Duration;

use super::Config;

use crate::error::Error;

/// Represents configuration used to create a connection pool using
/// [create_pool()](`crate::create_pool`).
#[derive(Clone, PartialEq, Eq)]
pub struct PoolConfig {
    connection_config: Config,
    min_connections: Option<usize>,
    max_connections: Option<usize>,
    connection_increment: Option<usize>,
    ping_interval: Option<Duration>,
    ping_timeout: Option<Duration>,
}

impl PoolConfig {
    /// Validates the configuration.
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.max_connections() < self.min_connections() {
            Err(Error::pool_max_less_than_min())
        } else if self.connection_increment() == 0
            && self.max_connections() != self.min_connections()
        {
            Err(Error::pool_increment_zero())
        } else {
            self.connection_config.validate()
        }
    }

    /// Returns the authorization mode associated with the configuration.
    pub fn auth_mode(&self) -> u8 {
        self.connection_config.auth_mode()
    }

    /// Returns the connection class associated with the configuration.
    pub fn cclass(&self) -> Option<&str> {
        self.connection_config.cclass()
    }

    /// Returns a reference to the stored connection configuration.
    pub fn connection_config(&self) -> &Config {
        &self.connection_config
    }

    /// Returns the number of connections a pool using this configuration
    /// should create when the pool needs to grow.
    pub fn connection_increment(&self) -> usize {
        self.connection_increment.unwrap_or(1)
    }

    /// Returns the driver name in use by the configuration.
    pub fn driver_name(&self) -> &str {
        self.connection_config.driver_name()
    }

    /// Returns the full connect descriptor associated with the configuration.
    pub fn get_connect_descriptor(&self) -> String {
        self.connection_config.get_connect_descriptor()
    }

    /// Returns the machine in use by the configuration. The default value is
    /// the name of the machine on which the program is running.
    pub fn machine(&self) -> &str {
        self.connection_config.machine()
    }

    /// Returns the maximum number of connections a pool using this
    /// configuration should contain.
    pub fn max_connections(&self) -> usize {
        self.max_connections.unwrap_or(2)
    }

    /// Returns the minimum number of connections a pool using this
    /// configuration should contain.
    pub fn min_connections(&self) -> usize {
        self.min_connections.unwrap_or(1)
    }

    /// Returns the name of the OS user in use by the configuration. The
    /// default value is the name of the OS user running the program.
    pub fn osuser(&self) -> &str {
        self.connection_config.osuser()
    }

    /// Returns the interval after which a ping should be performed on a
    /// connection before being returned to the caller. A value of None implies
    /// that pings will never be performed.
    pub fn ping_interval(&self) -> Option<Duration> {
        self.ping_interval
    }

    /// Returns the length of time that the pool will wait for a successful
    /// ping before the connection is discarded.
    pub fn ping_timeout(&self) -> Duration {
        self.ping_timeout.unwrap_or(Duration::from_secs(5))
    }

    /// Returns the program in use by the configuration. The default value is
    /// the name of the program that is running.
    pub fn program(&self) -> &str {
        self.connection_config.program()
    }

    /// Sets the authorization mode that should be used to establish a
    /// connection to the database.
    pub fn set_auth_mode(mut self, value: u8) -> Self {
        self.connection_config = self.connection_config.set_auth_mode(value);
        self
    }

    /// Sets the connection class to use when connecting to the database.
    pub fn set_cclass(mut self, value: impl Into<String>) -> Self {
        self.connection_config = self.connection_config.set_cclass(value);
        self
    }

    /// Sets the configuration directory to use when searching for a
    /// tnsnames.ora file.
    pub fn set_config_dir(mut self, value: &str) -> Self {
        self.connection_config = self.connection_config.set_config_dir(value);
        self
    }

    /// Sets the number of connections that should be created when a pool using
    /// this configuration needs to grow.
    pub fn set_connection_increment(mut self, value: usize) -> Self {
        self.connection_increment = Some(value);
        self
    }

    /// Sets the connect string to use for connecting to the database. The
    /// connect string will be parsed and checked for validity and an error
    /// returned if the connect string is invalid.
    pub fn set_connect_string(
        mut self,
        connect_string: &str,
    ) -> Result<Self, Error> {
        self.connection_config =
            self.connection_config.set_connect_string(connect_string)?;
        Ok(self)
    }

    /// Sets the user name and password to use for connecting to the database.
    pub fn set_credentials(self, user: &str, password: &str) -> Self {
        self.set_user(user).set_password(password)
    }

    /// Sets the driver name to use when connecting to the database.
    pub fn set_driver_name(mut self, value: impl Into<String>) -> Self {
        self.connection_config = self.connection_config.set_driver_name(value);
        self
    }

    /// Sets the machine to use when connecting to the database.
    pub fn set_machine(
        mut self,
        value: impl Into<String>,
    ) -> Result<Self, Error> {
        self.connection_config = self.connection_config.set_machine(value)?;
        Ok(self)
    }

    /// Sets the maximum number of connections a pool using this configuration
    /// should contain.
    pub fn set_max_connections(mut self, value: usize) -> Self {
        self.max_connections = Some(value);
        self
    }

    /// Sets the minimum number of connections a pool using this configuration
    /// should contain.
    pub fn set_min_connections(mut self, value: usize) -> Self {
        self.min_connections = Some(value);
        self
    }

    /// Sets the OS user to use when connecting to the database.
    pub fn set_osuser(
        mut self,
        value: impl Into<String>,
    ) -> Result<Self, Error> {
        self.connection_config = self.connection_config.set_osuser(value)?;
        Ok(self)
    }

    /// Sets the password to use when connecting to the database.
    pub fn set_password(mut self, value: &str) -> Self {
        self.connection_config = self.connection_config.set_password(value);
        self
    }

    /// Sets the interval after which a ping should be performed on a
    /// connection before being returned to the caller. A value of None implies
    /// that pings will never be performed.
    pub fn set_ping_interval(mut self, value: Option<Duration>) -> Self {
        self.ping_interval = value;
        self
    }

    /// Sets the length of time that the pool will wait for a successful ping
    /// before the connection is discarded.
    pub fn set_ping_timeout(mut self, value: Duration) -> Self {
        self.ping_timeout = Some(value);
        self
    }

    /// Sets the program name to use when connecting to the database.
    pub fn set_program(
        mut self,
        value: impl Into<String>,
    ) -> Result<Self, Error> {
        self.connection_config = self.connection_config.set_program(value)?;
        Ok(self)
    }

    /// Sets the size to use for the statement cache associated with the
    /// connection to the database.
    pub fn set_stmtcachesize(mut self, value: usize) -> Self {
        self.connection_config =
            self.connection_config.set_stmtcachesize(value);
        self
    }

    /// Sets the terminal to use when connecting to the database.
    pub fn set_terminal(mut self, value: impl Into<String>) -> Self {
        self.connection_config = self.connection_config.set_terminal(value);
        self
    }

    /// Sets the user to use when connecting to the database.
    pub fn set_user(mut self, value: &str) -> Self {
        self.connection_config = self.connection_config.set_user(value);
        self
    }

    /// Sets the location to use for loading a wallet (ewallet.pem).
    pub fn set_wallet_location(mut self, value: impl Into<String>) -> Self {
        self.connection_config =
            self.connection_config.set_wallet_location(value);
        self
    }

    /// Sets the password to use when loading a wallet (ewallet.pem).
    pub fn set_wallet_password(mut self, value: &str) -> Self {
        self.connection_config =
            self.connection_config.set_wallet_password(value);
        self
    }

    /// Returns the size of the statement cache associated with the
    /// configuration.
    pub fn stmtcachesize(&self) -> usize {
        self.connection_config.stmtcachesize()
    }

    /// Returns the terminal in use by the configuration or a default value of
    /// "unknown".
    pub fn terminal(&self) -> &str {
        self.connection_config.terminal()
    }

    /// Returns the user associated with the configuration.
    pub fn user(&self) -> Option<&str> {
        self.connection_config.user()
    }

    /// Returns the wallet location associated with the configuration.
    pub fn wallet_location(&self) -> Option<&str> {
        self.connection_config.wallet_location()
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            connection_config: Config::default(),
            min_connections: None,
            max_connections: None,
            connection_increment: None,
            ping_interval: Some(Duration::from_secs(60)),
            ping_timeout: None,
        }
    }
}
