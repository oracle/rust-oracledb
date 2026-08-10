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
// test_1100_numbers()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;

#[rstest]
/// test lossless conversion to OracleNumber for unsigned integer types
fn test_1100() {
    assert_eq!(oracledb::OracleNumber::from(0u8).to_string(), "0");
    assert_eq!(oracledb::OracleNumber::from(3105u16).to_string(), "3105");
    assert_eq!(oracledb::OracleNumber::from(65536u32).to_string(), "65536");
    assert_eq!(oracledb::OracleNumber::from(65537u64).to_string(), "65537");
    assert_eq!(
        oracledb::OracleNumber::from(100000u128).to_string(),
        "100000"
    );
}

#[test]
/// test lossless conversion to OracleNumber for signed integer types
fn test_1101() {
    assert_eq!(oracledb::OracleNumber::from(-10i8).to_string(), "-10");
    assert_eq!(oracledb::OracleNumber::from(-256i16).to_string(), "-256");
    assert_eq!(
        oracledb::OracleNumber::from(-65000i32).to_string(),
        "-65000"
    );
    assert_eq!(
        oracledb::OracleNumber::from(-68050i64).to_string(),
        "-68050"
    );
    assert_eq!(
        oracledb::OracleNumber::from(-10000i128).to_string(),
        "-10000"
    );
}

#[test]
/// test conversion to OracleNumber from string
fn test_1102() {
    let options = vec![
        ("0", "0"),
        ("00000", "0"),
        ("000001", "1"),
        ("-000008.1", "-8.1"),
        ("-1", "-1"),
        (".1", "0.1"),
        ("0.0", "0"),
        ("0.9", "0.9"),
        ("-.1", "-0.1"),
        (".01", "0.01"),
        (".00502", "0.00502"),
        ("-.0000102", "-0.0000102"),
        ("-15000", "-15000"),
        ("-99.12345", "-99.12345"),
        ("1000.0001", "1000.0001"),
        ("9000.000900", "9000.0009"),
    ];
    for (in_val, out_val) in options {
        let num: oracledb::OracleNumber = in_val.parse().unwrap();
        assert_eq!(num.to_string(), out_val);
    }
}

#[rstest]
/// test binding and fetching OracleNumber
fn test_1103(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let options = vec!["0", "1", "-1", "1.225", "-1.225"];
    for val in options {
        let num: oracledb::OracleNumber = val.parse().unwrap();
        let cursor = conn.query("select :1 from dual", &[&num])?;
        for row in cursor {
            let row = row?;
            let fetched_val: oracledb::OracleNumber = row.get(0)?;
            assert_eq!(fetched_val.to_string(), val);
        }
    }
    Ok(())
}

#[rstest]
/// test fetching duplicate values
fn test_1104(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
        select 1104 from dual
        union all
        select 1104 from dual
        union all
        select 1104 from dual
        union all
        select 1104 from dual
        union all
        select 1104 from dual
        "#,
        &[],
    )?;
    let mut num_rows = 0;
    for row in cursor {
        let row = row?;
        num_rows += 1;
        let fetched_val: oracledb::OracleNumber = row.get(0)?;
        assert_eq!(fetched_val.to_string(), "1104");
    }
    assert_eq!(num_rows, 5);
    Ok(())
}

#[rstest]
/// test fetching null values
fn test_1105(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
        select 1105 from dual
        union all
        select null from dual
        union all
        select 1105 from dual
        union all
        select null from dual
        union all
        select 1105 from dual
        "#,
        &[],
    )?;
    for (row_num, row) in cursor.enumerate() {
        let row = row?;
        let fetched_val: Option<oracledb::OracleNumber> = row.get(0)?;
        if row_num % 2 == 1 {
            assert!(fetched_val.is_none());
        } else {
            assert_eq!(fetched_val.unwrap().to_string(), "1105");
        }
    }
    Ok(())
}

#[rstest]
/// test fetching null values without accepting null values
fn test_1106(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
        select 1106 from dual
        union all
        select null from dual
        "#,
        &[],
    )?;
    for row in cursor {
        let row = row?;
        let res = row.get::<oracledb::OracleNumber>(0);
        match res {
            Ok(val) => {
                assert_eq!(val.to_string(), "1106");
            }
            Err(e) => {
                assert_eq!(e.kind(), &oracledb::ErrorKind::ValueWasNull);
            }
        }
    }
    Ok(())
}

#[rstest]
/// test fetching only null values
fn test_1107(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
        select to_number(null) from dual
        union all
        select to_number(null) from dual
        union all
        select to_number(null) from dual
        union all
        select to_number(null) from dual
        union all
        select to_number(null) from dual
        "#,
        &[],
    )?;
    for row in cursor {
        let row = row?;
        let fetched_val: Option<oracledb::OracleNumber> = row.get(0)?;
        assert!(fetched_val.is_none());
    }
    Ok(())
}

#[rstest]
/// test binding and fetching i32 mapped via NUMBER
fn test_1108(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let val: i32 = 1108;
    let cursor = conn.query("select :1 from dual", &[&val])?;
    for row in cursor {
        let row = row?;
        let fetched: i32 = row.get(0)?;
        assert_eq!(fetched, val);
    }
    Ok(())
}

