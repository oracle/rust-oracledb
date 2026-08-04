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
// test_3100_arrow_bind()
//-----------------------------------------------------------------------------

#![cfg(feature = "arrow")]

mod common;

use std::sync::Arc;

use arrow_array::{
    Array, BinaryArray, BooleanArray, Date32Array, Decimal128Array,
    Float32Array, Float64Array, Int8Array, Int16Array, Int32Array, Int64Array,
    RecordBatch, StringArray, TimestampMicrosecondArray, UInt8Array,
    UInt16Array, UInt32Array, UInt64Array,
};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use rstest::*;

use common::conn;

#[rstest]
/// test binding Arrow arrays without materializing row values
fn test_3100(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_3100",
        "id number(10), amount number(10, 2), note varchar2(30)",
    )?;
    let schema = Arc::new(Schema::new(vec![
        Field::new("ID", DataType::Int32, false),
        Field::new("AMOUNT", DataType::Float64, false),
        Field::new("NOTE", DataType::Utf8, true),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int32Array::from(vec![1, 2, 3])),
            Arc::new(Float64Array::from(vec![12.5, 13.75, 14.0])),
            Arc::new(StringArray::from(vec![
                Some("first"),
                None,
                Some("third"),
            ])),
        ],
    )
    .unwrap();

    let result = conn
        .execute_batch("insert into test_3100 values (:1, :2, :3)", batch)?;
    assert_eq!(result.rows_affected(), 3);

    let cursor = conn.query(
        r#"
        select id, cast(amount as binary_double), note
        from test_3100 order by id
        "#,
        &[],
    )?;
    let mut rows = Vec::new();
    for row in cursor {
        let row = row?;
        rows.push((
            row.get::<i32>(0)?,
            row.get::<f64>(1)?,
            row.get::<Option<String>>(2)?,
        ));
    }
    assert_eq!(
        rows,
        vec![
            (1, 12.5, Some("first".to_string())),
            (2, 13.75, None),
            (3, 14.0, Some("third".to_string())),
        ]
    );
    Ok(())
}

#[rstest]
/// test binding Arrow text and binary arrays as Oracle LOB datatypes
fn test_3101(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_3101",
        "id number(10), c_text clob, c_bytes blob",
    )?;
    let schema = Arc::new(Schema::new(vec![
        Field::new("ID", DataType::Int32, false),
        Field::new("C_TEXT", DataType::Utf8, true),
        Field::new("C_BYTES", DataType::Binary, true),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int32Array::from(vec![1])),
            Arc::new(StringArray::from(vec![Some("arrow clob value")])),
            Arc::new(BinaryArray::from_vec(vec![
                b"arrow blob value".as_slice(),
            ])),
        ],
    )
    .unwrap();
    let result = conn
        .execute_batch("insert into test_3101 values (:1, :2, :3)", batch)?;
    assert_eq!(result.rows_affected(), 1);

    let row = conn.query_row("select c_text, c_bytes from test_3101", &[])?;
    assert_eq!(row.get::<String>(0)?, "arrow clob value");
    assert_eq!(row.get::<Vec<u8>>(1)?, b"arrow blob value");
    Ok(())
}

#[rstest]
/// test querying with Arrow bind rows
fn test_3102(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Int32,
        false,
    )]));
    let batch = RecordBatch::try_new(
        schema,
        vec![Arc::new(Int32Array::from(vec![11, 22, 33]))],
    )
    .unwrap();
    let batch_result =
        conn.query_arrow("select cast(:1 as number(9, 0)) from dual", batch)?;
    let values = batch_result
        .column(0)
        .as_any()
        .downcast_ref::<Int32Array>()
        .unwrap()
        .values()
        .to_vec();

    assert_eq!(values, vec![11, 22, 33]);
    Ok(())
}

