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
// bind_query.rs
//
// Demonstrates the use of bind variables in queries.
//-----------------------------------------------------------------------------

mod common;

fn main() -> Result<(), oracledb::Error> {
    let config = common::get_sample_config()?;
    let connection = oracledb::connect(config)?;

    let _guard = common::create_table(
        &connection,
        "rso_examples_bind_query",
        "id number primary key, value varchar2(100)",
    )?;

    let rows = oracledb::BindParameters::Slice(&[
        &[&1, &"First row"],
        &[&2, &"Second row"],
        &[&3, &"Third row"],
        &[&4, &"Fourth row"],
        &[&5, &"Fifth row"],
        &[&6, &"Sixth row"],
        &[&7, &"Seventh row"],
    ]);
    connection.execute_batch(
        "insert into rso_examples_bind_query values (:1, :2)",
        rows,
    )?;
    connection.commit()?;

    println!("Bind by position");
    let cursor = connection.query(
        "select id, value
        from rso_examples_bind_query
         where id = :bvid",
        &[&2],
    )?;
    for row_result in cursor {
        let row = row_result?;
        let id: i32 = row.get(0)?;
        let value: String = row.get(1)?;
        println!("({id}, {value})");
    }
    println!();

    println!("Bind by name");
    let bind_id = 4;
    let cursor = connection.query_named(
        "select id, value from rso_examples_bind_query
         where id = :bvid",
        &[("bvid", &bind_id)],
    )?;
    for row_result in cursor {
        let row = row_result?;
        let id: i32 = row.get(0)?;
        let value: String = row.get(1)?;
        println!("({id}, {value})");
    }
    println!();

    Ok(())
}