#[rstest]
/// test i32 boundary values mapped via NUMBER
fn test_1109(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let min_val: i32 = i32::MIN;
    let max_val: i32 = i32::MAX;
    let cursor = conn.query(
        r#"
        select :1 from dual
        union all
        select :2 from dual
        "#,
        &[&min_val, &max_val],
    )?;
    let mut fetched_vals = Vec::new();
    for row in cursor {
        let row = row?;
        let fetched: i32 = row.get(0)?;
        fetched_vals.push(fetched);
    }
    assert_eq!(fetched_vals, vec![min_val, max_val]);
    Ok(())
}

#[rstest]
/// test null handling for i32 mapped via NUMBER
fn test_1110(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let val: Option<i32> = None;
    let cursor = conn.query("select :1 from dual", &[&val])?;
    for row in cursor {
        let row = row?;
        let fetched: Option<i32> = row.get(0)?;
        assert!(fetched.is_none());
    }
    Ok(())
}

#[rstest]
/// test large precision NUMBER (38 digits)
fn test_1111(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        "select to_number('12345678901234567890123456789012345678') from dual",
        &[],
    )?;
    for row in cursor {
        let row = row?;
        let fetched: oracledb::OracleNumber = row.get(0)?;
        assert_eq!(
            fetched.to_string(),
            "12345678901234567890123456789012345678"
        );
    }
    Ok(())
}

#[rstest]
/// test NUMBER with negative scale (rounding)
fn test_1112(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor =
        conn.query("select cast(12345 as number(5,-2)) from dual", &[])?;
    for row in cursor {
        let row = row?;
        let fetched: oracledb::OracleNumber = row.get(0)?;
        assert_eq!(fetched.to_string(), "12300");
    }
    Ok(())
}

#[rstest]
/// test NUMBER precision and scale metadata
fn test_1113(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
        select
            cast(12345 as number(8, 0)) as int_col,
            cast(123.45 as number(8, 2)) as scaled_col,
            cast(12300 as number(8, -2)) as neg_scale_col,
            cast(123.45 as number) as unconstrained_col
        from dual
        "#,
        &[],
    )?;
    let columns = cursor.columns();
    assert_eq!(columns[0].db_type(), &oracledb::DB_TYPE_NUMBER);
    assert_eq!(columns[0].precision(), 8);
    assert_eq!(columns[0].scale(), 0);
    assert_eq!(columns[1].precision(), 8);
    assert_eq!(columns[1].scale(), 2);
    assert_eq!(columns[2].precision(), 8);
    assert_eq!(columns[2].scale(), -2);
    assert_eq!(columns[3].db_type(), &oracledb::DB_TYPE_NUMBER);
    Ok(())
}

#[rstest]
/// test fetching NUMBER into incompatible Rust types returns conversion errors
fn test_1114(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row = conn.query_row("select 1.5 from dual", &[])?;
    let result = row.get::<i32>(0);
    assert!(matches!(
        result.unwrap_err().kind(),
        oracledb::ErrorKind::UnsupportedConversion(_, _)
    ));
    Ok(())
}

#[rstest]
/// test FLOAT and FLOAT(p) fetch as NUMBER-family values
fn test_1115(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
        select
            cast(1.25 as float) as float_col,
            cast(123456789 as float(63)) as float63_col,
            cast(null as float) as null_float_col
        from dual
        "#,
        &[],
    )?;
    let columns = cursor.columns();
    assert_eq!(columns[0].db_type(), &oracledb::DB_TYPE_NUMBER);
    assert_eq!(columns[1].db_type(), &oracledb::DB_TYPE_NUMBER);
    assert_eq!(columns[2].db_type(), &oracledb::DB_TYPE_NUMBER);
    for row in cursor {
        let row = row?;
        let float_col: oracledb::OracleNumber = row.get(0)?;
        let float63_col: oracledb::OracleNumber = row.get(1)?;
        let null_float_col: Option<oracledb::OracleNumber> = row.get(2)?;
        assert_eq!(float_col.to_string(), "1.25");
        assert_eq!(float63_col.to_string(), "123456789");
        assert!(null_float_col.is_none());
    }
    Ok(())
}

#[test]
/// test invalid OracleNumber parse errors
fn test_1116() {
    for value in ["", "abc", "1.2.3", "--1"] {
        let result = value.parse::<oracledb::OracleNumber>();
        assert!(matches!(
            result.unwrap_err().kind(),
            oracledb::ErrorKind::InvalidOracleNumber(_)
        ));
    }
}

#[rstest]
/// test i64 and u64 boundary values mapped via NUMBER
fn test_1117(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let signed_values = [i64::MIN, i64::MAX];
    for value in signed_values {
        let row = conn.query_row("select :1 from dual", &[&value])?;
        let fetched: i64 = row.get(0)?;
        assert_eq!(fetched, value);
    }

    let unsigned_values = [0u64, u64::MAX];
    for value in unsigned_values {
        let row = conn.query_row("select :1 from dual", &[&value])?;
        let fetched: u64 = row.get(0)?;
        assert_eq!(fetched, value);
    }
    Ok(())
}

#[rstest]
/// test integer conversion errors at both sides of the i32 range
fn test_1118(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    for expression in ["2147483648", "-2147483649"] {
        let row =
            conn.query_row(&format!("select {expression} from dual"), &[])?;
        let error = row.get::<i32>(0).unwrap_err();
        assert!(matches!(
            error.kind(),
            oracledb::ErrorKind::UnsupportedConversion(_, _)
        ));
    }
    Ok(())
}
