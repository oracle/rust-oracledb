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
// test_3200_arrow_fetch()
//-----------------------------------------------------------------------------

#![cfg(feature = "arrow")]

mod common;

use arrow_array::{
    Array, Decimal128Array, Float64Array, Int8Array, Int16Array, Int32Array,
    Int64Array, LargeBinaryArray, LargeStringArray, StringArray,
    TimestampMicrosecondArray, TimestampMillisecondArray,
    TimestampNanosecondArray, TimestampSecondArray,
};
use arrow_schema::{DataType, TimeUnit};
use rstest::*;

use common::conn;

#[rstest]
/// test fetching query results as Arrow record batches
fn test_3200(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let batch = conn.query_arrow(
        r#"
        select
            cast(1 as number(10, 0)) as ID,
            cast('first' as varchar2(30)) as NOTE,
            cast(1.5 as binary_double) as AMOUNT
        from dual
        union all
        select
            cast(2 as number(10, 0)) as ID,
            cast(null as varchar2(30)) as NOTE,
            cast(2.5 as binary_double) as AMOUNT
        from dual
        union all
        select
            cast(3 as number(10, 0)) as ID,
            cast('third' as varchar2(30)) as NOTE,
            cast(3.5 as binary_double) as AMOUNT
        from dual
        order by ID
        "#,
        oracledb::BindParameters::default(),
    )?;
    assert_eq!(batch.num_rows(), 3);
    let ids = batch.column(0).as_any().downcast_ref::<Int64Array>().unwrap();
    let notes =
        batch.column(1).as_any().downcast_ref::<StringArray>().unwrap();
    let amounts =
        batch.column(2).as_any().downcast_ref::<Float64Array>().unwrap();
    assert_eq!(ids.values(), &[1, 2, 3]);
    assert_eq!(notes.value(0), "first");
    assert!(notes.is_null(1));
    assert_eq!(notes.value(2), "third");
    assert_eq!(amounts.values(), &[1.5, 2.5, 3.5]);
    Ok(())
}

#[rstest]
/// test fetching CLOB, BLOB, and NCLOB values and nulls as large Arrow arrays
fn test_3201(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_3201",
        "id number(10), c_text clob, c_bytes blob, c_ntext nclob",
    )?;
    let text = "arrow clob value".to_string();
    let bytes = b"arrow blob value".to_vec();
    let nclob_text = "NCLOB Unicode: 日本語 🦀";
    conn.execute(
        "insert into test_3201 values (:1, :2, :3, :4)",
        &[&1, &text, &bytes, &nclob_text],
    )?;
    conn.execute("insert into test_3201 (id) values (2)", &[])?;

    let batch = conn.query_arrow(
        "select id, c_text, c_bytes, c_ntext from test_3201 order by id",
        oracledb::BindParameters::default(),
    )?;

    assert_eq!(batch.num_rows(), 2);
    assert_eq!(batch.schema().field(1).data_type(), &DataType::LargeUtf8);
    assert_eq!(batch.schema().field(2).data_type(), &DataType::LargeBinary);
    assert_eq!(batch.schema().field(3).data_type(), &DataType::LargeUtf8);
    assert_eq!(
        batch
            .column(1)
            .as_any()
            .downcast_ref::<LargeStringArray>()
            .unwrap()
            .value(0),
        text
    );
    assert_eq!(
        batch
            .column(2)
            .as_any()
            .downcast_ref::<LargeBinaryArray>()
            .unwrap()
            .value(0),
        bytes.as_slice()
    );
    let nclob_values =
        batch.column(3).as_any().downcast_ref::<LargeStringArray>().unwrap();
    assert_eq!(nclob_values.value(0), nclob_text);
    assert!(batch.column(1).is_null(1));
    assert!(batch.column(2).is_null(1));
    assert!(nclob_values.is_null(1));
    Ok(())
}

#[rstest]
/// test requested Arrow fetch mappings for Oracle NUMBER columns
fn test_3202(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let batch = conn.query_arrow(
        r#"
        select
            cast(1 as number(2, 0)) as C_INT8,
            cast(2 as number(4, 0)) as C_INT16,
            cast(3 as number(9, 0)) as C_INT32,
            cast(4 as number(18, 0)) as C_INT64,
            cast(123.45 as number(10, 2)) as C_DECIMAL,
            cast(9.25 as number) as C_FLOAT
        from dual
        "#,
        oracledb::BindParameters::default(),
    )?;

    assert_eq!(
        batch.column(0).as_any().downcast_ref::<Int8Array>().unwrap().value(0),
        1
    );
    assert_eq!(
        batch
            .column(1)
            .as_any()
            .downcast_ref::<Int16Array>()
            .unwrap()
            .value(0),
        2
    );
    assert_eq!(
        batch
            .column(2)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap()
            .value(0),
        3
    );
    assert_eq!(
        batch
            .column(3)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .value(0),
        4
    );
    let decimals =
        batch.column(4).as_any().downcast_ref::<Decimal128Array>().unwrap();
    assert_eq!(decimals.value(0), 12345);
    assert_eq!(decimals.precision(), 10);
    assert_eq!(decimals.scale(), 2);
    assert_eq!(
        batch
            .column(5)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap()
            .value(0),
        9.25
    );
    Ok(())
}

