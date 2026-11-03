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
#[case::varchar(
    "VARCHAR_COL",
    "'abc'",
    "VARCHAR2(30)",
    30,
    0,
    0,
    oracledb::DB_TYPE_VARCHAR
)]
#[case::char("CHAR_COL", "'abc'", "CHAR(5)", 5, 0, 0, oracledb::DB_TYPE_CHAR)]
#[case::nvarchar(
    "VARCHAR_COL",
    "'abc'",
    "NVARCHAR2(20)",
    20,
    0,
    0,
    oracledb::DB_TYPE_NVARCHAR
)]
#[case::nchar(
    "NCHAR_COL",
    "'abc'",
    "NCHAR(4)",
    4,
    0,
    0,
    oracledb::DB_TYPE_NCHAR
)]
#[case::unconstrained_number(
    "UNCONSTRAINED_NUMBER_COL",
    "123.45",
    "NUMBER",
    0,
    0,
    -127,
    oracledb::DB_TYPE_NUMBER
)]
#[case::number_with_precision(
    "NUMBER_WITH_PRECISION_COL",
    "12345",
    "NUMBER(9)",
    0,
    9,
    0,
    oracledb::DB_TYPE_NUMBER
)]
#[case::constrained_number(
    "CONSTRAINED_NUMBER_COL",
    "123.45",
    "NUMBER(5,2)",
    0,
    5,
    2,
    oracledb::DB_TYPE_NUMBER
)]
#[case::binary_float(
    "BINARY_FLOAT_COL",
    "1.25",
    "BINARY_FLOAT",
    0,
    0,
    0,
    oracledb::DB_TYPE_BINARY_FLOAT
)]
#[case::binary_double(
    "BINARY_DOUBLE_COL",
    "1.25",
    "BINARY_DOUBLE",
    0,
    0,
    0,
    oracledb::DB_TYPE_BINARY_DOUBLE
)]
#[case::date(
    "DATE_COL",
    "date '2024-02-29'",
    "DATE",
    0,
    0,
    0,
    oracledb::DB_TYPE_DATE
)]
#[case::timestamp(
    "TIMESTAMP_COL",
    "timestamp '2024-02-29 23:59:25.123456789'",
    "TIMESTAMP(9)",
    0,
    0,
    9,
    oracledb::DB_TYPE_TIMESTAMP
)]
#[case::timestamp_ltz(
    "TIMESTAMP_LTZ_COL",
    "timestamp '2024-02-29 23:59:25.123456'",
    "TIMESTAMP(6) WITH LOCAL TIME ZONE",
    0,
    0,
    6,
    oracledb::DB_TYPE_TIMESTAMP_LTZ
)]
#[case::timestamp_tz(
    "TIMESTAMP_TZ_COL",
    "timestamp '2024-02-29 23:59:25.123 +05:30'",
    "TIMESTAMP(3) WITH TIME ZONE",
    0,
    0,
    3,
    oracledb::DB_TYPE_TIMESTAMP_TZ
)]
#[case::interval_ds(
    "INTERVAL_DS_COL",
    "interval '12 01:02:03.123456789' day to second",
    "INTERVAL DAY(2) TO SECOND(9)",
    0,
    2,
    9,
    oracledb::DB_TYPE_INTERVAL_DS
)]
#[case::interval_ym(
    "INTERVAL_YM_COL",
    "interval '10-11' year to month",
    "INTERVAL YEAR(2) TO MONTH",
    0,
    2,
    0,
    oracledb::DB_TYPE_INTERVAL_YM
)]
#[case::raw(
    "RAW_COL",
    "hextoraw('DEADBEEF')",
    "RAW(4)",
    4,
    0,
    0,
    oracledb::DB_TYPE_RAW
)]
#[case::float_col(
    "FLOAT_COL",
    "123.45",
    "FLOAT(10)",
    0,
    10,
    -127,
    oracledb::DB_TYPE_NUMBER
)]
/// test metadata for supported scalar data types
fn test_2600(
    conn: oracledb::Connection,
    #[case] name: &str,
    #[case] value: &str,
    #[case] data_type: &str,
    #[case] max_size: u32,
    #[case] precision: u8,
    #[case] scale: i8,
    #[case] db_type: &'static oracledb::DbType,
) -> Result<(), oracledb::Error> {
    let sql =
        format!("select cast({value} as {data_type}) as {name} from dual");
    let cursor = conn.query(&sql, &[])?;
    let column = &cursor.columns()[0];
    assert_eq!(column.name(), name);
    assert_eq!(column.data_type(), data_type);
    assert_eq!(column.max_size(), max_size);
    assert_eq!(column.precision(), precision);
    assert_eq!(column.scale(), scale);
    assert_eq!(column.db_type(), db_type);
    Ok(())
}

