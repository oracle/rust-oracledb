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
// Common module for testing the oracledb crate.
//-----------------------------------------------------------------------------

use std::env;

/// Gets an environment variable, or returns the supplied default.
/// Panics when the variable is absent and no default is provided.
fn get_env_value(name: &str) -> String {
    match env::var_os(name) {
        Some(value) => value.to_str().unwrap().to_string(),
        None => {
            panic!("Value for environment variable {name} is not set!");
        }
    }
}

#[allow(dead_code)]
/// Returns the sample configuration.
pub fn get_sample_config() -> Result<oracledb::Config, oracledb::Error> {
    let user = get_env_value("RSO_EXAMPLES_MAIN_USER");
    let password = get_env_value("RSO_EXAMPLES_MAIN_PASSWORD");
    let connect_string = get_env_value("RSO_EXAMPLES_CONNECT_STRING");
    oracledb::Config::default()
        .set_credentials(&user, &password)
        .set_connect_string(&connect_string)
}

#[allow(dead_code)]
/// Returns the sample pool configuration.
pub fn get_sample_pool_config() -> Result<oracledb::PoolConfig, oracledb::Error>
{
    let user = get_env_value("RSO_EXAMPLES_MAIN_USER");
    let password = get_env_value("RSO_EXAMPLES_MAIN_PASSWORD");
    let connect_string = get_env_value("RSO_EXAMPLES_CONNECT_STRING");
    oracledb::PoolConfig::default()
        .set_credentials(&user, &password)
        .set_connect_string(&connect_string)
}
