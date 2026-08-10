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
// test_1800_raw()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;

#[rstest]
/// test binding and fetching RAW
fn test_1800(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let val: Vec<u8> = b"test_1800".to_vec();
    let cursor = conn.query("select :1 from dual", &[&val])?;
    for row in cursor {
        let row = row?;
        let fetched_val: Vec<u8> = row.get(0)?;
        assert_eq!(fetched_val, val);
    }
    Ok(())
}

#[rstest]
/// test fetching null values for RAW
fn test_1801(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query("select cast(null as raw(10)) from dual", &[])?;
    for row in cursor {
        let row = row?;
        let fetched_val: Option<Vec<u8>> = row.get(0)?;
        assert!(fetched_val.is_none());
    }
    Ok(())
}

#[rstest]
/// test zero-length RAW is fetched as null
fn test_1802(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row = conn.query_row("select hextoraw('') from dual", &[])?;
    let fetched_val: Option<Vec<u8>> = row.get(0)?;
    assert!(fetched_val.is_none());
    Ok(())
}

#[rstest]
/// test maximum length RAW fetch
fn test_1803(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let value = vec![0xAB_u8; 2000];
    let row =
        conn.query_row("select cast(:1 as raw(2000)) from dual", &[&value])?;
    let fetched_val: Vec<u8> = row.get(0)?;
    assert_eq!(fetched_val, value);
    Ok(())
}

#[rstest]
/// test fetching LONG RAW values as bytes
fn test_1804(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard =
        common::create_table(&conn, "test_1804", "id number, c long raw")?;
    let short_value = b"short long raw value".to_vec();
    let long_value = vec![0xCD_u8; 40_000];
    conn.execute("insert into test_1804 values (1, :1)", &[&short_value])?;
    conn.execute("insert into test_1804 values (2, :1)", &[&long_value])?;
    conn.execute("insert into test_1804 values (3, null)", &[])?;

    let cursor = conn.query("select c from test_1804 order by id", &[])?;
    let mut values = Vec::<Option<Vec<u8>>>::new();
    for row in cursor {
        values.push(row?.get(0)?);
    }
    assert_eq!(values[0].as_deref(), Some(short_value.as_slice()));
    assert_eq!(values[1].as_deref(), Some(long_value.as_slice()));
    assert!(values[2].is_none());
    Ok(())
}

#[rstest]
/// Tests LONG RAW batch binding with NULL, short, and large values.
fn test_1805(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard =
        common::create_table(&conn, "test_1805", "id number, c long raw")?;
    let null_value: Option<Vec<u8>> = None;
    let short_value = b"short batch value".to_vec();
    let long_value = vec![0xEF_u8; 40_000];
    let first: &[&dyn oracledb::ToDbValue] = &[&1, &null_value];
    let second: &[&dyn oracledb::ToDbValue] = &[&2, &short_value];
    let third: &[&dyn oracledb::ToDbValue] = &[&3, &long_value];
    let params = oracledb::BindParameters::Slice(&[first, second, third]);
    let result = conn.execute_batch(
        "insert into test_1805 (id, c) values (:1, :2)",
        params,
    )?;
    assert_eq!(result.rows_affected(), 3);

    let cursor = conn.query("select c from test_1805 order by id", &[])?;
    let values: Vec<Option<Vec<u8>>> = cursor
        .map(|row| row?.get(0))
        .collect::<Result<Vec<_>, _>>()?;
    assert!(values[0].is_none());
    assert_eq!(values[1].as_deref(), Some(short_value.as_slice()));
    assert_eq!(values[2].as_deref(), Some(long_value.as_slice()));
    Ok(())
}
