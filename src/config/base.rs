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
// base.rs
//
// Defines the structure used for defining the configuration used for
// establishing a connection to the database.
//-----------------------------------------------------------------------------

use super::connect_options::DescriptionList;
use super::connect_options::DescriptionOption;
use super::connect_string_parser;
use super::defaults;

use crate::error::Error;
use crate::secret_value::SecretValue;

/// Represents configuration used to establish a standalone connection to the
/// database using [connect()](`crate::connect`).
#[derive(Clone, PartialEq, Eq)]
pub struct Config {
    user: Option<String>,
    password: Option<SecretValue>,
    new_password: Option<SecretValue>,
    config_dir: Option<String>,
    description_list: Option<DescriptionList>,
    stmtcachesize: Option<usize>,
    auth_mode: Option<u8>,
    driver_name: Option<String>,
    osuser: Option<String>,
    program: Option<String>,
    machine: Option<String>,
    terminal: Option<String>,
    cclass: Option<String>,
    wallet_location: Option<String>,
    wallet_password: Option<SecretValue>,
}

impl Config {
    /// Returns the new password to use, if one was configured.
    pub(crate) fn get_new_password_bytes(&self) -> Option<Vec<u8>> {
        self.new_password.as_ref().map(|s| s.get_value())
    }

    /// Returns the list of options to use when attempting to connect to the
    /// database.
    pub(crate) fn get_options(&self) -> Result<Vec<DescriptionOption>, Error> {
        self.description_list.as_ref().unwrap().get_options(self)
    }

    /// Returns the password bytes associated with the configuration.
    pub(crate) fn get_password_bytes(&self) -> Vec<u8> {
        self.password.as_ref().unwrap().get_value()
    }

    /// Returns the SDU to use by examining the configuration.
    pub(crate) fn get_sdu(&self) -> usize {
        if let Some(description_list) = &self.description_list {
            description_list.sdu()
        } else {
            0
        }
    }

    /// Returns the wallet password associated with the configuration.
    pub(crate) fn get_wallet_password_bytes(&self) -> Vec<u8> {
        self.wallet_password
            .as_ref()
            .map(|value| value.get_value())
            .unwrap_or_default()
    }

