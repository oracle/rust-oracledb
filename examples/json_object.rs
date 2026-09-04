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
// json_object.rs
//
// Shows how to use JSON_OBJECT to return rows as JSON values.
//-----------------------------------------------------------------------------

mod common;

fn main() -> Result<(), oracledb::Error> {
    let config = common::get_sample_config()?;
    let connection = oracledb::connect(config)?;

    // native JSON columns require Oracle Database 21 or later
    let version = connection.version()?;
    if version.0 < 21 {
        println!("JSON columns require Oracle Database 21 or later.");
        return Ok(());
    }

    let _guard = common::create_table(
        &connection,
        "rso_examples_departments",
        "id number primary key, name varchar2(100)",
    )?;
    let rows = oracledb::BindParameters::Slice(&[
        &[&10, &"Administration"],
        &[&20, &"Marketing"],
        &[&30, &"Purchasing"],
        &[&40, &"Human Resources"],
    ]);
    connection.execute_batch(
        "insert into rso_examples_departments values (:1, :2)",
        rows,
    )?;
    connection.commit()?;

    let cursor = connection.query(
        r#"select json_object(
              'deptId' value d.id,
              'name' value d.name
              returning json
          ) department
        from rso_examples_departments d
        where id in (:1, :2, :3, :4)
        order by d.name"#,
        &[&10, &20, &30, &40],
    )?;

    for row_result in cursor {
        let row = row_result?;
        let department: oracledb::JsonValue = row.get(0)?;
        println!("{department:?}");
    }

    Ok(())
}
