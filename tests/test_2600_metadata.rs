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
// test_2600_metadata()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;

fn assert_column(
    column: &oracledb::Metadata,
    name: &str,
    db_type: &'static oracledb::DbType,
) {
    assert_eq!(column.name(), name);
    assert_eq!(column.db_type(), db_type);
}

#[rstest]
/// test metadata for supported scalar data types
fn test_2600(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
        select
            cast('abc' as varchar2(30)) as varchar_col,
            cast('abc' as char(5)) as char_col,
            cast(n'abc' as nvarchar2(30)) as nvarchar_col,
            cast(n'abc' as nchar(5)) as nchar_col,
            cast(123.45 as number) as number_col,
            cast(1.25 as binary_float) as binary_float_col,
            cast(1.25 as binary_double) as binary_double_col,
            cast(date '2024-02-29' as date) as date_col,
            cast(timestamp '2024-02-29 23:59:59.123456789' as timestamp)
                as timestamp_col,
            cast(
                timestamp '2024-02-29 23:59:59.123456789 +05:30'
                as timestamp with time zone
            ) as timestamp_tz_col,
            cast(
                timestamp '2024-02-29 23:59:59.123456789'
                as timestamp with local time zone
            ) as timestamp_ltz_col,
            interval '12 01:02:03.123456789' day to second
                as interval_ds_col,
            interval '10-11' year to month as interval_ym_col,
            cast(hextoraw('DEADBEEF') as raw(4)) as raw_col
        from dual
        where 1 = 0
        "#,
        &[],
    )?;

    let columns = cursor.columns();
    assert_column(&columns[0], "VARCHAR_COL", &oracledb::DB_TYPE_VARCHAR);
    assert_eq!(columns[0].max_size(), 30);
    assert_column(&columns[1], "CHAR_COL", &oracledb::DB_TYPE_CHAR);
    assert_eq!(columns[1].max_size(), 5);
    assert_column(&columns[2], "NVARCHAR_COL", &oracledb::DB_TYPE_NVARCHAR);
    assert_eq!(columns[2].max_size(), 30);
    assert_column(&columns[3], "NCHAR_COL", &oracledb::DB_TYPE_NCHAR);
    assert_eq!(columns[3].max_size(), 5);
    assert_column(&columns[4], "NUMBER_COL", &oracledb::DB_TYPE_NUMBER);
    assert_column(
        &columns[5],
        "BINARY_FLOAT_COL",
        &oracledb::DB_TYPE_BINARY_FLOAT,
    );
    assert_column(
        &columns[6],
        "BINARY_DOUBLE_COL",
        &oracledb::DB_TYPE_BINARY_DOUBLE,
    );
    assert_column(&columns[7], "DATE_COL", &oracledb::DB_TYPE_DATE);
    assert_column(&columns[8], "TIMESTAMP_COL", &oracledb::DB_TYPE_TIMESTAMP);
    assert_column(
        &columns[9],
        "TIMESTAMP_TZ_COL",
        &oracledb::DB_TYPE_TIMESTAMP_TZ,
    );
    assert_column(
        &columns[10],
        "TIMESTAMP_LTZ_COL",
        &oracledb::DB_TYPE_TIMESTAMP_LTZ,
    );
    assert_column(
        &columns[11],
        "INTERVAL_DS_COL",
        &oracledb::DB_TYPE_INTERVAL_DS,
    );
    assert_column(
        &columns[12],
        "INTERVAL_YM_COL",
        &oracledb::DB_TYPE_INTERVAL_YM,
    );
    assert_column(&columns[13], "RAW_COL", &oracledb::DB_TYPE_RAW);
    assert_eq!(columns[13].max_size(), 4);
    Ok(())
}

