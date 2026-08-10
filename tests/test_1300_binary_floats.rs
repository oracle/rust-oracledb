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
// test_1300_binary_floats()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;

#[rstest]
/// test query with BINARY_FLOAT
fn test_1300(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query("select to_binary_float(3.75) from dual", &[])?;
    for row in cursor {
        let row = row?;
        let value: f32 = row.get(0)?;
        assert_eq!(value, 3.75);
    }
    Ok(())
}

#[rstest]
/// test binding BINARY_FLOAT
fn test_1301(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let val: f32 = 1301.25;
    let cursor = conn.query("select :1 from dual", &[&val])?;
    for row in cursor {
        let row = row?;
        let fetched_val: f32 = row.get(0)?;
        assert_eq!(fetched_val, val);
    }
    Ok(())
}

#[rstest]
/// test fetching null values for BINARY_FLOAT
fn test_1302(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor =
        conn.query("select cast(null as binary_float) from dual", &[])?;
    for row in cursor {
        let row = row?;
        let fetched_val: Option<f32> = row.get(0)?;
        assert!(fetched_val.is_none());
    }
    Ok(())
}

#[rstest]
/// test fetching special BINARY_FLOAT values
fn test_1303(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
        select binary_float_nan from dual
        union all
        select binary_float_infinity from dual
        union all
        select -binary_float_infinity from dual
        "#,
        &[],
    )?;
    let mut values = Vec::new();
    for row in cursor {
        values.push(row?.get::<f32>(0)?);
    }
    assert!(values[0].is_nan());
    assert_eq!(values[1], f32::INFINITY);
    assert_eq!(values[2], f32::NEG_INFINITY);
    Ok(())
}

#[rstest]
/// Tests preservation of the IEEE-754 negative-zero sign bit for BINARY_FLOAT.
fn test_1304(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let value = -0.0_f32;
    let row = conn.query_row("select :1 from dual", &[&value])?;
    let fetched: f32 = row.get(0)?;
    assert_eq!(fetched, 0.0);
    assert_eq!(fetched.to_bits(), value.to_bits());
    Ok(())
}
