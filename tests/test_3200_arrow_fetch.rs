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
    Array, BinaryArray, Decimal128Array, Float64Array, Int8Array, Int16Array,
    Int32Array, Int64Array, StringArray,
};
use arrow_schema::DataType;
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
    let ids = batch
        .column(0)
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap();
    let notes = batch
        .column(1)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let amounts = batch
        .column(2)
        .as_any()
        .downcast_ref::<Float64Array>()
        .unwrap();
    assert_eq!(ids.values(), &[1, 2, 3]);
    assert_eq!(notes.value(0), "first");
    assert!(notes.is_null(1));
    assert_eq!(notes.value(2), "third");
    assert_eq!(amounts.values(), &[1.5, 2.5, 3.5]);
    Ok(())
}

#[rstest]
/// test fetching Oracle LOB values as Arrow string and binary arrays
fn test_3201(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_3201",
        "id number(10), c_text clob, c_bytes blob",
    )?;
    let text = "arrow clob value".to_string();
    let bytes = b"arrow blob value".to_vec();
    conn.execute(
        "insert into test_3201 values (:1, :2, :3)",
        &[&1, &text, &bytes],
    )?;

    let batch = conn.query_arrow(
        "select id, c_text, c_bytes from test_3201",
        oracledb::BindParameters::default(),
    )?;

    assert_eq!(batch.schema().field(1).data_type(), &DataType::Utf8);
    assert_eq!(batch.schema().field(2).data_type(), &DataType::Binary);
    assert_eq!(
        batch
            .column(1)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(0),
        text
    );
    assert_eq!(
        batch
            .column(2)
            .as_any()
            .downcast_ref::<BinaryArray>()
            .unwrap()
            .value(0),
        bytes.as_slice()
    );
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
        batch
            .column(0)
            .as_any()
            .downcast_ref::<Int8Array>()
            .unwrap()
            .value(0),
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
    let decimals = batch
        .column(4)
        .as_any()
        .downcast_ref::<Decimal128Array>()
        .unwrap();
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
    for (index, value) in data.as_slice().into_iter().enumerate() {
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
    let array = batch
        .column(0)
        .as_any()
        .downcast_ref::<Decimal128Array>()
        .unwrap();
    let fetched_values: Vec<String> =
        (0..array.len()).map(|i| array.value_as_string(i)).collect();
    assert_eq!(fetched_values, data);
    Ok(())
}