#[rstest]
/// test binding Arrow primitive numeric arrays to Oracle NUMBER columns
fn test_3103(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_3103",
        r#"
        c_int8 number(3),
        c_int16 number(5),
        c_int32 number(10),
        c_int64 number(19),
        c_uint8 number(3),
        c_uint16 number(5),
        c_uint32 number(10),
        c_uint64 number(20),
        c_decimal number(10, 2)
        "#,
    )?;
    let schema = Arc::new(Schema::new(vec![
        Field::new("C_INT8", DataType::Int8, false),
        Field::new("C_INT16", DataType::Int16, false),
        Field::new("C_INT32", DataType::Int32, false),
        Field::new("C_INT64", DataType::Int64, false),
        Field::new("C_UINT8", DataType::UInt8, false),
        Field::new("C_UINT16", DataType::UInt16, false),
        Field::new("C_UINT32", DataType::UInt32, false),
        Field::new("C_UINT64", DataType::UInt64, false),
        Field::new("C_DECIMAL", DataType::Decimal128(10, 2), false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int8Array::from(vec![-8i8])),
            Arc::new(Int16Array::from(vec![-1600i16])),
            Arc::new(Int32Array::from(vec![-320_000i32])),
            Arc::new(Int64Array::from(vec![-6_400_000_000i64])),
            Arc::new(UInt8Array::from(vec![8u8])),
            Arc::new(UInt16Array::from(vec![1600u16])),
            Arc::new(UInt32Array::from(vec![320_000u32])),
            Arc::new(UInt64Array::from(vec![6_400_000_000u64])),
            Arc::new(
                Decimal128Array::from(vec![12345i128])
                    .with_data_type(DataType::Decimal128(10, 2)),
            ),
        ],
    )
    .unwrap();

    let result = conn.execute_batch(
        "insert into test_3103 values (:1, :2, :3, :4, :5, :6, :7, :8, :9)",
        batch,
    )?;
    assert_eq!(result.rows_affected(), 1);

    let cursor = conn.query(
        r#"
        select
            c_int8,
            c_int16,
            c_int32,
            c_int64,
            c_uint8,
            c_uint16,
            c_uint32,
            c_uint64,
            cast(c_decimal as binary_double)
        from test_3103
        "#,
        &[],
    )?;
    for row in cursor {
        let row = row?;
        assert_eq!(row.get::<i8>(0)?, -8);
        assert_eq!(row.get::<i16>(1)?, -1600);
        assert_eq!(row.get::<i32>(2)?, -320_000);
        assert_eq!(row.get::<i64>(3)?, -6_400_000_000);
        assert_eq!(row.get::<u8>(4)?, 8);
        assert_eq!(row.get::<u16>(5)?, 1600);
        assert_eq!(row.get::<u32>(6)?, 320_000);
        assert_eq!(row.get::<u64>(7)?, 6_400_000_000);
        assert_eq!(row.get::<f64>(8)?, 123.45);
    }
    Ok(())
}

