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
use oracledb;
use rstest::*;

#[rstest]
/// test fetching ROWID
fn test_1900(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let mut cursor =
        conn.query("select cast(rowid as varchar2(18)) from dual", &[])?;
    let mut rowid_as_string = String::new();
    for row in cursor {
        let row = row?;
        rowid_as_string = row.get(0)?;
    }
    cursor = conn.query("select rowid from dual", &[])?;
    for row in cursor {
        let row = row?;
        let fetched_val: String = row.get(0)?;
        assert_eq!(fetched_val.to_string(), rowid_as_string);
    }
    Ok(())
}

#[rstest]
/// test ROWID metadata and string representation
fn test_1901(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        "select rowid as rid, cast(rowid as varchar2(20)) from dual",
        &[],
    )?;
    let columns = cursor.columns();
    assert_eq!(columns[0].name(), "RID");
    assert_eq!(columns[0].db_type(), &oracledb::DB_TYPE_ROWID);
    for row in cursor {
        let row = row?;
        let fetched_val: String = row.get(0)?;
        let str_val: String = row.get(1)?;
        assert_eq!(fetched_val, str_val);
    }
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
    let returned = result.returned_data();
    assert_eq!(returned.len(), 1);
    let rowids: Vec<String> = returned[0].get_array(0)?;
    assert_eq!(rowids.len(), 1);
    assert!(!rowids[0].is_empty());
    Ok(())
}
