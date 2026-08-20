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
    let values: Vec<String> = returned_data[0].get_array(0)?;
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

#[rstest]
/// Tests a NULL PL/SQL OUT bind, which must remain distinguishable from zero.
fn test_2705(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let mut result = conn.execute_named(
        "begin :out_value := cast(null as number); end;",
        &[("out_value", &0)],
    )?;
    let returned_data = result.returned_data();
    assert_eq!(returned_data.len(), 1);
    let out_value: Option<i32> = returned_data[0].get(0)?;
    assert!(out_value.is_none());
    Ok(())
}

#[rstest]
/// Tests that a server-side statement error does not poison a later valid
/// statement on the same connection.
fn test_2706(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    assert!(
        conn.query("select no_such_column_2706 from dual", &[])
            .is_err()
    );
    let row = conn.query_row("select 2706 from dual", &[])?;
    let value: i32 = row.get(0)?;
    assert_eq!(value, 2706);
    Ok(())
}

#[rstest]
/// Tests that a duplicate key in execute_batch returns an error and that the
/// connection can be rolled back and reused afterward.
fn test_2707(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_2707",
        "id number primary key, value varchar2(30)",
    )?;
    let first: &[&dyn oracledb::ToDbValue] = &[&1, &"first"];
    let duplicate: &[&dyn oracledb::ToDbValue] = &[&1, &"duplicate"];
    let third: &[&dyn oracledb::ToDbValue] = &[&2, &"third"];
    let params = oracledb::BindParameters::Slice(&[first, duplicate, third]);
    let error = match conn.execute_batch(
        "insert into test_2707 (id, value) values (:1, :2)",
        params,
    ) {
        Ok(_) => panic!("duplicate primary key must fail execute_batch"),
        Err(error) => error,
    };
    assert!(matches!(error.kind(), oracledb::ErrorKind::DbError(_)));
    conn.rollback()?;

    let row = conn.query_row("select count(*) from test_2707", &[])?;
    let count: i32 = row.get(0)?;
    assert_eq!(count, 0);
    Ok(())
}

#[rstest]
/// Tests Oracle's implicit commit when DDL is executed after uncommitted DML.
fn test_2708(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _data_guard = common::create_table(
        &conn,
        "test_2708_data",
        "id number primary key",
    )?;
    let observer = common::conn();
    conn.execute("insert into test_2708_data values (1)", &[])?;

    let row =
        observer.query_row("select count(*) from test_2708_data", &[])?;
    let count: i32 = row.get(0)?;
    assert_eq!(count, 0);

    let _ddl_guard =
        common::create_table(&conn, "test_2708_ddl", "id number")?;
    let row =
        observer.query_row("select count(*) from test_2708_data", &[])?;
    let count: i32 = row.get(0)?;
    assert_eq!(count, 1);
    Ok(())
}

#[rstest]
/// Tests statement execution options, including excluding a statement from
/// the statement cache while fetching in small batches.
fn test_2709(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let mut statement = conn.statement(
        "select level from dual connect by level <= :1 order by level",
    )?;
    statement
        .exclude_from_cache()
        .prefetch_rows(1)
        .fetch_array_size(1);
    let values = statement
        .query(&[&5])?
        .map(|row| row?.get::<i32>(0))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(values, vec![1, 2, 3, 4, 5]);
    Ok(())
}

#[rstest]
/// Tests named and positional bind validation errors without relying on a
/// server-side SQL error.
fn test_2710(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let wrong_count = match conn.query("select :1, :2 from dual", &[&1]) {
        Ok(_) => panic!("an incorrect positional bind count must fail"),
        Err(err) => err,
    };
    assert!(matches!(
        wrong_count.kind(),
        oracledb::ErrorKind::WrongNumPositionalBinds(2, 1)
    ));

    let missing = match conn
        .query_named("select :expected from dual", &[("other", &1)])
    {
        Ok(_) => panic!("a missing named bind must fail"),
        Err(err) => err,
    };
    assert!(matches!(
        missing.kind(),
        oracledb::ErrorKind::MissingBindValue(name) if name == "EXPECTED"
    ));

    let invalid = match conn.query_named(
        "select :expected from dual",
        &[("expected", &1), ("unexpected", &2)],
    ) {
        Ok(_) => panic!("an unknown named bind must fail"),
        Err(err) => err,
    };
    assert!(matches!(
        invalid.kind(),
        oracledb::ErrorKind::InvalidBindName(name) if name == "UNEXPECTED"
    ));
    Ok(())
}

#[rstest]
/// Tests batch validation rejects mixed database types in one bind column.
fn test_2711(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard =
        common::create_table(&conn, "test_2711", "value varchar2(30)")?;
    let first: &[&dyn oracledb::ToDbValue] = &[&"first"];
    let second: &[&dyn oracledb::ToDbValue] = &[&42];
    let params = oracledb::BindParameters::Slice(&[first, second]);
    let err = match conn
        .execute_batch("insert into test_2711 values (:1)", params)
    {
        Ok(_) => panic!("mixed bind types in a batch must fail"),
        Err(err) => err,
    };
    assert!(matches!(
        err.kind(),
        oracledb::ErrorKind::DifferentTypes(_, _)
    ));
    Ok(())
}