#[rstest]
/// test metadata for supported large LOB data types
fn test_2601(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let sql = r#"
        select
            to_clob('abc') as clob_col,
            to_nclob('abc') as nclob_col,
            to_blob(hextoraw('DEADBEEF')) as blob_col
        from dual
        where 1 = 0
        "#;

    let cursor = conn.query(sql, &[])?;
    let columns = cursor.columns();
    assert_eq!(columns[0].db_type(), &oracledb::DB_TYPE_LONG);
    assert_eq!(columns[1].db_type(), &oracledb::DB_TYPE_LONG_NVARCHAR);
    assert_eq!(columns[2].db_type(), &oracledb::DB_TYPE_LONG_RAW);

    let cursor = conn.statement(sql)?.fetch_lobs().query(&[])?;
    let columns = cursor.columns();
    assert_eq!(columns[0].db_type(), &oracledb::DB_TYPE_CLOB);
    assert_eq!(columns[1].db_type(), &oracledb::DB_TYPE_NCLOB);
    assert_eq!(columns[2].db_type(), &oracledb::DB_TYPE_BLOB);
    Ok(())
}

#[rstest]
/// test metadata for mapped but unsupported value data types
fn test_2602(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
        select
            cursor(select 1 as n from dual) as cursor_col,
            xmltype('<root/>') as xml_col
        from dual
        where 1 = 0
        "#,
        &[],
    )?;

    let columns = cursor.columns();
    assert_column(&columns[0], "CURSOR_COL", &oracledb::DB_TYPE_CURSOR);
    assert_column(&columns[1], "XML_COL", &oracledb::DB_TYPE_OBJECT);
    Ok(())
}

#[rstest]
/// test metadata for duplicate and quoted column aliases
fn test_2603(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let cursor = conn.query(
        r#"
        select
            1 as dup_col,
            2 as dup_col,
            3 as "Mixed Case Alias",
            4 as "alias-with-symbols"
        from dual
        "#,
        &[],
    )?;

    let columns = cursor.columns();
    assert_column(&columns[0], "DUP_COL", &oracledb::DB_TYPE_NUMBER);
    assert_column(&columns[1], "DUP_COL", &oracledb::DB_TYPE_NUMBER);
    assert_column(&columns[2], "Mixed Case Alias", &oracledb::DB_TYPE_NUMBER);
    assert_column(
        &columns[3],
        "alias-with-symbols",
        &oracledb::DB_TYPE_NUMBER,
    );
    Ok(())
}

#[rstest]
/// test nullable metadata for table columns
fn test_2604(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        "test_2604",
        "required_col number not null, optional_col varchar2(20)",
    )?;
    let cursor = conn.query(
        "select required_col, optional_col from test_2604 where 1 = 0",
        &[],
    )?;

    let columns = cursor.columns();
    assert_column(&columns[0], "REQUIRED_COL", &oracledb::DB_TYPE_NUMBER);
    assert!(!columns[0].nullable());
    assert_column(&columns[1], "OPTIONAL_COL", &oracledb::DB_TYPE_VARCHAR);
    assert!(columns[1].nullable());
    Ok(())
}

#[rstest]
/// test for empty result sets ending fetch cleanly
fn test_2605(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let mut cursor = conn.query("select 1 from dual where 1 = 0", &[])?;
    assert!(cursor.next().is_none());
    Ok(())
}

#[rstest]
/// Tests metadata for JSON and VECTOR expressions on supported server
/// versions.
fn test_2606(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_json_supported(&conn) {
        return Ok(());
    }
    let cursor = conn.query(
        "select json_object('metadata' value true returning json)",
        &[],
    )?;
    assert_eq!(cursor.columns()[0].db_type(), &oracledb::DB_TYPE_JSON);
    Ok(())
}

#[rstest]
/// Tests metadata for JSON and VECTOR expressions on supported server
/// versions.
fn test_2607(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let cursor = conn.query("select to_vector('[1, 2]', 2, float32)", &[])?;
    assert_eq!(cursor.columns()[0].db_type(), &oracledb::DB_TYPE_VECTOR);
    Ok(())
}

#[rstest]
/// Tests fetching a nested REF CURSOR through Row::get_cursor().
fn test_2608(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let mut row = conn.query_row(
        "select cursor(select level from dual connect by level <= 3) from dual",
        &[],
    )?;
    let cursor = row.get_cursor(0)?;
    let values: Vec<i32> = cursor
        .map(|row| row?.get(0))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(values, vec![1, 2, 3]);
    Ok(())
}
