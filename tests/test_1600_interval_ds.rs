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
// test_1600_interval_ds()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;

#[rstest]
/// test query with INTERVAL DAY TO SECOND
fn test_1600(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor =
        conn.query("select to_dsinterval('P5DT3H4M6S') from dual", &[])?;
    for row in cursor {
        let row = row?;
        let value: oracledb::OracleIntervalDS = row.get(0)?;
        assert_eq!(value.to_string(), "P5DT3H4M6.000000000S");
    }
    Ok(())
}

#[rstest]
/// test binding INTERVAL DAY TO SECOND
fn test_1601(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let options = vec![
        oracledb::OracleIntervalDS::new(5, 4, 3, 2, 123456789),
        oracledb::OracleIntervalDS::new(-325, -8, -5, -30, -987654321),
    ];
    for val in options {
        let cursor = conn.query("select :1 from dual", &[&val])?;
        for row in cursor {
            let row = row?;
            let fetched_val: oracledb::OracleIntervalDS = row.get(0)?;
            assert_eq!(fetched_val, val);
        }
    }
    Ok(())
}

#[rstest]
/// test fetching null values for INTERVAL DAY TO SECOND
fn test_1602(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn
        .query("select cast(null as interval day to second) from dual", &[])?;
    for row in cursor {
        let row = row?;
        let fetched_val: Option<oracledb::OracleIntervalDS> = row.get(0)?;
        assert!(fetched_val.is_none());
    }
    Ok(())
}

#[rstest]
/// test zero and negative INTERVAL DAY TO SECOND literals
fn test_1603(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row = conn.query_row(
        r#"
        select
            interval '0 00:00:00.000000000' day to second(9),
            interval '-12 03:04:05.123456789' day to second(9)
        from dual
        "#,
        &[],
    )?;
    let zero: oracledb::OracleIntervalDS = row.get(0)?;
    let negative: oracledb::OracleIntervalDS = row.get(1)?;
    assert_eq!(zero, oracledb::OracleIntervalDS::new(0, 0, 0, 0, 0));
    assert_eq!(
        negative,
        oracledb::OracleIntervalDS::new(-12, -3, -4, -5, -123_456_789)
    );
    Ok(())
}