#[rstest]
#[case::clob(
    "CLOB_COL",
    "to_clob('abc')",
    oracledb::DB_TYPE_CLOB,
    oracledb::DB_TYPE_LONG
)]
#[case::nclob(
    "NCLOB_COL",
    "to_nclob('abc')",
    oracledb::DB_TYPE_NCLOB,
    oracledb::DB_TYPE_LONG_NVARCHAR
)]
#[case::blob(
    "BLOB_COL",
    "to_blob(hextoraw('DEADBEEF'))",
    oracledb::DB_TYPE_BLOB,
    oracledb::DB_TYPE_LONG_RAW
)]
/// test metadata for supported large LOB data types
fn test_2601(
    conn: oracledb::Connection,
    #[case] name: &str,
    #[case] value: &str,
    #[case] db_type_with_lobs: &'static oracledb::DbType,
    #[case] db_type_without_lobs: &'static oracledb::DbType,
) -> Result<(), oracledb::Error> {
    let sql = format!("select {value} as {name} from dual");
    let cursor = conn.query(&sql, &[])?;
    let column = &cursor.columns()[0];
    assert_eq!(column.name(), name);
    assert_eq!(column.db_type(), db_type_without_lobs);
    let cursor = conn.statement(&sql)?.fetch_lobs().build()?.query(&[])?;
    let column = &cursor.columns()[0];
    assert_eq!(column.db_type(), db_type_with_lobs);
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
    assert_column(&columns[0], "CURSOR_COL", oracledb::DB_TYPE_CURSOR);
    assert_column(&columns[1], "XML_COL", oracledb::DB_TYPE_XMLTYPE);
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
    assert_column(&columns[0], "DUP_COL", oracledb::DB_TYPE_NUMBER);
    assert_column(&columns[1], "DUP_COL", oracledb::DB_TYPE_NUMBER);
    assert_column(&columns[2], "Mixed Case Alias", oracledb::DB_TYPE_NUMBER);
    assert_column(&columns[3], "alias-with-symbols", oracledb::DB_TYPE_NUMBER);
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
    assert_column(&columns[0], "REQUIRED_COL", oracledb::DB_TYPE_NUMBER);
    assert!(!columns[0].nullable());
    assert_column(&columns[1], "OPTIONAL_COL", oracledb::DB_TYPE_VARCHAR);
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
    assert_eq!(cursor.columns()[0].db_type(), oracledb::DB_TYPE_JSON);
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
    assert_eq!(cursor.columns()[0].db_type(), oracledb::DB_TYPE_VECTOR);
    Ok(())
}

#[rstest]
/// Tests fetching a cursor returned as a column through Row::take().
fn test_2608(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let mut row = conn.query_row(
        "select cursor(select level from dual connect by level <= 3) from dual",
        &[],
    )?;
    let cursor: oracledb::Cursor = row.take(0)?;
    let values: Vec<i32> =
        cursor.map(|row| row?.get(0)).collect::<Result<Vec<_>, _>>()?;
    assert_eq!(values, vec![1, 2, 3]);
    Ok(())
}

#[test]
/// Verifies exported database type descriptors classify their value families
/// consistently.
fn test_2609() {
    for db_type in [
        &oracledb::DB_TYPE_RAW,
        &oracledb::DB_TYPE_LONG_RAW,
        &oracledb::DB_TYPE_BLOB,
    ] {
        assert!(db_type.is_binary_type(), "{}", db_type.name());
        assert!(!db_type.is_date_type(), "{}", db_type.name());
    }
    for db_type in [
        &oracledb::DB_TYPE_DATE,
        &oracledb::DB_TYPE_TIMESTAMP,
        &oracledb::DB_TYPE_TIMESTAMP_LTZ,
        &oracledb::DB_TYPE_TIMESTAMP_TZ,
    ] {
        assert!(db_type.is_date_type(), "{}", db_type.name());
        assert!(!db_type.is_binary_type(), "{}", db_type.name());
    }
    for db_type in [
        &oracledb::DB_TYPE_CHAR,
        &oracledb::DB_TYPE_NCHAR,
        &oracledb::DB_TYPE_VARCHAR,
        &oracledb::DB_TYPE_NVARCHAR,
        &oracledb::DB_TYPE_LONG,
        &oracledb::DB_TYPE_LONG_NVARCHAR,
        &oracledb::DB_TYPE_ROWID,
        &oracledb::DB_TYPE_UROWID,
        &oracledb::DB_TYPE_CLOB,
        &oracledb::DB_TYPE_NCLOB,
    ] {
        assert!(db_type.is_string_type(), "{}", db_type.name());
    }
}

#[rstest]
/// Tests annotations of table columns.
fn test_2610(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_domains_supported(&conn) {
        return Ok(());
    }
    let _guard = common::create_table(
        &conn,
        "test_2610",
        r#"
        annotated_col number annotations (Display 'x', Hidden),
        not_annotated_col number,
        domain_col positive_number_d
        "#,
    )?;
    let cursor = conn.query(
        "select annotated_col, not_annotated_col, domain_col from test_2610",
        &[],
    )?;
    let columns = cursor.columns();
    assert_eq!(
        columns[0].annotations(),
        [
            ("DISPLAY".to_string(), "x".to_string()),
            ("HIDDEN".to_string(), String::new())
        ]
    );
    assert!(columns[1].annotations().is_empty());
    assert!(columns[0].domain().is_none());
    assert_eq!(columns[2].domain(), Some(("SYS", "POSITIVE_NUMBER_D")));
    Ok(())
}
