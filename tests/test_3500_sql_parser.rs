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
use oracledb;
use rstest::*;

#[rstest]
/// handling of single line comments
fn test_3500(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement(
        "--begin :value2 := :a + :b + :c +:a +3; end;\n\
        begin :value2 := :a + :c +3; end; -- not a :bind_variable",
    )?;
    assert_eq!(statement.bind_names()?, ["VALUE2", "A", "C"]);
    Ok(())
}

#[rstest]
/// handling of multiple line comments
fn test_3501(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement(
        "/*--select * from :a where :a = 1\n\
        select * from table_names where :a = 1*/\n\
        select :table_name, :value from dual",
    )?;
    assert_eq!(statement.bind_names()?, ["TABLE_NAME", "VALUE"]);
    Ok(())
}

#[rstest]
/// handling of constant strings
fn test_3502(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement(
        "begin \
            :value := to_date('20021231 12:31:00', :format); \
        end;",
    )?;
    assert_eq!(statement.bind_names()?, ["VALUE", "FORMAT"]);
    Ok(())
}

#[rstest]
/// multiple division operators
fn test_3503(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement("select :a / :b, :c / :d from dual")?;
    assert_eq!(statement.bind_names()?, ["A", "B", "C", "D"]);
    Ok(())
}

#[rstest]
/// subqueries starting with parentheses
fn test_3504(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement =
        conn.statement("(select :a from dual) union (select :b from dual")?;
    assert_eq!(statement.bind_names()?, ["A", "B"]);
    Ok(())
}

#[rstest]
/// invalid quoted bind
fn test_3505(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement(r#"select ":test", :a from dual"#)?;
    assert_eq!(statement.bind_names()?, ["A"]);
    Ok(())
}

#[rstest]
/// non-ascii characters in the bind name
fn test_3506(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement("select :méil$ from dual")?;
    assert_eq!(statement.bind_names()?, ["MÉIL$"]);
    Ok(())
}

#[rstest]
#[case(r#"select :"percent%" from dual"#, vec!["percent%"])]
#[case(r#"select : "q?marks" from dual"#, vec!["q?marks"])]
#[case(r#"select :"percent%(ens)yah" from dual"#, vec!["percent%(ens)yah"])]
#[case(r#"select :  "per % cent" from dual"#, vec!["per % cent"])]
#[case(r#"select :"par(ens)" from dual"#, vec!["par(ens)"])]
#[case(r#"select :"more/slashes" from dual"#, vec!["more/slashes"])]
#[case(r#"select :"%percent" from dual"#, vec!["%percent"])]
#[case(r#"select :"/slashes/" from dual"#, vec!["/slashes/"])]
#[case(r#"select :"1col:on" from dual"#, vec!["1col:on"])]
#[case(r#"select :"col:ons" from dual"#, vec!["col:ons"])]
#[case(r#"select :"more :: %colons%"#, vec!["more :: %colons%"])]
#[case(r#"select :"more/slashes" from dual"#, vec!["more/slashes"])]
#[case(r#"select :"spaces % spaces" from dual"#, vec!["spaces % spaces"])]
#[case(r#"select "col:nns", :"col:ons", :id"#, vec!["col:ons", "ID"])]
/// quoted bind names
fn test_3507(
    conn: oracledb::Connection,
    #[case] sql: &str,
    #[case] expected_bind_names: Vec<&str>,
) -> Result<(), oracledb::Error> {
    let statement = conn.statement(sql)?;
    assert_eq!(statement.bind_names()?, expected_bind_names);
    Ok(())
}

#[rstest]
/// quoted identifiers and strings together
fn test_3508(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement(
        r#"select "/*_value1" + : "VaLue_2" + :"*/3VALUE" from dual"#,
    )?;
    assert_eq!(statement.bind_names()?, ["VaLue_2", "*/3VALUE"]);
    Ok(())
}

#[rstest]
/// binds between simple strings
fn test_3509(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn
        .statement(r#"select '"string_1"', :bind_1, ':string_2' from dual"#)?;
    assert_eq!(statement.bind_names()?, ["BIND_1"]);
    Ok(())
}

#[rstest]
/// binds between comment blocks
fn test_3510(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement(
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
    )?;
    assert_eq!(statement.bind_names()?, ["A", "B", "C"]);
    Ok(())
}

#[rstest]
/// binds between q-strings
fn test_3511(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement(
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
    )?;
    assert_eq!(statement.bind_names()?, ["A", "B", "C", "D", "E", "F"]);
    Ok(())
}

#[rstest]
/// binds between JSON constants
fn test_3512(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement(
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
    )?;
    assert_eq!(statement.bind_names()?, ["BV1", "BV2", "BV3", "BV4"]);
    Ok(())
}

#[rstest]
/// multiple line comment with multiple asterisks
fn test_3513(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement(
        r#"
        /****--select * from :a where :a = 1
        select * from table_names where :a = 1****/
        select :table_name, :value from dual
        "#,
    )?;
    assert_eq!(statement.bind_names()?, ["TABLE_NAME", "VALUE"]);
    Ok(())
}

#[rstest]
/// q-string without a closing quote
fn test_3514(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement("select q'[something from dual")?;
    let err = match statement.bind_names() {
        Ok(_) => panic!("expected failure"),
        Err(err) => err,
    };
    assert!(matches!(err.kind(), oracledb::ErrorKind::ParseError(_, _)));
    Ok(())
}

#[rstest]
/// different space combinations with :=
fn test_3515(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement(
        r#"
        begin
            :value2 :=
                :a  + :b  +   :c +:a +3;
            :value2
                := :a + :c +3;
        end;
        "#,
    )?;
    assert_eq!(statement.bind_names()?, ["VALUE2", "A", "B", "C"]);
    Ok(())
}

#[rstest]
/// binds between multiple comment blocks with quotes
fn test_3516(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement(
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
    )?;
    assert_eq!(statement.bind_names()?, ["A", "B", "C"]);
    Ok(())
}

#[rstest]
/// query with a missing end quote
fn test_3517(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement("select 'abc, :a from dual")?;
    let err = match statement.bind_names() {
        Ok(_) => panic!("expected failure"),
        Err(err) => err,
    };
    assert!(matches!(err.kind(), oracledb::ErrorKind::ParseError(_, _)));
    Ok(())
}

#[rstest]
/// q-string with incorrect closing symbols
fn test_3518(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let statement = conn.statement("select q'[abc'], 5 from dual")?;
    let err = match statement.bind_names() {
        Ok(_) => panic!("expected failure"),
        Err(err) => err,
    };
    assert!(matches!(err.kind(), oracledb::ErrorKind::ParseError(_, _)));
    Ok(())
}
