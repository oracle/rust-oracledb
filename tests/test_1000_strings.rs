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
        conn.query("select cast('test_1001' as char(9)) from dual", &[])?;
    for row in cursor {
        let row = row?;
        let value: String = row.get(0)?;
        assert_eq!(value, "test_1001");
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

#[rstest]
/// Verifies supplementary Unicode survives VARCHAR2, NVARCHAR2, CLOB and
/// NCLOB bind/fetch paths.
fn test_1012(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let value = "Rust 🦀 東京 e\u{301}";
    let row = conn.query_row(
        r#"
        select
            cast(:1 as varchar2(100 char)),
            cast(:2 as nvarchar2(100)),
            to_clob(:3),
            to_nclob(:4)
        from dual
        "#,
        &[&value, &value, &value, &value],
    )?;
    for index in 0..4 {
        assert_eq!(row.get::<String>(index)?, value);
    }
    Ok(())
}

#[rstest]
/// Tests a string bound as CHAR with BindValue is compared with blank-padded
/// semantics, unlike the same string bound as VARCHAR2.
fn test_1013(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_1013", "c char(5)")?;
    conn.execute("insert into test_1013 values ('ab')", &[])?;
    let sql = "select count(*) from test_1013 where c = :1";
    let count: i32 = conn.query_row(sql, &[&"ab"])?.get(0)?;
    assert_eq!(count, 0);
    let value = oracledb::BindValue::new(oracledb::DB_TYPE_CHAR, "ab")?;
    let count: i32 = conn.query_row(sql, &[&value])?.get(0)?;
    assert_eq!(count, 1);
    Ok(())
}

#[rstest]
/// Tests strings with 2-, 3- and 4-byte UTF-8 characters bound as NVARCHAR2
/// and NCHAR with BindValue reach the database in the national character set
/// and round-trip through a table and an IN/OUT bind.
fn test_1014(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let value = "ç€😀";
    let nvarchar =
        oracledb::BindValue::new(oracledb::DB_TYPE_NVARCHAR, value)?;
    let nchar = oracledb::BindValue::new(oracledb::DB_TYPE_NCHAR, value)?;

    // the database receives the characters themselves, in AL16UTF16
    let row = conn.query_row_named(
        r#"
        select
            dump(:nv, 1016),
            dump(:nc, 1016),
            case when :nv = unistr('\00E7\20AC\D83D\DE00') then 1 else 0 end
        from dual
        "#,
        &[("nv", &nvarchar), ("nc", &nchar)],
    )?;
    let encoded = "Len=8 CharacterSet=AL16UTF16: 0,e7,20,ac,d8,3d,de,0";
    assert_eq!(row.get::<String>(0)?, format!("Typ=1 {encoded}"));
    assert_eq!(row.get::<String>(1)?, format!("Typ=96 {encoded}"));
    assert_eq!(row.get::<i32>(2)?, 1);

    // the value round-trips through NVARCHAR2 and NCHAR columns
    let _guard = common::create_table(
        &conn,
        "test_1014",
        "nv nvarchar2(10), nc nchar(10)",
    )?;
    conn.execute(
        "insert into test_1014 values (:1, :2)",
        &[&nvarchar, &nchar],
    )?;
    let row = conn.query_row("select nv, nc from test_1014", &[])?;
    assert_eq!(row.get::<String>(0)?, value);
    assert_eq!(row.get::<String>(1)?.trim_end(), value);

    // and through an IN/OUT bind
    let mut result = conn
        .execute("begin :1 := :1 || unistr('\\00E7'); end;", &[&nvarchar])?;
    let out_value: String = result.out_bind_data().get(0)?;
    assert_eq!(out_value, format!("{value}ç"));
    Ok(())
}
