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
// test_3500_sql_parser()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use rstest::*;

/// Verifies that the SQL contains the named bind variables.
fn verify_bind_names(
    conn: oracledb::Connection,
    sql: &str,
    bind_names: &[&str],
) -> Result<(), oracledb::Error> {
    assert_eq!(conn.statement(sql)?.build()?.bind_names(), bind_names);
    Ok(())
}

#[rstest]
/// handling of single line comments
fn test_3500(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        "--begin :value2 := :a + :b + :c +:a +3; end;\n\
        begin :value2 := :a + :c +3; end; -- not a :bind_variable",
        &["VALUE2", "A", "C"],
    )
}

#[rstest]
/// handling of multiple line comments
fn test_3501(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        "/*--select * from :a where :a = 1\n\
        select * from table_names where :a = 1*/\n\
        select :table_name, :value from dual",
        &["TABLE_NAME", "VALUE"],
    )
}

#[rstest]
/// handling of constant strings
fn test_3502(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        "begin \
            :value := to_date('20021231 12:31:00', :format); \
        end;",
        &["VALUE", "FORMAT"],
    )
}

#[rstest]
/// multiple division operators
fn test_3503(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        "select :a / :b, :c / :d from dual",
        &["A", "B", "C", "D"],
    )
}

#[rstest]
/// subqueries starting with parentheses
fn test_3504(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        "(select :a from dual) union (select :b from dual",
        &["A", "B"],
    )
}

#[rstest]
/// invalid quoted bind
fn test_3505(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(conn, r#"select ":test", :a from dual"#, &["A"])
}

#[rstest]
/// non-ascii characters in the bind name
fn test_3506(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(conn, "select :méil$ from dual", &["MÉIL$"])
}

#[rstest]
#[case(r#"select :"percent%" from dual"#, &["percent%"])]
#[case(r#"select : "q?marks" from dual"#, &["q?marks"])]
#[case(r#"select :"percent%(ens)yah" from dual"#, &["percent%(ens)yah"])]
#[case(r#"select :  "per % cent" from dual"#, &["per % cent"])]
#[case(r#"select :"per cent" from dual"#, &["per cent"])]
#[case(r#"select :"par(ens)" from dual"#, &["par(ens)"])]
#[case(r#"select :"more/slashes" from dual"#, &["more/slashes"])]
#[case(r#"select :"%percent" from dual"#, &["%percent"])]
#[case(r#"select :"/slashes/" from dual"#, &["/slashes/"])]
#[case(r#"select :"1col:on" from dual"#, &["1col:on"])]
#[case(r#"select :"col:ons" from dual"#, &["col:ons"])]
#[case(r#"select :"more :: %colons%"#, &["more :: %colons%"])]
#[case(r#"select :"more/slashes" from dual"#, &["more/slashes"])]
#[case(r#"select :"spaces % spaces" from dual"#, &["spaces % spaces"])]
#[case(r#"select "col:nns", :"col:ons", :id"#, &["col:ons", "ID"])]
/// quoted bind names
fn test_3507(
    conn: oracledb::Connection,
    #[case] sql: &str,
    #[case] expected_bind_names: &[&str],
) -> Result<(), oracledb::Error> {
    verify_bind_names(conn, sql, expected_bind_names)
}

#[rstest]
/// quoted identifiers and strings together
fn test_3508(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        r#"select "/*_value1" + : "VaLue_2" + :"*/3VALUE" from dual"#,
        &["VaLue_2", "*/3VALUE"],
    )
}

#[rstest]
/// binds between simple strings
fn test_3509(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        r#"select '"string_1"', :bind_1, ':string_2' from dual"#,
        &["BIND_1"],
    )
}

#[rstest]
/// binds between comment blocks
fn test_3510(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        r#"
        select
            /* comment 1 with /* */
            :a,
            /* comment 2 with another /* */
            :b
            /* comment 3 * * * / */,
            :c
        from dual
        "#,
        &["A", "B", "C"],
    )
}

#[rstest]
/// binds between q-strings
fn test_3511(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        r#"
        select
            :a,
            q'{This contains ' and " and : just fine}',
            :b,
            q'[This contains ' and " and : just fine]',
            :c,
            q'<This contains ' and " and : just fine>',
            :d,
            q'(This contains ' and " and : just fine)',
            :e,
            q'$This contains ' and " and : just fine$',
            :f
        from dual
        "#,
        &["A", "B", "C", "D", "E", "F"],
    )
}

#[rstest]
/// binds between JSON constants
fn test_3512(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        r#"
        select
            json_object('foo':dummy),
            :bv1,
            json_object('foo'::bv2),
            :bv3,
            json { 'key1': 57, 'key2' : 58 },
            :bv4
        from dual
        "#,
        &["BV1", "BV2", "BV3", "BV4"],
    )
}

#[rstest]
/// multiple line comment with multiple asterisks
fn test_3513(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        r#"
        /****--select * from :a where :a = 1
        select * from table_names where :a = 1****/
        select :table_name, :value from dual
        "#,
        &["TABLE_NAME", "VALUE"],
    )
}

#[rstest]
/// q-string without a closing quote
fn test_3514(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let err = match conn.statement("select q'[something from dual")?.build() {
        Ok(_) => panic!("expected failure"),
        Err(err) => err,
    };
    assert!(matches!(err.kind(), oracledb::ErrorKind::ParseError(_, _)));
    Ok(())
}

#[rstest]
/// different space combinations with :=
fn test_3515(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        r#"
        begin
            :value2 :=
                :a  + :b  +   :c +:a +3;
            :value2
                := :a + :c +3;
        end;
        "#,
        &["VALUE2", "A", "B", "C"],
    )
}

#[rstest]
/// binds between multiple comment blocks with quotes
fn test_3516(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        r#"
        select
            /* ' comment 1 */
            :a,
            /* "comment " 2 ' */:b
            /* comment 3 '*/,
            :c
            /* comment 4 ""*/
        from dual
        "#,
        &["A", "B", "C"],
    )
}

#[rstest]
/// query with a missing end quote
fn test_3517(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let err = match conn.statement("select 'abc, :a from dual")?.build() {
        Ok(_) => panic!("expected failure"),
        Err(err) => err,
    };
    assert!(matches!(err.kind(), oracledb::ErrorKind::ParseError(_, _)));
    Ok(())
}

#[rstest]
/// q-string with incorrect closing symbols
fn test_3518(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let err = match conn.statement("select q'[abc'], 5 from dual")?.build() {
        Ok(_) => panic!("expected failure"),
        Err(err) => err,
    };
    assert!(matches!(err.kind(), oracledb::ErrorKind::ParseError(_, _)));
    Ok(())
}

#[rstest]
/// DDL does not allow bind variables
fn test_3519(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    verify_bind_names(
        conn,
        r#"
        create or replace trigger test_3519_tg
        after update of updated_column on test_3519
        for each row
        begin
            update test_3519 set
                some_other_column = :new.updated_column
            where some_other_column = :old.updated_column;
        end;
        "#,
        &[],
    )
}
