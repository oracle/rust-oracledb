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
// dml_returning.rs
//
// Shows the use of DML returning.
//-----------------------------------------------------------------------------

mod common;

fn main() -> Result<(), oracledb::Error> {
    let config = common::get_sample_config()?;
    let connection = oracledb::connect(config)?;

    let _guard = common::create_table(
        &connection,
        "rso_examples_dml_returning",
        r#"
        department_id number primary key,
        department_name varchar2(100),
        location_id number
        "#,
    )?;

    connection.execute(
        "insert into rso_examples_dml_returning values (:1, :2, :3)",
        &[&50 as &dyn oracledb::ToDbValue, &"Shipping", &1500],
    )?;
    connection.commit()?;

    let dept_name = " ".repeat(100);
    let mut result = connection.execute_named(
        r#"
        update rso_examples_dml_returning set
            location_id = :loc_id
        where department_id = :dept_id
        returning department_name
        into :dept_name
        "#,
        &[
            ("loc_id", &1700 as &dyn oracledb::ToDbValue),
            ("dept_id", &50),
            ("dept_name", &dept_name),
        ],
    )?;

    let dept_names: Vec<String> = result
        .returned_data()
        .into_iter()
        .map(|r| r.get(0).unwrap())
        .collect();

    println!("{dept_names:?}");

    Ok(())
}
