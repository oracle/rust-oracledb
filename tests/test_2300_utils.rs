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
// test_2300_utils()
//-----------------------------------------------------------------------------

use oracledb;

#[test]
/// Test enquote_literal().
fn test_2300() -> Result<(), oracledb::Error> {
    assert_eq!(oracledb::enquote_literal("test_2300"), "'test_2300'");
    assert_eq!(oracledb::enquote_literal("a'b'c'd"), "'a''b''c''d'");
    assert_eq!(oracledb::enquote_literal("'abc'"), "'''abc'''");
    assert_eq!(oracledb::enquote_literal(""), "''");
    Ok(())
}

#[test]
/// Test enquote_name().
fn test_2301() -> Result<(), oracledb::Error> {
    let result_1 = oracledb::enquote_name("test_2301a", false)?;
    assert_eq!(result_1, "\"test_2301a\"");
    let result_2 = oracledb::enquote_name("test_2301b", true)?;
    assert_eq!(result_2, "\"TEST_2301B\"");
    let result_3 = oracledb::enquote_name("", true)?;
    assert_eq!(result_3, "\"\"");
    assert!(oracledb::enquote_name("test_\"2301c", true).is_err());
    Ok(())
}

#[test]
/// Test is_qualified_sql_name().
fn test_2302() -> Result<(), oracledb::Error> {
    assert!(oracledb::is_qualified_sql_name("test_2302"));
    assert!(oracledb::is_qualified_sql_name("test_2302.subvalue"));
    assert!(oracledb::is_qualified_sql_name(
        "test_2302.subvalue.tertiary"
    ));
    assert!(oracledb::is_qualified_sql_name("    test_2302   "));
    assert!(oracledb::is_qualified_sql_name(
        "    test_2302  .  subvalue  "
    ));
    assert!(oracledb::is_qualified_sql_name(
        "    \"test_2302\"  .  subvalue  "
    ));
    assert!(oracledb::is_qualified_sql_name(
        "    \"test_2302\"  .  \"subvalue\"  "
    ));
    assert!(oracledb::is_qualified_sql_name("test2302@dblink"));
    assert!(oracledb::is_qualified_sql_name("test2302   @   dblink"));
    assert!(oracledb::is_qualified_sql_name("test2302@\"dblink\""));
    assert!(!oracledb::is_qualified_sql_name(
        "    \"test_2302\"  .  \"subvalue\"  extraneous "
    ));
    assert!(!oracledb::is_qualified_sql_name(
        "    \"test_2302\"  -  \"subvalue\"  "
    ));
    assert!(!oracledb::is_qualified_sql_name("test2302   @   1notalink"));
    assert!(!oracledb::is_qualified_sql_name("test2302@dblink.wrong"));
    Ok(())
}

#[test]
/// Test is_simple_sql_name().
fn test_2303() -> Result<(), oracledb::Error> {
    assert!(oracledb::is_simple_sql_name("test_2303#$"));
    assert!(oracledb::is_simple_sql_name("   test_2303    "));
    assert!(oracledb::is_simple_sql_name("\"test_2303\""));
    assert!(oracledb::is_simple_sql_name("  \"test_2303\"    "));
    assert!(!oracledb::is_simple_sql_name("    "));
    assert!(!oracledb::is_simple_sql_name("\"\""));
    assert!(!oracledb::is_simple_sql_name("   \"\"   "));
    assert!(!oracledb::is_simple_sql_name("\"test_2303"));
    assert!(!oracledb::is_simple_sql_name("test_2303."));
    assert!(!oracledb::is_simple_sql_name("\"test_2303\" after\""));
    assert!(!oracledb::is_simple_sql_name("test_2303 embedded spaces"));
    assert!(!oracledb::is_simple_sql_name("\"test_2303\" extraneous"));
    assert!(!oracledb::is_simple_sql_name("12345"));
    Ok(())
}