#[rstest]
/// test conversion of Oracle NUMBER to Array Decimal128
fn test_3203(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_3203",
        "id number(9), value number(9, 4)",
    )?;
    let data = [
        "90000.0000",
        "0.1000",
        "0.0200",
        "0.0030",
        "0.0004",
        "12345.6789",
        "12.5000",
        "234.5570",
        "1.0620",
    ];
    for (index, value) in data.as_slice().iter().enumerate() {
        conn.execute(
            "insert into test_3203 values (:1, to_number(:2))",
            &[&(index + 1), value],
        )?;
    }
    conn.commit()?;
    let batch = conn.query_arrow(
        "select value from test_3203 order by id",
        oracledb::BindParameters::default(),
    )?;
    let array =
        batch.column(0).as_any().downcast_ref::<Decimal128Array>().unwrap();
    let fetched_values: Vec<String> =
        (0..array.len()).map(|i| array.value_as_string(i)).collect();
    assert_eq!(fetched_values, data);
    Ok(())
}

#[rstest]
/// Tests that Arrow fetch rejects a database type without a supported Arrow
/// mapping instead of silently changing its representation.
fn test_3204(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let error = conn
        .query_arrow(
            "select xmltype('<root/>') from dual",
            oracledb::BindParameters::default(),
        )
        .expect_err("XMLTYPE must not be mapped to an arbitrary Arrow type");
    assert!(matches!(
        error.kind(),
        oracledb::ErrorKind::UnsupportedDbType(_)
            | oracledb::ErrorKind::ArrowOperation
    ));
    Ok(())
}

#[rstest]
/// test fetching LONG and LONG RAW columns as LargeUtf8 and LargeBinary
fn test_3205(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _text_guard =
        common::create_table(&conn, "test_3205_long", "id number, c long")?;
    let _bytes_guard =
        common::create_table(&conn, "test_3205_raw", "id number, c long raw")?;
    let text = "arrow long value".to_string();
    let bytes = b"arrow long raw value".to_vec();
    conn.execute("insert into test_3205_long values (:1, :2)", &[&1, &text])?;
    conn.execute("insert into test_3205_raw values (:1, :2)", &[&1, &bytes])?;
    let text_batch = conn.query_arrow(
        "select c from test_3205_long",
        oracledb::BindParameters::default(),
    )?;
    assert_eq!(text_batch.schema().field(0).data_type(), &DataType::LargeUtf8);
    assert_eq!(
        text_batch
            .column(0)
            .as_any()
            .downcast_ref::<LargeStringArray>()
            .unwrap()
            .value(0),
        text
    );
    let bytes_batch = conn.query_arrow(
        "select c from test_3205_raw",
        oracledb::BindParameters::default(),
    )?;
    assert_eq!(
        bytes_batch.schema().field(0).data_type(),
        &DataType::LargeBinary
    );
    assert_eq!(
        bytes_batch
            .column(0)
            .as_any()
            .downcast_ref::<LargeBinaryArray>()
            .unwrap()
            .value(0),
        bytes.as_slice()
    );
    Ok(())
}

#[rstest]
/// test default Arrow timestamp units and values for Oracle timestamp precision
fn test_3206(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let batch = conn.query_arrow(
        r#"
        select cast(timestamp '2026-01-02 03:04:05' as timestamp(0)),
               cast(timestamp '2026-01-02 03:04:05.123' as timestamp(3)),
               cast(timestamp '2026-01-02 03:04:05.123456' as timestamp(6)),
               cast(timestamp '2026-01-02 03:04:05.123456789' as timestamp(9))
        from dual
        "#,
        oracledb::BindParameters::default(),
    )?;
    let units = [
        TimeUnit::Second,
        TimeUnit::Millisecond,
        TimeUnit::Microsecond,
        TimeUnit::Nanosecond,
    ];
    for (index, unit) in units.into_iter().enumerate() {
        assert_eq!(
            batch.schema().field(index).data_type(),
            &DataType::Timestamp(unit, None)
        );
    }
    let expected = chrono::NaiveDate::from_ymd_opt(2026, 1, 2)
        .unwrap()
        .and_hms_nano_opt(3, 4, 5, 123_456_789)
        .unwrap()
        .and_utc();
    assert_eq!(
        batch
            .column(0)
            .as_any()
            .downcast_ref::<TimestampSecondArray>()
            .unwrap()
            .value(0),
        expected.timestamp()
    );
    assert_eq!(
        batch
            .column(1)
            .as_any()
            .downcast_ref::<TimestampMillisecondArray>()
            .unwrap()
            .value(0),
        expected.timestamp_millis()
    );
    assert_eq!(
        batch
            .column(2)
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .unwrap()
            .value(0),
        expected.timestamp_micros()
    );
    assert_eq!(
        batch
            .column(3)
            .as_any()
            .downcast_ref::<TimestampNanosecondArray>()
            .unwrap()
            .value(0),
        expected.timestamp_nanos_opt().unwrap()
    );
    Ok(())
}
