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
// test_1100_timestamps()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;

#[rstest]
/// test binding and fetching OracleTimestamp
fn test_1200(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let new_date = oracledb::OracleTimestamp::new_date;
    let new_timestamp = oracledb::OracleTimestamp::new_timestamp;
    let options = vec![
        new_date(2025, 11, 21),
        new_timestamp(1987, 6, 15, 14, 38, 25, 123456789),
    ];
    for value in options {
        let cursor = conn.query("select :1 from dual", &[&value])?;
        for row in cursor {
            let row = row?;
            let fetched_value: oracledb::OracleTimestamp = row.get(0)?;
            assert_eq!(fetched_value, value);
        }
    }
    Ok(())
}

#[rstest]
/// test query with DATE
fn test_1201(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
            select to_date('2025-10-31 09:46:25', 'YYYY-MM-DD HH24:MI:SS')
            from dual
        "#,
        &[],
    )?;
    for row in cursor {
        let row = row?;
        let value: oracledb::OracleTimestamp = row.get(0)?;
        assert_eq!(value.to_string(), "2025-10-31T09:46:25.000000000Z");
    }
    Ok(())
}

#[rstest]
/// test query with TIMESTAMP
fn test_1202(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
            select to_timestamp('2025-10-31 11:39:58.123456789',
                'YYYY-MM-DD HH24:MI:SS.FF9')
            from dual
        "#,
        &[],
    )?;
    for row in cursor {
        let row = row?;
        let value: oracledb::OracleTimestamp = row.get(0)?;
        assert_eq!(value.to_string(), "2025-10-31T11:39:58.123456789Z");
    }
    Ok(())
}

#[rstest]
/// test query with TIMESTAMP WITH LOCAL TIME ZONE
fn test_1203(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
            select cast(to_timestamp('2025-10-31 11:46:01.987654',
                'YYYY-MM-DD HH24:MI:SS.FF6') as timestamp with local time zone)
            from dual
        "#,
        &[],
    )?;
    for row in cursor {
        let row = row?;
        let value: oracledb::OracleTimestamp = row.get(0)?;
        assert_eq!(value.to_string(), "2025-10-31T11:46:01.987654000Z");
    }
    Ok(())
}

#[rstest]
/// test query with TIMESTAMP WITH TIME ZONE
fn test_1204(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
            select cast(to_timestamp_tz('2025-10-31 11:52:15.456 -06:00',
                'YYYY-MM-DD HH24:MI:SS.FF3 TZH:TZM')
                as timestamp with time zone)
            from dual
        "#,
        &[],
    )?;
    for row in cursor {
        let row = row?;
        let value: oracledb::OracleTimestamp = row.get(0)?;
        assert_eq!(value.to_string(), "2025-10-31T17:52:15.456000000-06:00");
    }
    Ok(())
}

#[rstest]
/// test fetching null values for DATE/TIMESTAMP/TIMESTAMP TZ/TIMESTAMP LTZ
fn test_1205(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
        select
            cast(null as date),
            cast(null as timestamp),
            cast(null as timestamp with local time zone),
            cast(null as timestamp with time zone)
        from dual
        "#,
        &[],
    )?;
    for row in cursor {
        let row = row?;
        let v1: Option<oracledb::OracleTimestamp> = row.get(0)?;
        let v2: Option<oracledb::OracleTimestamp> = row.get(1)?;
        let v3: Option<oracledb::OracleTimestamp> = row.get(2)?;
        let v4: Option<oracledb::OracleTimestamp> = row.get(3)?;
        assert!(v1.is_none());
        assert!(v2.is_none());
        assert!(v3.is_none());
        assert!(v4.is_none());
    }
    Ok(())
}

#[rstest]
/// test timestamp fractional second boundaries
fn test_1206(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row = conn.query_row(
        r#"
        select
            timestamp '2026-01-01 00:00:00.000000001',
            timestamp '2026-12-31 23:59:59.999999999'
        from dual
        "#,
        &[],
    )?;
    let low: oracledb::OracleTimestamp = row.get(0)?;
    let high: oracledb::OracleTimestamp = row.get(1)?;
    assert_eq!(low.nanoseconds(), 1);
    assert_eq!(high.nanoseconds(), 999_999_999);
    assert_eq!(high.second(), 59);
    Ok(())
}

#[rstest]
/// test supported DATE lower and upper year boundaries
fn test_1207(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row = conn.query_row(
        r#"
        select
            date '0001-01-01',
            date '9999-12-31'
        from dual
        "#,
        &[],
    )?;
    let low: oracledb::OracleTimestamp = row.get(0)?;
    let high: oracledb::OracleTimestamp = row.get(1)?;
    assert_eq!(low.to_string(), "0001-01-01T00:00:00.000000000Z");
    assert_eq!(high.to_string(), "9999-12-31T00:00:00.000000000Z");
    Ok(())
}

#[rstest]
/// test invalid DATE values return database errors
fn test_1208(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    for value in ["0000-01-01", "10000-01-01"] {
        let sql = format!("select to_date('{value}', 'YYYY-MM-DD') from dual");
        let err = match conn.query(&sql, &[]) {
            Ok(_) => panic!("expected invalid DATE value to fail: {value}"),
            Err(err) => err,
        };
        assert!(matches!(err.kind(), oracledb::ErrorKind::DbError(_)));
    }
    Ok(())
}
