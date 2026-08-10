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
// test_3300_config()
//-----------------------------------------------------------------------------

use oracledb;
use rstest::*;

#[rstest]
/// Tests configuration setters and getters.
fn test_3300() -> Result<(), oracledb::Error> {
    let default_config = oracledb::Config::default();
    assert_eq!(default_config.auth_mode(), oracledb::AUTH_MODE_DEFAULT);
    assert!(default_config.cclass().is_none());
    assert!(default_config.get_connect_descriptor().is_empty());
    assert!(default_config.user().is_none());
    assert!(default_config.wallet_location().is_none());
    let config = default_config
        .set_auth_mode(oracledb::AUTH_MODE_SYSDBA)
        .set_cclass("cclass_3300")
        .set_credentials("user_3300", "password_3300")
        .set_driver_name("driver_name_3300")
        .set_machine("machine_3300")?
        .set_osuser("osuser_3300")?
        .set_program("program_3300")?
        .set_stmtcachesize(50)
        .set_terminal("terminal_3300")
        .set_wallet_location("wallet_location_3300");
    assert_eq!(config.auth_mode(), oracledb::AUTH_MODE_SYSDBA);
    assert_eq!(config.cclass(), Some("cclass_3300"));
    assert_eq!(config.driver_name(), "driver_name_3300");
    assert_eq!(config.machine(), "machine_3300");
    assert_eq!(config.osuser(), "osuser_3300");
    assert_eq!(config.program(), "program_3300");
    assert_eq!(config.stmtcachesize(), 50);
    assert_eq!(config.terminal(), "terminal_3300");
    assert_eq!(config.user(), Some("user_3300"));
    assert_eq!(config.wallet_location(), Some("wallet_location_3300"));
    Ok(())
}

#[rstest]
#[case(oracledb::Config::set_machine)]
#[case(oracledb::Config::set_osuser)]
#[case(oracledb::Config::set_program)]
/// Tests invalid network names for machine, osuser and program.
fn test_3301(
    #[case] method: fn(
        oracledb::Config,
        String,
    ) -> Result<oracledb::Config, oracledb::Error>,
    #[values(
        "'contains_quotes'",
        "\"contains_double_quotes\"",
        "contains spaces",
        "contains_opening_paren(",
        "contains_closing_paren)",
        "contains_equals=",
        "contains_trailing_slash\\",
        "contains_unicode_東京"
    )]
    value: &str,
) -> Result<(), oracledb::Error> {
    let config = oracledb::Config::default();
    let err = match method(config, value.to_string()) {
        Ok(_) => panic!("expected failure"),
        Err(err) => err,
    };
    assert!(matches!(
        err.kind(),
        oracledb::ErrorKind::InvalidNetworkName(_)
    ));
    Ok(())
}

#[rstest]
// simple easy connect string
#[case(
    "host_3302:3302/service_name_3302",
    "(DESCRIPTION=(ADDRESS=(PROTOCOL=tcp)(HOST=host_3302)(PORT=3302))\
         (CONNECT_DATA=(SERVICE_NAME=service_name_3302)))"
)]
// easy connect string with multiple hosts, all with the same port
#[case(
    "host_3302a,host_3302b:3302/service_name_3302",
    "(DESCRIPTION=(ADDRESS_LIST=\
         (ADDRESS=(PROTOCOL=tcp)(HOST=host_3302a)(PORT=3302))\
         (ADDRESS=(PROTOCOL=tcp)(HOST=host_3302b)(PORT=3302)))\
         (CONNECT_DATA=(SERVICE_NAME=service_name_3302)))"
)]
// easy connect string with multiple hosts, with diffent ports
#[case(
    "host_3302a,host_3302b:3302,host_3302c,host_3302d:8302/service_name_3302",
    "(DESCRIPTION=(ADDRESS_LIST=\
         (ADDRESS=(PROTOCOL=tcp)(HOST=host_3302a)(PORT=3302))\
         (ADDRESS=(PROTOCOL=tcp)(HOST=host_3302b)(PORT=3302))\
         (ADDRESS=(PROTOCOL=tcp)(HOST=host_3302c)(PORT=8302))\
         (ADDRESS=(PROTOCOL=tcp)(HOST=host_3302d)(PORT=8302)))\
         (CONNECT_DATA=(SERVICE_NAME=service_name_3302)))"
)]
// easy connect string with multiple address lists
#[case(
    "host_3302a;host_3302b,host_3302c:3302;host_3302d/service_name_3302",
    "(DESCRIPTION=\
         (ADDRESS=(PROTOCOL=tcp)(HOST=host_3302a)(PORT=1521))\
         (ADDRESS_LIST=(ADDRESS=(PROTOCOL=tcp)(HOST=host_3302b)(PORT=3302))\
         (ADDRESS=(PROTOCOL=tcp)(HOST=host_3302c)(PORT=3302)))\
         (ADDRESS=(PROTOCOL=tcp)(HOST=host_3302d)(PORT=1521))\
         (CONNECT_DATA=(SERVICE_NAME=service_name_3302)))"
)]
// full descriptor with address immediately under description
#[case(
    "(DESCRIPTION=(ADDRESS=(PROTOCOL=TCP)(HOST=host_3302)(PORT=3302))\
        (CONNECT_DATA=(SERVICE_NAME=service_name_3302)))",
    "(DESCRIPTION=(ADDRESS=(PROTOCOL=tcp)(HOST=host_3302)(PORT=3302))\
        (CONNECT_DATA=(SERVICE_NAME=service_name_3302)))"
)]
// full descriptor with all components specified
#[case(
    "(DESCRIPTION=(ADDRESS_LIST=(ADDRESS=(PROTOCOL=TCP)(HOST=host_3302)\
        (PORT=1521)))(CONNECT_DATA=(SERVICE_NAME=service_name_3302)))",
    "(DESCRIPTION=(ADDRESS=(PROTOCOL=tcp)(HOST=host_3302)\
        (PORT=1521))(CONNECT_DATA=(SERVICE_NAME=service_name_3302)))"
)]
/// Tests parsing connect strings.
fn test_3302(
    #[case] in_value: &str,
    #[case] expected_value: &str,
) -> Result<(), oracledb::Error> {
    let config = oracledb::Config::default().set_connect_string(in_value)?;
    assert_eq!(config.get_connect_descriptor(), expected_value);
    Ok(())
}

#[rstest]
// missing equals sign
#[case("(NOT_VALID)")]
// no top node of "description_list" or "description"
#[case("(KEY=VALUE)")]
// no child node of type description
#[case("(DESCRIPTION_LIST=(KEY=VALUE))")]
// no child node of type address
#[case("(DESCRIPTION_LIST=(ADRESS_LIST=(KEY=VALUE)))")]
/// Tests parsing invalid connect strings.
fn test_3303(#[case] value: &str) -> Result<(), oracledb::Error> {
    let err = match oracledb::Config::default().set_connect_string(value) {
        Ok(_) => panic!("expected failure"),
        Err(err) => err,
    };
    println!("error kind is {:?}", err.kind());
    assert!(matches!(
        err.kind(),
        oracledb::ErrorKind::InvalidConnectString(_, _)
            | oracledb::ErrorKind::ParseError(_, _)
    ));
    Ok(())
}