#[rstest]
/// Tests no-data, invalid-column-index, and multi-fetch cursor paths.
fn test_2712(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let no_data = match conn.query_row("select 1 from dual where 1 = 0", &[]) {
        Ok(_) => panic!("query_row without rows must fail"),
        Err(err) => err,
    };
    assert!(matches!(no_data.kind(), oracledb::ErrorKind::NoDataFound));

    let row = conn.query_row("select 1 from dual", &[])?;
    let invalid_index = row.get::<i32>(1).unwrap_err();
    assert!(matches!(
        invalid_index.kind(),
        oracledb::ErrorKind::InvalidColumnIndex(1)
    ));

    let mut statement = conn.statement(
        "select level from dual connect by level <= 11 order by level",
    )?;
    statement.prefetch_rows(1).fetch_array_size(1);
    let values = statement
        .query(&[])?
        .map(|row| row?.get::<i32>(0))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(values, (1..=11).collect::<Vec<_>>());
    Ok(())
}

#[rstest]
/// Tests that a cached named statement resizes its bind metadata when a later
/// execution supplies a substantially longer value.
fn test_2713(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement("select length(:value) from dual")?;
    for value in ["x".to_string(), "y".repeat(4000)] {
        let row = statement.query_row_named(&[("value", &value)])?;
        let length: i32 = row.get(0)?;
        assert_eq!(length, value.len() as i32);
    }
    Ok(())
}

#[rstest]
/// Tests that changing the statement options for fetching LOBs is honored,
/// even when the cursor is found in the statement cache.
/// execution supplies a substantially longer value.
fn test_2714(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let sql = "select to_clob(:1) from dual";
    let value = "statement cache LOB option".to_string();
    let mut row = conn.statement(sql)?.fetch_lobs().query_row(&[&value])?;
    let _: oracledb::Lob = row.get(0)?;
    row = conn.query_row(sql, &[&value])?;
    let fetched_value: String = row.get(0)?;
    assert_eq!(fetched_value, value);
    Ok(())
}

#[rstest]
/// Tests DML RETURNING for multiple affected rows and multiple return columns.
fn test_2715(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_2715",
        "id number primary key, value varchar2(30)",
    )?;
    conn.execute("insert into test_2715 values (1, 'one')", &[])?;
    conn.execute("insert into test_2715 values (2, 'two')", &[])?;

    let mut result = conn.execute_named(
        "update test_2715 set value = value || :suffix where id <= :max_id \
         returning id, value into :out_id, :out_value",
        &[
            ("suffix", &"-updated"),
            ("max_id", &2),
            ("out_id", &0),
            ("out_value", &" ".repeat(30)),
        ],
    )?;
    assert_eq!(result.rows_affected(), 2);

    let returned_data = result.returned_data();
    assert_eq!(returned_data.len(), 1);
    let ids: Vec<usize> = returned_data[0].get_array(0)?;
    let values: Vec<String> = returned_data[0].get_array(1)?;
    assert_eq!(ids, vec![1, 2]);
    assert_eq!(values, vec!["one-updated", "two-updated"]);
    Ok(())
}

#[rstest]
/// Tests execution of DML RETURNING when Oracle keywords are adjacent to the
/// surrounding syntax, not separated by whitespace.
fn test_2716(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_2716",
        "id number primary key, value varchar2(30)",
    )?;
    let mut result = conn.execute_named(
        "insert into test_2716 (id, value) values (:in_id, :in_value)\
         returning(value)into :out_value",
        &[
            ("in_id", &1),
            ("in_value", &"no-space-returning"),
            ("out_value", &" ".repeat(30)),
        ],
    )?;
    assert_eq!(result.rows_affected(), 1);

    let returned_data = result.returned_data();
    assert_eq!(returned_data.len(), 1);
    let values: Vec<String> = returned_data[0].get_array(0)?;
    assert_eq!(values, vec!["no-space-returning"]);
    Ok(())
}

#[rstest]
/// Tests DML RETURNING reports an empty returned array when no rows match.
fn test_2717(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_2717",
        "id number primary key, value varchar2(30)",
    )?;
    let mut result = conn.execute_named(
        "update test_2717 set value = :value where id = :id \
         returning value into :out_value",
        &[
            ("value", &"not-written"),
            ("id", &1),
            ("out_value", &" ".repeat(30)),
        ],
    )?;
    assert_eq!(result.rows_affected(), 0);

    let returned_data = result.returned_data();
    assert_eq!(returned_data.len(), 1);
    let values: Vec<String> = returned_data[0].get_array(0)?;
    assert!(values.is_empty());
    Ok(())
}
