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
// test_1000_strings()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;

#[rstest]
/// test query with VARCHAR2
fn test_1000(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn
        .query("select cast('test_1006' as varchar2(100)) from dual", &[])?;
    for row in cursor {
        let row = row?;
        let value: String = row.get(0)?;
        assert_eq!(value, "test_1006");
    }
    Ok(())
}

#[rstest]
/// test query with CHAR
fn test_1001(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor =
        conn.query("select cast('test_1007' as char(9)) from dual", &[])?;
    for row in cursor {
        let row = row?;
        let value: String = row.get(0)?;
        assert_eq!(value, "test_1007");
    }
    Ok(())
}

#[rstest]
/// test query with NCHAR
fn test_1002(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor =
        conn.query("select cast('test_1008' as nchar(9)) from dual", &[])?;
    for row in cursor {
        let row = row?;
        let value: String = row.get(0)?;
        assert_eq!(value, "test_1008");
    }
    Ok(())
}

#[rstest]
/// test query with NVARCHAR2
fn test_1003(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn
        .query("select cast('test_1009' as nvarchar2(100)) from dual", &[])?;
    for row in cursor {
        let row = row?;
        let value: String = row.get(0)?;
        assert_eq!(value, "test_1009");
    }
    Ok(())
}

#[rstest]
/// test binding and fetching VARCHAR2
fn test_1004(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let val = "test_1004".to_string();
    let cursor = conn.query("select :1 from dual", &[&val])?;
    for row in cursor {
        let row = row?;
        let fetched_val: String = row.get(0)?;
        assert_eq!(fetched_val, val);
    }
    Ok(())
}

#[rstest]
/// test binding and fetching data that exceeds 32,767 bytes (aka LONG)
fn test_1005(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_1005", "c clob")?;
    let val = String::from("ABCDEFGHIJ").repeat(4096);
    conn.execute("insert into test_1005 values (:1)", &[&val])?;
    let cursor = conn.query("select c from test_1005", &[])?;
    for row in cursor {
        let row = row?;
        let fetched_val: String = row.get(0)?;
        assert_eq!(fetched_val, val);
    }
    Ok(())
}

#[rstest]
/// test fetching null values for VARCHAR2/CHAR/NCHAR/NVARCHAR2
fn test_1006(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
        select
            cast(null as varchar2(10)),
            cast(null as char(10)),
            cast(null as nchar(10)),
            cast(null as nvarchar2(10))
        from dual
        "#,
        &[],
    )?;
    for row in cursor {
        let row = row?;
        let v1: Option<String> = row.get(0)?;
        let v2: Option<String> = row.get(1)?;
        let v3: Option<String> = row.get(2)?;
        let v4: Option<String> = row.get(3)?;
        assert!(v1.is_none());
        assert!(v2.is_none());
        assert!(v3.is_none());
        assert!(v4.is_none());
    }
    Ok(())
}

#[rstest]
/// test max-length handling for VARCHAR2 (4000) and NVARCHAR2 (2000)
fn test_1007(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let max_varchar = "A".repeat(4000);
    let max_nvarchar = "B".repeat(2000);
    let cursor = conn.query(
        r#"
        select
            cast(:1 as varchar2(4000)),
            cast(:2 as nvarchar2(2000))
        from dual
        "#,
        &[&max_varchar, &max_nvarchar],
    )?;
    for row in cursor {
        let row = row?;
        let v1: String = row.get(0)?;
        let v2: String = row.get(1)?;
        assert_eq!(v1, max_varchar);
        assert_eq!(v2, max_nvarchar);
    }
    Ok(())
}

#[rstest]
/// test fixed-width handling for CHAR/NCHAR
fn test_1008(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let base = "fixed_width";
    let cursor = conn.query(
        r#"
        select
            cast(:1 as char(100)),
            cast(:2 as nchar(100))
        from dual
        "#,
        &[&base, &base],
    )?;
    for row in cursor {
        let row = row?;
        let c1: String = row.get(0)?;
        let c2: String = row.get(1)?;
        assert_eq!(c1.trim_end(), base);
        assert_eq!(c2.trim_end(), base);
        assert!(c1.len() == 100);
        assert!(c2.len() == 100);
    }
    Ok(())
}

#[rstest]
/// test empty strings are fetched as nulls for string data types
fn test_1009(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row = conn.query_row(
        r#"
        select
            cast('' as varchar2(10)),
            cast('' as char(10)),
            cast(n'' as nvarchar2(10)),
            cast(n'' as nchar(10))
        from dual
        "#,
        &[],
    )?;
    let v1: Option<String> = row.get(0)?;
    let v2: Option<String> = row.get(1)?;
    let v3: Option<String> = row.get(2)?;
    let v4: Option<String> = row.get(3)?;
    assert!(v1.is_none());
    assert!(v2.is_none());
    assert!(v3.is_none());
    assert!(v4.is_none());
    Ok(())
}

#[rstest]
/// test Unicode data round-trips through NCHAR/NVARCHAR2
fn test_1010(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let value = "Grüße 東京";
    let row = conn.query_row(
        r#"
        select
            cast(n'Grüße 東京' as nvarchar2(100)),
            cast(n'Grüße 東京' as nchar(100))
        from dual
        "#,
        &[],
    )?;
    let nvarchar_value: String = row.get(0)?;
    let nchar_value: String = row.get(1)?;
    assert_eq!(nvarchar_value, value);
    assert_eq!(nchar_value.trim_end(), value);
    Ok(())
}

#[rstest]
/// test fetching LONG values as strings
fn test_1011(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard =
        common::create_table(&conn, "test_1011", "id number, c long")?;
    let short_value = "short long value";
    let long_value = "long-value-".repeat(4000);
    conn.execute("insert into test_1011 values (1, :1)", &[&short_value])?;
    conn.execute("insert into test_1011 values (2, :1)", &[&long_value])?;
    conn.execute("insert into test_1011 values (3, null)", &[])?;

    let cursor = conn.query("select c from test_1011 order by id", &[])?;
    let mut values = Vec::<Option<String>>::new();
    for row in cursor {
        values.push(row?.get(0)?);
    }
    assert_eq!(values[0].as_deref(), Some(short_value));
    assert_eq!(values[1].as_deref(), Some(long_value.as_str()));
    assert!(values[2].is_none());
    Ok(())
}
