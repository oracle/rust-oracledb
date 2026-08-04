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
// test_config.rs
//
// Acquires the test configuration from the environment. This is used by the
// integration tests as well as the helper binaries that create and destroy the
// schema used for running the integration tests.
//
// Rust runs each integration test file independently so setting up and
// destroying the user for each test file would incur a great deal of overhead,
// so the helper binaries are available to create the schema prior to running
// the tests.
//-----------------------------------------------------------------------------

use std::env;
use std::sync::LazyLock;

use crate::Config;

static TEST_CONFIG: LazyLock<TestConfig> = LazyLock::new(|| {
    // get values from environment
    let admin_user = get_env_value("RSO_TEST_ADMIN_USER", "sys");
    let admin_password = get_env_value("RSO_TEST_ADMIN_PASSWORD", "");
    let user = get_env_value("RSO_TEST_MAIN_USER", "rsotest");
    let password = get_env_value("RSO_TEST_MAIN_PASSWORD", "");
    let connect_string = get_env_value("RSO_TEST_CONNECT_STRING", "");

    // setup admin configuration and validate it
    let base_admin_config = Config::default()
        .set_credentials(&admin_user, &admin_password)
        .set_connect_string(&connect_string)
        .expect("Invalid connect string");
    let admin_config = if admin_user.to_lowercase() == "sys" {
        base_admin_config.set_auth_mode(crate::AUTH_MODE_SYSDBA)
    } else {
        base_admin_config
    };
    crate::connect(admin_config.clone())
        .expect("Unable to establish connection as test administrator.");

    // setup regular test user configuration but do not validate it since the
    // test user may not have been created at this point!
    let config = Config::default()
        .set_credentials(&user, &password)
        .set_connect_string(&connect_string)
        .expect("Invalid connect string");

    // return the configuration now that it has been validated
    TestConfig {
        admin_user,
        admin_password,
        user,
        password,
        connect_string,
        admin_config,
        config,
    }
});

#[doc(hidden)]
pub struct TestConfig {
    pub admin_user: String,
    pub admin_password: String,
    pub user: String,
    pub password: String,
    pub connect_string: String,
    pub admin_config: Config,
    pub config: Config,
}

/// Internal method for getting a value from the environment with a default
/// value. The function panics if no value is found in the environment and no
/// default value is supplied.
fn get_env_value(name: &str, default_value: &str) -> String {
    let value = env::var_os(name);
    if value.is_none() {
        if !default_value.is_empty() {
            return default_value.to_string();
        }
        panic!("Value for environment variable {name} is not set!");
    }
    value.unwrap().to_str().unwrap().to_string()
}

/// Returns the test configuration.
#[doc(hidden)]
pub fn get_test_config() -> &'static TestConfig {
    &TEST_CONFIG
}