#[rstest]
/// test binding Arrow scalar, text, binary, and timestamp arrays
fn test_3104(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_3104",
        r#"
        c_bool boolean,
        c_float binary_float,
        c_double binary_double,
        c_text varchar2(30),
        c_raw raw(10),
        c_ts timestamp(6),
        c_date date
        "#,
    )?;
    let schema = Arc::new(Schema::new(vec![
        Field::new("C_BOOL", DataType::Boolean, false),
        Field::new("C_FLOAT", DataType::Float32, false),
        Field::new("C_DOUBLE", DataType::Float64, false),
        Field::new("C_TEXT", DataType::Utf8, true),
        Field::new("C_RAW", DataType::Binary, true),
        Field::new(
            "C_TS",
            DataType::Timestamp(TimeUnit::Microsecond, None),
            false,
        ),
        Field::new("C_DATE", DataType::Date32, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(BooleanArray::from(vec![true])),
            Arc::new(Float32Array::from(vec![1.25f32])),
            Arc::new(Float64Array::from(vec![2.5f64])),
            Arc::new(StringArray::from(vec![Some("arrow")])),
            Arc::new(BinaryArray::from_vec(vec![b"raw".as_slice()])),
            Arc::new(TimestampMicrosecondArray::from(vec![
                1_735_689_600_123_456i64,
            ])),
            Arc::new(Date32Array::from(vec![20_089i32])),
        ],
    )
    .unwrap();
    let result = conn.execute_batch(
        "insert into test_3104 values (:1, :2, :3, :4, :5, :6, :7)",
        batch,
    )?;
    assert_eq!(result.rows_affected(), 1);

    let batch = conn.query_arrow(
        r#"
        select
            c_bool,
            c_float,
            c_double,
            c_text,
            c_raw,
            c_ts,
            c_date
        from test_3104
        "#,
        oracledb::BindParameters::default(),
    )?;

    assert_eq!(batch.num_rows(), 1);
    assert!(
        batch
            .column(0)
            .as_any()
            .downcast_ref::<BooleanArray>()
            .unwrap()
            .value(0)
    );
    assert_eq!(
        batch
            .column(1)
            .as_any()
            .downcast_ref::<Float32Array>()
            .unwrap()
            .value(0),
        1.25
    );
    assert_eq!(
        batch
            .column(2)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap()
            .value(0),
        2.5
    );
    assert_eq!(
        batch
            .column(3)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(0),
        "arrow"
    );
    assert_eq!(
        batch
            .column(4)
            .as_any()
            .downcast_ref::<BinaryArray>()
            .unwrap()
            .value(0),
        b"raw"
    );
    assert_eq!(
        batch
            .column(5)
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .unwrap()
            .value(0),
        1_735_689_600_123_456
    );
    assert_eq!(
        batch
            .column(6)
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .unwrap()
            .value(0),
        1_735_689_600_000_000
    );
    Ok(())
}

#[rstest]
/// test using execute_batch for non-keyed UPDATE statements
fn test_3105(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    // Avoid ORA-12860: deadlock detected while waiting for a sibling row lock.
    conn.execute("alter session disable parallel dml", &[])?;
    conn.execute("alter session disable parallel query", &[])?;

    let _guard = common::create_table(
        &conn,
        "test_3105",
        "id number(9), value number(9)",
    )?;
    conn.execute("insert into test_3105 values (1, 10)", &[])?;
    conn.execute("insert into test_3105 values (2, 20)", &[])?;
    conn.execute("insert into test_3105 values (3, 30)", &[])?;

    let update_sql = "update test_3105 set value = :1 where id = :2";
    let schema = Arc::new(Schema::new(vec![
        Field::new("VALUE", DataType::Int32, false),
        Field::new("ID", DataType::Int32, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int32Array::from(vec![100, 300])),
            Arc::new(Int32Array::from(vec![1, 3])),
        ],
    )
    .unwrap();

    let result = conn.execute_batch(update_sql, batch).unwrap_or_else(|err| {
        panic!("Arrow execute_batch() update failed: {err}")
    });

    assert_eq!(result.rows_affected(), 2);
    let cursor =
        conn.query("select id, value from test_3105 order by id", &[])?;
    let mut rows = Vec::new();
    for row in cursor {
        let row = row?;
        rows.push((row.get::<i32>(0)?, row.get::<i32>(1)?));
    }
    assert_eq!(rows, vec![(1, 100), (2, 20), (3, 300)]);
    Ok(())
}

#[rstest]
/// test using execute_batch() for keyed DELETE statements
fn test_3106(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_3106",
        "id number(9) primary key, value number(9)",
    )?;
    conn.execute("insert into test_3106 values (1, 10)", &[])?;
    conn.execute("insert into test_3106 values (2, 20)", &[])?;
    conn.execute("insert into test_3106 values (3, 30)", &[])?;

    let schema =
        Arc::new(Schema::new(vec![Field::new("ID", DataType::Int32, false)]));
    let batch = RecordBatch::try_new(
        schema,
        vec![Arc::new(Int32Array::from(vec![1, 3]))],
    )
    .unwrap();

    let result =
        conn.execute_batch("delete from test_3106 where id = :1", batch)?;

    assert_eq!(result.rows_affected(), 2);
    assert_eq!(
        conn.query_row("select id from test_3106", &[],)?
            .get::<i32>(0)?,
        2
    );
    Ok(())
}
