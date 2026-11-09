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

use common::admin_conn;
use common::conn;
use rstest::*;
use std::time::{Duration, Instant};

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
        assert_eq!(fetched_value.unwrap_or_default(), value);
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
    if !common::is_on_oracle_cloud(&conn)? {
        for value in ["oracledb_dbop", ""] {
            let sql = r#"
                select dbop_name
                from v$sql_monitor
                where sid = sys_context('userenv', 'sid')
                  and status = 'EXECUTING'
                "#;
            conn.set_db_op(value)?;
            test_end_to_end_attr(&conn, value, sql)?;
        }
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
    let error = create_end_user_security_context(
        "x".repeat(70_000),
        "y".repeat(70_000),
    )
    .expect_err("expected context creation to fail");
    assert!(matches!(
        error.kind(),
        oracledb::ErrorKind::InvalidEndUserSecurityContextLength(_)
    ));
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
    let error = oracledb::EndUserSecurityContext::new(
        identity,
        "database_access_token_2002".to_string(),
    )
    .expect_err("expected context creation to fail");
    assert!(matches!(
        error.kind(),
        oracledb::ErrorKind::InvalidEndUserSecurityContext(_)
    ));
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
    let error = create_end_user_security_context(
        "end_user_token_2004".to_string(),
        "".to_string(),
    )
    .expect_err("expected context creation to fail");
    assert!(matches!(
        error.kind(),
        oracledb::ErrorKind::InvalidEndUserSecurityContext(_)
    ));
    Ok(())
}

#[rstest]
/// Tests missing credentials and missing connect strings before the network
/// layer is used.
fn test_2005() {
    let error = oracledb::connect(oracledb::Config::default())
        .expect_err("missing credentials must fail");
    assert!(matches!(error.kind(), oracledb::ErrorKind::NoCredentials));

    let error = oracledb::connect(
        oracledb::Config::default().set_credentials("user", "password"),
    )
    .expect_err("missing connect string must fail");
    assert!(matches!(error.kind(), oracledb::ErrorKind::NoConnectString));
}

#[rstest]
/// Tests that a call timeout is retained across a round trip and can be
/// cleared.
fn test_2006(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let timeout = Duration::from_secs(5);
    conn.set_call_timeout(Some(timeout))?;
    conn.ping()?;
    assert_eq!(conn.call_timeout()?, Some(timeout));

    conn.set_call_timeout(None)?;
    conn.ping()?;
    assert_eq!(conn.call_timeout()?, None);
    Ok(())
}

#[rstest]
/// Tests that explicitly closing a standalone connection rolls back its
/// uncommitted transaction before the session is released.
fn test_2007(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let mut conn = conn;
    let observer = common::conn();
    let _guard =
        common::create_table(&observer, "test_2007", "id number primary key")?;

    conn.execute("insert into test_2007 values (1)", &[])?;
    conn.close()?;

    let row = observer.query_row("select count(*) from test_2007", &[])?;
    let count: i32 = row.get(0)?;
    assert_eq!(count, 0);
    Ok(())
}

#[rstest]
/// Tests that dropping a standalone connection rolls back its uncommitted
/// transaction before its underlying session is closed.
fn test_2008(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let observer = common::conn();
    let _guard =
        common::create_table(&observer, "test_2008", "id number primary key")?;

    conn.execute("insert into test_2008 values (1)", &[])?;
    drop(conn);

    let row = observer.query_row("select count(*) from test_2008", &[])?;
    let count: i32 = row.get(0)?;
    assert_eq!(count, 0);
    Ok(())
}

#[rstest]
/// Verifies a real call timeout interrupts a long operation and leaves the
/// connection reusable after protocol reset.
fn test_2009(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    conn.set_call_timeout(Some(Duration::from_millis(600)))?;
    let started = Instant::now();
    let error = conn
        .execute("begin dbms_session.sleep(1); end;", &[])
        .expect_err("the database call must exceed the configured timeout");
    let elapsed = started.elapsed();
    assert!(matches!(error.kind(), oracledb::ErrorKind::CallTimeoutExceeded));
    assert!(elapsed >= Duration::from_millis(200));
    assert!(elapsed < Duration::from_secs(5));

    conn.set_call_timeout(None)?;
    let row = conn.query_row("select 2009 from dual", &[])?;
    assert_eq!(row.get::<i32>(0)?, 2009);
    Ok(())
}

#[rstest]
/// Tests that a call cancelled by the server (using ALTER SYSTEM CANCEL SQL,
/// available in Oracle Database 18c and higher) fails with error ORA-01013
/// and leaves the connection reusable after protocol reset.
fn test_2010(
    admin_conn: oracledb::Connection,
    conn: oracledb::Connection,
) -> Result<(), oracledb::Error> {
    if conn.version()?.0 < 18 {
        common::skip_test("test requires Oracle Database 18 or higher");
        return Ok(());
    }
    let session_id = conn.session_id()?;
    let serial_num = conn.serial_num()?;
    let thread = std::thread::spawn(move || {
        let result = conn.query_row(
            "select count(*) from all_objects cross join all_objects",
            &[],
        );
        (result, conn)
    });
    std::thread::sleep(Duration::from_millis(1000));
    admin_conn.execute(
        &format!("alter system cancel sql '{session_id},{serial_num}'"),
        &[],
    )?;
    let (result, conn) = thread.join().unwrap();
    let error = result.unwrap_err();
    assert_eq!(error.db_error_code(), Some(1013));
    let row = conn.query_row("select 2010 from dual", &[])?;
    assert_eq!(row.get::<usize>(0)?, 2010);
    Ok(())
}

#[rstest]
/// Tests that proxy user authentication works as expected.
fn test_2011() -> Result<(), oracledb::Error> {
    let test_config = oracledb::get_test_config();
    let config =
        test_config.config.clone().set_proxy_user(&test_config.proxy_user);
    let conn = oracledb::connect(config)?;
    let row = conn.query_row(
        r#"
        select
            sys_context('userenv', 'session_user'),
            sys_context('userenv', 'proxy_user')
        from dual
        "#,
        &[],
    )?;
    assert_eq!(row.get::<&str>(0)?, test_config.proxy_user.to_uppercase());
    assert_eq!(row.get::<&str>(1)?, test_config.user.to_uppercase());
    Ok(())
}
