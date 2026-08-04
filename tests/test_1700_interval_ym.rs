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
// test_1700_interval_ym()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;

#[rstest]
/// test query with INTERVAL YEAR TO MONTH
fn test_1700(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor =
        conn.query("select to_yminterval('P25Y10M') from dual", &[])?;
    for row in cursor {
        let row = row?;
        let value: oracledb::OracleIntervalYM = row.get(0)?;
        assert_eq!(value.to_string(), "P25Y10M");
    }
    Ok(())
}

#[rstest]
/// test binding INTERVAL YEAR TO MONTH
fn test_1701(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let options = vec![
        oracledb::OracleIntervalYM::new(50, 4),
        oracledb::OracleIntervalYM::new(-25, -8),
    ];
    for val in options {
        let cursor = conn.query("select :1 from dual", &[&val])?;
        for row in cursor {
            let row = row?;
            let fetched_val: oracledb::OracleIntervalYM = row.get(0)?;
            assert_eq!(fetched_val, val);
        }
    }
    Ok(())
}

#[rstest]
/// test fetching null values for INTERVAL YEAR TO MONTH
fn test_1702(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn
        .query("select cast(null as interval year to month) from dual", &[])?;
    for row in cursor {
        let row = row?;
        let fetched_val: Option<oracledb::OracleIntervalYM> = row.get(0)?;
        assert!(fetched_val.is_none());
    }
    Ok(())
}

#[rstest]
/// test zero and negative INTERVAL YEAR TO MONTH literals
fn test_1703(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row = conn.query_row(
        r#"
        select
            interval '0-0' year to month,
            interval '-25-11' year to month
        from dual
        "#,
        &[],
    )?;
    let zero: oracledb::OracleIntervalYM = row.get(0)?;
    let negative: oracledb::OracleIntervalYM = row.get(1)?;
    assert_eq!(zero, oracledb::OracleIntervalYM::new(0, 0));
    assert_eq!(negative, oracledb::OracleIntervalYM::new(-25, -11));
    Ok(())
}
