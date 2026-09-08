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

use std::io::Read;

mod common;

use common::conn;
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
/// Tests PL/SQL OUT and IN/OUT binds through ExecResult::out_bind_data().
fn test_2701(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    for value in [100, 200, 300] {
        let mut result = conn.execute_named(
            "begin :out_value := :input_value * 2; end;",
            &[("input_value", &value), ("out_value", &0)],
        )?;
        let out_bind_data = result.out_bind_data();
        assert_eq!(out_bind_data.get::<i32>(0)?, value * 2);
    }
    let mut result =
        conn.execute("begin :1 := :1 || :2; end;", &[&"value", &"-updated"])?;
    let value: String = result.out_bind_data().get(0)?;
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
    let values: Vec<String> = returned_data
        .into_iter()
        .map(|v| v.get(0).unwrap())
        .collect();
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
    let out_value: Option<i32> = result.out_bind_data().get(0)?;
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
    let _: oracledb::Lob = row.take(0)?;
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
    let ids: Vec<usize> =
        returned_data.iter().map(|r| r.get(0).unwrap()).collect();
    let values: Vec<String> =
        returned_data.iter().map(|r| r.get(1).unwrap()).collect();
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

    let values: Vec<String> = result
        .returned_data()
        .into_iter()
        .map(|r| r.get(0).unwrap())
        .collect();
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

    let values: Vec<String> = result
        .returned_data()
        .into_iter()
        .map(|r| r.get(0).unwrap())
        .collect();
    assert!(values.is_empty());
    Ok(())
}

#[rstest]
/// Tests named column lookup with Row::get() and Row::take()
fn test_2718(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    // named lookups are case insensitive
    let row = conn.query_row(
        "select 'test_2718' as test_name, 2718 as test_number from dual",
        &[],
    )?;
    assert_eq!(row.get::<&str>("test_name")?, "test_2718");
    assert_eq!(row.get::<i32>("test_number")?, 2718);
    assert_eq!(row.get::<&str>("test_Name")?, "test_2718");
    assert_eq!(row.get::<i32>("test_Number")?, 2718);
    assert_eq!(row.get::<&str>("TEST_NAME")?, "test_2718");
    assert_eq!(row.get::<i32>("TEST_NUMBER")?, 2718);

    // invalid indexes result in an error
    assert!(row.get::<&str>(99).is_err());
    assert!(row.get::<&str>("NO-EXISTS").is_err());

    // duplicate column names (first match wins)
    let row = conn.query_row(
        "select 'first' as name, 'second' as name from dual",
        &[],
    )?;
    assert_eq!(row.get::<&str>("name")?, "first");

    // named lookup on nested cursor with Row::take()
    let mut row = conn.query_row(
        r#"
        select
            cursor(
                select level
                from dual
                connect by level <= 3
            ) as nested_cur
        from dual
        "#,
        &[],
    )?;
    let cursor: oracledb::Cursor = row.take("nested_cur")?;
    let values: Vec<i32> = cursor
        .map(|row| row?.get("level"))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(values, vec![1, 2, 3]);
    Ok(())
}

#[rstest]
/// Tests quoted named bind (valid and invalid and mixed with unquoted binds)
fn test_2719(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let result = conn.query_named("select :missing from dual", &[("\"", &0)]);
    assert!(result.is_err());
    let row = conn.query_row_named(
        "select :\"TeSt_2719\" from dual",
        &[("\"TeSt_2719\"", &2719)],
    )?;
    assert_eq!(row.get::<usize>(0)?, 2719);
    let row = conn.query_row_named(
        "select :TeSt_2719a, :\"TeSt_2719b\" from dual",
        &[("test_2719a", &"test_2719a"), ("\"TeSt_2719b\"", &2719)],
    )?;
    assert_eq!(row.get::<&str>(0)?, "test_2719a");
    assert_eq!(row.get::<usize>(1)?, 2719);
    Ok(())
}

