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

use std::sync::LazyLock;

use crate::config::connect_string_parser::sanitize_network_name;
use crate::constants;

static DEFAULTS: LazyLock<Defaults> = LazyLock::new(|| {
    // calculate default driver name
    let driver_name =
        format!("{} : {}", constants::DRIVER_NAME, env!("CARGO_PKG_VERSION"));

    // calculate default configuration directory; this will be either the value
    // of the environment variable TNS_ADMIN, or, if the ORACLE_HOME
    // environment variable is set, the value of $ORACLE_HOME/network/admin
    // instead. If neither environment variable is set, None is used.
    let mut config_dir: Option<String> = None;
    if let Ok(value) = std::env::var("TNS_ADMIN")
        && !value.is_empty()
    {
        config_dir = Some(value);
    } else if let Ok(value) = std::env::var("ORACLE_HOME")
        && !value.is_empty()
    {
        let mut path = std::path::PathBuf::new();
        path.push(value);
        path.push("network");
        path.push("admin");
        config_dir = Some(path.to_string_lossy().into_owned());
    }

    // calculate the program that is being run
    let program = std::env::current_exe()
        .map(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or(None)
        .unwrap_or(String::from("unknown"));

    // calculate the host name on which the program is being run
    let machine = whoami::hostname().unwrap_or(String::from("unknown"));

    // calculate the name of the user running the program
    let osuser = whoami::username().unwrap_or(String::from("unknown"));

    Defaults {
        config_dir,
        driver_name,
        program: sanitize_network_name(&program),
        machine: sanitize_network_name(&machine),
        osuser: sanitize_network_name(&osuser),
    }
});

struct Defaults {
    config_dir: Option<String>,
    driver_name: String,
    program: String,
    machine: String,
    osuser: String,
}

/// Returns the default configuration directory.
pub(crate) fn default_config_dir() -> &'static Option<String> {
    &DEFAULTS.config_dir
}

/// Returns the default driver name.
pub(crate) fn default_driver_name() -> &'static String {
    &DEFAULTS.driver_name
}

/// Returns the default machine.
pub(crate) fn default_machine() -> &'static String {
    &DEFAULTS.machine
}

/// Returns the default OS username.
pub(crate) fn default_osuser() -> &'static String {
    &DEFAULTS.osuser
}

/// Returns the default program.
pub(crate) fn default_program() -> &'static String {
    &DEFAULTS.program
}
