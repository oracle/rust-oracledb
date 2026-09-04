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
// plsql_in_binds.rs
//
// Shows using IN binds with PL/SQL.
//-----------------------------------------------------------------------------

mod common;

fn main() -> Result<(), oracledb::Error> {
    let config = common::get_sample_config()?;
    let connection = oracledb::connect(config)?;

    let _guard = common::create_table(
        &connection,
        "rso_examples_parent_tab",
        "parent_id number primary key, parent_name varchar2(100)",
    )?;

    connection.execute(
        r#"
        create or replace package rso_examples_pkg as
            procedure create_parent (
                p_parent_id number,
                p_parent_name varchar2
            );
        end;
        "#,
        &[],
    )?;

    connection.execute(
        r#"
        create or replace package body rso_examples_pkg as
            procedure create_parent(
                p_parent_id number,
                p_parent_name varchar2
            ) as
            begin
                insert into rso_examples_parent_tab (parent_id, parent_name)
                values (p_parent_id, p_parent_name);
            end;
        end;
        "#,
        &[],
    )?;

    let parents = oracledb::BindParameters::Slice(&[
        &[&10, &"Parent 10"],
        &[&20, &"Parent 20"],
        &[&30, &"Parent 30"],
        &[&40, &"Parent 40"],
        &[&50, &"Parent 50"],
    ]);

    connection.execute_batch(
        "begin rso_examples_pkg.create_parent(:1, :2); end;",
        parents,
    )?;

    connection.commit()?;

    let cursor = connection.query(
        "select parent_id, parent_name
         from rso_examples_parent_tab
         order by parent_id",
        &[],
    )?;

    for row_result in cursor {
        let row = row_result?;
        let parent_id: i32 = row.get(0)?;
        let parent_name: String = row.get(1)?;

        println!("{parent_id}: {parent_name}");
    }

    Ok(())
}