#[rstest]
/// Tests binding order when long columns are detected.
fn test_2720(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_2720",
        "id number, c clob, v varchar2(30)",
    )?;
    let big_data = "x".repeat(40_000);
    let small_data = "small_value";
    conn.execute(
        "insert into test_2720 values (:1, :2, :3)",
        &[&1, &big_data, &small_data],
    )?;
    conn.commit()?;
    let row = conn.query_row("select * from test_2720", &[])?;
    assert_eq!(row.get::<usize>("id")?, 1);
    assert_eq!(row.get::<&str>("c")?, big_data);
    assert_eq!(row.get::<&str>("v")?, small_data);
    Ok(())
}

#[rstest]
/// Tests mixed regular and pending values across execute prefetch and fetch.
fn test_2721(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_2721",
        "id number primary key, data1 blob, data2 blob",
    )?;
    let payloads = [
        (vec![1, 2, 3], vec![4, 5, 6]),
        (vec![7, 8, 9], vec![10, 11, 12]),
        (vec![13, 14, 15], vec![16, 17, 18]),
    ];
    for (index, (payload1, payload2)) in payloads.iter().enumerate() {
        let id = (index + 1) as i32;
        conn.execute(
            "insert into test_2721 values (:1, :2, :3)",
            &[&id, payload1, payload2],
        )?;
    }
    let mut statement = conn.statement(
        r#"
        select id, data1, id + 20, data2, cursor(select 99 from dual)
        from test_2721
        order by id
        "#,
    )?;
    statement.prefetch_rows(1).fetch_array_size(1).fetch_lobs();
    let cursor = statement.query(&[])?;
    for (index, row) in cursor.enumerate() {
        let mut row = row?;
        let id = (index + 1) as i32;
        assert_eq!(row.get::<i32>(0)?, id);
        assert_eq!(row.get::<i32>(2)?, id + 20);
        let mut lob1: oracledb::Lob = row.take(1)?;
        let mut lob2: oracledb::Lob = row.take(3)?;
        let nested_cursor: oracledb::Cursor = row.take(4)?;
        let mut data1 = Vec::new();
        let mut data2 = Vec::new();
        lob1.read_to_end(&mut data1)?;
        lob2.read_to_end(&mut data2)?;
        assert_eq!(data1, payloads[index].0);
        assert_eq!(data2, payloads[index].1);
        let values: Vec<i32> = nested_cursor
            .map(|row| row?.get(0))
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(values, vec![99]);
    }
    Ok(())
}

#[rstest]
/// Tests PL/SQL OUT binds returned from each execute_batch invocation.
fn test_2722(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let params = oracledb::BindParameters::Slice(&[
        &[&0, &100],
        &[&0, &200],
        &[&0, &300],
    ]);
    let mut result = conn.execute_batch("begin :2 := :1 * 2; end;", params)?;

    let out_bind_data = result.out_bind_data();
    let values: Vec<i32> = out_bind_data
        .iter()
        .map(|row| row.get(0).unwrap())
        .collect();
    assert_eq!(values, vec![200, 400, 600]);
    Ok(())
}

#[rstest]
/// Tests DML RETURNING data grouped by execute_batch invocation.
fn test_2723(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_2723",
        "id number primary key, value varchar2(30)",
    )?;

    conn.execute("insert into test_2723 values (1, 'one')", &[])?;
    conn.execute("insert into test_2723 values (2, 'two')", &[])?;
    conn.execute("insert into test_2723 values (3, 'three')", &[])?;
    let params = oracledb::BindParameters::Slice(&[
        &[&"-first", &1, &0],
        &[&"-second", &2, &0],
        &[&"-third", &3, &0],
    ]);
    let mut result = conn.execute_batch(
        r#"
        update test_2723
            set value = value || :1
        where id <= :2
        returning id
        into :3
        "#,
        params,
    )?;
    conn.commit()?;

    let returned_data = result.returned_data();
    let ids: Vec<Vec<usize>> = returned_data
        .iter()
        .map(|rows| rows.iter().map(|row| row.get(0).unwrap()).collect())
        .collect();
    assert_eq!(ids, vec![vec![1], vec![1, 2], vec![1, 2, 3]]);
    Ok(())
}
