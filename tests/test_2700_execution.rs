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
// test_2700_execution()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;

#[rstest]
/// Tests named execute/query/query_row, including bind order and a repeated
/// placeholder.
fn test_2700(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_2700",
        "id number primary key, value varchar2(30)",
    )?;

    let result = conn.execute_named(
        "insert into test_2700 (id, value) values (:id, :value)",
        &[("value", &"first bind"), ("id", &42)],
    )?;
    assert_eq!(result.rows_affected(), 1);
    let result = conn.execute_named(
        "insert into test_2700 (id, value) values (:id, :value)",
        &[("value", &"second bind"), ("id", &91)],
    )?;
    assert_eq!(result.rows_affected(), 1);
    conn.commit()?;

    let row = conn.query_row_named(
        "select value from test_2700 where id = :id and :id = 42",
        &[("id", &42)],
    )?;
    let value: String = row.get(0)?;
    assert_eq!(value, "first bind");

    let cursor = conn.query_named(
        "select id from test_2700 where id > :id order by id",
        &[("id", &5)],
    )?;
    let ids: Vec<i32> = cursor
        .into_iter()
        .map(|row| row?.get::<i32>(0))
        .collect::<Result<Vec<i32>, _>>()?;
    assert_eq!(ids, [42, 91]);
    Ok(())
}

#[rstest]
/// Tests PL/SQL OUT and IN/OUT binds through ExecResult::returned_data().
fn test_2701(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let mut result = conn.execute_named(
        "begin :out_value := :input_value * 2; end;",
        &[("input_value", &21), ("out_value", &0)],
    )?;
    let returned_data = result.returned_data();
    assert_eq!(returned_data.len(), 1);
    let out_value: i32 = returned_data[0].get(0)?;
    assert_eq!(out_value, 42);
    assert!(result.returned_data().is_empty());

    let mut result =
        conn.execute("begin :1 := :1 || :2; end;", &[&"value", &"-updated"])?;
    let returned_data = result.returned_data();
    let value: String = returned_data[0].get(0)?;
    assert_eq!(value, "value-updated");
    Ok(())
}

#[rstest]
/// Tests DML RETURNING and the returned data shape for a single affected row.
fn test_2702(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_2702",
        "id number primary key, value varchar2(30)",
    )?;

    let out_value = " ".repeat(30);
    let mut result = conn.execute_named(
        "insert into test_2702 (id, value) values (:id, :value) \
         returning value into :out_value",
        &[
            ("id", &1),
            ("value", &"returned value"),
            ("out_value", &out_value),
        ],
    )?;
    conn.commit()?;
    assert_eq!(result.rows_affected(), 1);
    let returned_data = result.returned_data();
    assert_eq!(returned_data.len(), 1);
    let values: Vec<String> = returned_data[0].get(0)?;
    assert_eq!(values, vec!["returned value"]);
    Ok(())
}

#[rstest]
/// Tests batch DML total row count and the inserted values.
fn test_2703(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_2703",
        "id number primary key, value varchar2(30)",
    )?;

    let params = oracledb::BindParameters::Slice(&[
        &[&1, &"one"],
        &[&2, &"two"],
        &[&3, &"three"],
    ]);
    let result = conn.execute_batch(
        "insert into test_2703 (id, value) values (:1, :2)",
        params,
    )?;
    conn.commit()?;
    assert_eq!(result.rows_affected(), 3);

    let row = conn.query_row("select count(*) from test_2703", &[])?;
    let count: i32 = row.get(0)?;
    assert_eq!(count, 3);
    Ok(())
}

#[rstest]
/// Tests commit and rollback visibility from an independent connection.
fn test_2704(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_2704",
        "id number primary key, value varchar2(30)",
    )?;
    let observer = common::conn();

    let row_count =
        |c: &oracledb::Connection| -> Result<i32, oracledb::Error> {
            let row = c.query_row("select count(*) from test_2704", &[])?;
            row.get(0)
        };

    conn.execute("insert into test_2704 values (1, 'committed')", &[])?;
    assert_eq!(row_count(&observer)?, 0);
    conn.commit()?;
    assert_eq!(row_count(&observer)?, 1);

    conn.execute("insert into test_2704 values (2, 'rolled back')", &[])?;
    assert_eq!(row_count(&conn)?, 2);
    conn.rollback()?;
    assert_eq!(row_count(&observer)?, 1);
    Ok(())
}
