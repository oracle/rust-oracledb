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
// test_2000_connection()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;

/// Creates an end user security context for testing.
fn create_end_user_security_context(
    token: String,
    database_access_token: String,
) -> Result<oracledb::EndUserSecurityContext, oracledb::Error> {
    oracledb::EndUserSecurityContext::new(
        oracledb::EndUserIdentity::Token(token),
        database_access_token,
    )
}

/// Tests a single end-to-end attribute matches the reality.
fn test_end_to_end_attr(
    conn: &oracledb::Connection,
    value: &str,
    sql: &str,
) -> Result<(), oracledb::Error> {
    let cursor = conn.query(sql, &[])?;
    for row in cursor {
        let row = row?;
        let fetched_value: Option<String> = row.get(0)?;
        assert_eq!(fetched_value.unwrap_or(String::new()), value);
    }
    Ok(())
}

#[rstest]
/// test end-to-end attributes
fn test_2000(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    for value in ["oracledb_action", ""] {
        let sql = "select sys_context('userenv', 'action') from dual";
        conn.set_action(value)?;
        test_end_to_end_attr(&conn, value, sql)?;
    }
    for value in ["oracledb_cid", ""] {
        let sql =
            "select sys_context('userenv', 'client_identifier') from dual";
        conn.set_client_identifier(value)?;
        test_end_to_end_attr(&conn, value, sql)?;
    }
    for value in ["oracledb_cinfo", ""] {
        let sql = "select sys_context('userenv', 'client_info') from dual";
        conn.set_client_info(value)?;
        test_end_to_end_attr(&conn, value, sql)?;
    }
    for value in ["oracledb_dbop", "oracledb_other_dbop"] {
        let sql = r#"
            select dbop_name from v$sql_monitor
            where sid = sys_context('userenv', 'sid')
            and status = 'EXECUTING'
            "#;
        conn.set_db_op(value)?;
        test_end_to_end_attr(&conn, value, sql)?;
    }
    for value in ["oracledb_module", ""] {
        let sql = "select sys_context('userenv', 'module') from dual";
        conn.set_module(value)?;
        test_end_to_end_attr(&conn, value, sql)?;
    }
    Ok(())
}

#[rstest]
/// test invalid end user security context length
fn test_2001() -> Result<(), oracledb::Error> {
    match create_end_user_security_context(
        "x".repeat(70_000),
        "y".repeat(70_000),
    ) {
        Ok(_) => panic!("expected context creation to fail"),
        Err(err) => {
            assert!(matches!(
                err.kind(),
                oracledb::ErrorKind::InvalidEndUserSecurityContextLength(_)
            ));
        }
    }
    Ok(())
}

#[rstest]
#[case(oracledb::EndUserIdentity::Token("".to_string()))]
#[case(oracledb::EndUserIdentity::DatabaseUser {
    name: "".to_string(),
    key: None,
})]
#[case(oracledb::EndUserIdentity::DatabaseUser {
    name: "".to_string(),
    key: Some("key_2002".to_string()),
})]
#[case(oracledb::EndUserIdentity::DatabaseUser {
    name: "end_user_2002".to_string(),
    key: Some("".to_string()),
})]
/// test end user security context rejects invalid identities
fn test_2002(
    #[case] identity: oracledb::EndUserIdentity,
) -> Result<(), oracledb::Error> {
    match oracledb::EndUserSecurityContext::new(
        identity,
        "database_access_token_2002".to_string(),
    ) {
        Ok(_) => panic!("expected context creation to fail"),
        Err(err) => {
            assert!(matches!(
                err.kind(),
                oracledb::ErrorKind::InvalidEndUserSecurityContext(_)
            ));
        }
    }
    Ok(())
}

#[rstest]
#[case(oracledb::EndUserIdentity::DatabaseUser {
    name: "end_user_2003".to_string(),
    key: Some("key_2003".to_string()),
})]
#[case(oracledb::EndUserIdentity::DatabaseUser {
    name: "end_user_2003".to_string(),
    key: None,
})]
/// test end user security context accepts database user identities
fn test_2003(
    #[case] identity: oracledb::EndUserIdentity,
) -> Result<(), oracledb::Error> {
    oracledb::EndUserSecurityContext::new(
        identity,
        "database_access_token_2003".to_string(),
    )?;
    Ok(())
}

#[rstest]
/// test end user security context rejects empty database access token
fn test_2004() -> Result<(), oracledb::Error> {
    match create_end_user_security_context(
        "end_user_token_2004".to_string(),
        "".to_string(),
    ) {
        Ok(_) => panic!("expected context creation to fail"),
        Err(err) => {
            assert!(matches!(
                err.kind(),
                oracledb::ErrorKind::InvalidEndUserSecurityContext(_)
            ));
        }
    }
    Ok(())
}

#[rstest]
/// Tests missing credentials and missing connect strings before the network
/// layer is used.
fn test_2005() {
    let err = match oracledb::connect(oracledb::Config::default()) {
        Ok(_) => panic!("missing credentials must fail"),
        Err(err) => err,
    };
    assert!(matches!(err.kind(), oracledb::ErrorKind::NoCredentials));

    let err = match oracledb::connect(
        oracledb::Config::default().set_credentials("user", "password"),
    ) {
        Ok(_) => panic!("missing connect string must fail"),
        Err(err) => err,
    };
    assert!(matches!(err.kind(), oracledb::ErrorKind::NoConnectString));
}