    /// Validates the configuration.
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.user.is_none() || self.password.is_none() {
            Err(Error::no_credentials())
        } else if self.description_list.is_none() {
            Err(Error::no_connect_string())
        } else {
            Ok(())
        }
    }

    /// Returns the authorization mode associated with the configuration.
    pub fn auth_mode(&self) -> u8 {
        self.auth_mode.unwrap_or(0)
    }

    /// Returns the connection class associated with the configuration.
    pub fn cclass(&self) -> Option<&str> {
        self.cclass.as_deref()
    }

    /// Returns the driver name in use by the configuration.
    pub fn driver_name(&self) -> &str {
        self.driver_name
            .as_deref()
            .unwrap_or(defaults::default_driver_name())
    }

    /// Returns the full connect descriptor associated with the configuration.
    pub fn get_connect_descriptor(&self) -> String {
        match self.description_list.as_ref() {
            Some(description_list) => description_list.build_connect_string(),
            None => String::new(),
        }
    }

    /// Returns the machine in use by the configuration. The default value is
    /// the name of the machine on which the program is running.
    pub fn machine(&self) -> &str {
        self.machine
            .as_deref()
            .unwrap_or(defaults::default_machine())
    }

    /// Returns the name of the OS user in use by the configuration. The
    /// default value is the name of the OS user running the program.
    pub fn osuser(&self) -> &str {
        self.osuser.as_deref().unwrap_or(defaults::default_osuser())
    }

    /// Returns the program in use by the configuration. The default value is
    /// the name of the program that is running.
    pub fn program(&self) -> &str {
        self.program
            .as_deref()
            .unwrap_or(defaults::default_program())
    }

    /// Sets the authorization mode that should be used to establish a
    /// connection to the database.
    pub fn set_auth_mode(mut self, value: u8) -> Self {
        self.auth_mode = Some(value);
        self
    }

    /// Sets the connection class to use when connecting to the database.
    pub fn set_cclass(mut self, value: impl Into<String>) -> Self {
        self.cclass = Some(value.into());
        self
    }

    /// Sets the configuration directory to use when searching for a
    /// tnsnames.ora file.
    pub fn set_config_dir(mut self, value: &str) -> Self {
        self.config_dir = Some(value.into());
        self
    }

    /// Sets the connect string to use for connecting to the database. The
    /// connect string will be parsed and checked for validity and an error
    /// returned if the connect string is invalid.
    pub fn set_connect_string(
        mut self,
        connect_string: &str,
    ) -> Result<Self, Error> {
        self.description_list =
            Some(connect_string_parser::parse_connect_string_or_lookup_alias(
                connect_string,
                &self.config_dir,
            )?);
        Ok(self)
    }

    /// Sets the user name and password to use for connecting to the database.
    pub fn set_credentials(self, user: &str, password: &str) -> Self {
        self.set_user(user).set_password(password)
    }

    /// Sets the driver name to use when connecting to the database.
    pub fn set_driver_name(mut self, value: impl Into<String>) -> Self {
        self.driver_name = Some(value.into());
        self
    }

    /// Sets the machine to use when connecting to the database.
    pub fn set_machine(
        mut self,
        value: impl Into<String>,
    ) -> Result<Self, Error> {
        let machine = value.into();
        connect_string_parser::validate_network_name(&machine)?;
        self.machine = Some(machine);
        Ok(self)
    }

    /// Sets the new password to use when connecting to the database.
    pub fn set_new_password(mut self, value: &str) -> Self {
        if value.is_empty() {
            self.new_password = None;
        } else {
            self.new_password = Some(SecretValue::new(value.as_bytes()));
        }
        self
    }

    /// Sets the OS user to use when connecting to the database.
    pub fn set_osuser(
        mut self,
        value: impl Into<String>,
    ) -> Result<Self, Error> {
        let osuser = value.into();
        connect_string_parser::validate_network_name(&osuser)?;
        self.osuser = Some(osuser);
        Ok(self)
    }

    /// Sets the password to use when connecting to the database.
    pub fn set_password(mut self, value: &str) -> Self {
        if value.is_empty() {
            self.password = None;
        } else {
            self.password = Some(SecretValue::new(value.as_bytes()));
        }
        self
    }

    /// Sets the program name to use when connecting to the database.
    pub fn set_program(
        mut self,
        value: impl Into<String>,
    ) -> Result<Self, Error> {
        let program = value.into();
        connect_string_parser::validate_network_name(&program)?;
        self.program = Some(program);
        Ok(self)
    }

    /// Sets the size to use for the statement cache associated with the
    /// connection to the database.
    pub fn set_stmtcachesize(mut self, value: usize) -> Self {
        self.stmtcachesize = Some(value);
        self
    }

    /// Sets the terminal to use when connecting to the database.
    pub fn set_terminal(mut self, value: impl Into<String>) -> Self {
        self.terminal = Some(value.into());
        self
    }

    /// Sets the user to use when connecting to the database.
    pub fn set_user(mut self, value: &str) -> Self {
        if value.is_empty() {
            self.user = None;
        } else {
            self.user = Some(value.into());
        }
        self
    }

    /// Sets the location to use for loading a wallet (ewallet.pem).
    pub fn set_wallet_location(mut self, value: impl Into<String>) -> Self {
        self.wallet_location = Some(value.into());
        self
    }

    /// Sets the password to use when loading a wallet (ewallet.pem).
    pub fn set_wallet_password(mut self, value: &str) -> Self {
        if value.is_empty() {
            self.wallet_password = None;
        } else {
            self.wallet_password = Some(SecretValue::new(value.as_bytes()));
        }
        self
    }

    /// Returns the size of the statement cache associated with the
    /// configuration.
    pub fn stmtcachesize(&self) -> usize {
        self.stmtcachesize.unwrap_or(20)
    }

    /// Returns the terminal in use by the configuration or a default value of
    /// "unknown".
    pub fn terminal(&self) -> &str {
        self.terminal.as_deref().unwrap_or("unknown")
    }

    /// Returns the user associated with the configuration.
    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    /// Returns the wallet location associated with the configuration.
    pub fn wallet_location(&self) -> Option<&str> {
        self.wallet_location.as_deref()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            user: None,
            password: None,
            new_password: None,
            config_dir: defaults::default_config_dir().clone(),
            description_list: None,
            stmtcachesize: None,
            auth_mode: None,
            driver_name: None,
            osuser: None,
            program: None,
            machine: None,
            terminal: None,
            cclass: None,
            wallet_location: None,
            wallet_password: None,
        }
    }
}
