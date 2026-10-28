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
// test_1900_rowid()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use rstest::*;

#[rstest]
/// test fetching ROWID
fn test_1900(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row =
        conn.query_row("select cast(rowid as varchar2(18)) from dual", &[])?;
    let rowid_as_string: String = row.get(0)?;
    let row = conn.query_row("select rowid from dual", &[])?;
    assert_eq!(row.get::<String>(0)?, rowid_as_string);
    Ok(())
}

#[rstest]
/// test ROWID metadata and string representation
fn test_1901(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row = conn.query_row(
        "select rowid as rid, cast(rowid as varchar2(20)) from dual",
        &[],
    )?;
    let columns = row.columns();
    assert_eq!(columns[0].name(), "RID");
    assert_eq!(columns[0].db_type(), oracledb::DB_TYPE_ROWID);
    let fetched_val: String = row.get(0)?;
    let str_val: String = row.get(1)?;
    assert_eq!(fetched_val, str_val);
    Ok(())
}

#[rstest]
/// test ROWID can be used to locate and update a row
fn test_1902(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_1903",
        "id number, name varchar2(30)",
    )?;
    conn.execute("insert into test_1903 values (1, 'before')", &[])?;

    let row =
        conn.query_row("select rowid from test_1903 where id = 1", &[])?;
    let rowid: String = row.get(0)?;
    let result = conn.execute(
        "update test_1903 set name = 'after' where rowid = chartorowid(:1)",
        &[&rowid],
    )?;
    assert_eq!(result.rows_affected(), 1);

    let row = conn.query_row(
        "select name from test_1903 where rowid = chartorowid(:1)",
        &[&rowid],
    )?;
    let name: String = row.get(0)?;
    assert_eq!(name, "after");
    Ok(())
}

#[rstest]
/// Tests NULL ROWID conversion and ROWID returned by DML.
fn test_1903(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row = conn.query_row("select cast(null as rowid) from dual", &[])?;
    let null_rowid: Option<String> = row.get(0)?;
    assert!(null_rowid.is_none());

    let _guard = common::create_table(
        &conn,
        "test_1903_returning",
        "value varchar2(30)",
    )?;
    let mut result = conn.execute_named(
        "insert into test_1903_returning (value) values (:value) \
         returning rowid into :out_rowid",
        &[("value", &"rowid value"), ("out_rowid", &" ".repeat(18))],
    )?;
    assert_eq!(result.rows_affected(), 1);
    let returned_row = result.returned_data().into_iter().next().unwrap();
    assert!(!returned_row.get::<&str>(0)?.is_empty());
    Ok(())
}

#[rstest]
/// Verifies an invalid ROWID is surfaced as ORA-01410 rather than a panic.
fn test_1904(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let error = match conn.query_row(
        "select chartorowid(:1) from dual",
        &[&"not-an-oracle-rowid"],
    ) {
        Ok(_) => panic!("an invalid ROWID must be rejected"),
        Err(error) => error,
    };
    assert!(matches!(
        error.kind(),
        oracledb::ErrorKind::DbError(db_error) if db_error.code() == 1410
    ));
    Ok(())
}

#[rstest]
/// Tests null UROWID and UROWID wrapper of a physical rowid.
fn test_1905(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row = conn.query_row("select cast(null as urowid) from dual", &[])?;
    assert_eq!(row.columns()[0].db_type(), oracledb::DB_TYPE_UROWID);
    assert!(row.get::<Option<String>>(0)?.is_none());
    let row = conn.query_row("select rowid from dual", &[])?;
    let rowid: String = row.get(0)?;
    let row = conn.query_row("select cast(rowid as urowid) from dual", &[])?;
    assert_eq!(row.columns()[0].db_type(), oracledb::DB_TYPE_UROWID);
    assert_eq!(row.get::<String>(0)?, rowid);
    Ok(())
}

#[rstest]
/// Tests fetching UROWID
fn test_1906(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table_with_options(
        &conn,
        "test_1906",
        r#"
        int_val number(9) not null,
        string_val varchar2(250) not null,
        date_val date not null,
        constraint test_1906_pk primary key (int_val, string_val, date_val)
        "#,
        "organization index",
    )?;
    let data = oracledb::BindParameters::Slice(&[
        &[&1, &"String #1", &oracledb::OracleTimestamp::new_date(2017, 4, 4)],
        &[&2, &"String #2", &oracledb::OracleTimestamp::new_date(2017, 4, 5)],
        &[
            &3,
            &"3".repeat(249),
            &oracledb::OracleTimestamp::new_date(2017, 4, 6),
        ],
        &[
            &3,
            &"4".repeat(250),
            &oracledb::OracleTimestamp::new_date(2017, 4, 7),
        ],
    ]);
    conn.execute_batch("insert into test_1906 values (:1, :2, :3)", data)?;
    let cursor = conn.query(
        r#"
        select int_val, rowid
        from test_1906
        order by int_val
        "#,
        &[],
    )?;
    for row_result in cursor {
        let row = row_result?;
        let int_val: u8 = row.get(0)?;
        let rowid: String = row.get(1)?;
        let fetched_row = conn.query_row(
            "select int_val from test_1906 where rowid = :1",
            &[&rowid],
        )?;
        assert_eq!(fetched_row.get::<u8>(0)?, int_val);
    }
    Ok(())
}
