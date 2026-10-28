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
// statement_building.rs
//
// Shows building and inspecting a statement.
//-----------------------------------------------------------------------------

mod common;

fn main() -> Result<(), oracledb::Error> {
    let config = common::get_sample_config()?;
    let connection = oracledb::connect(config)?;

    let sql = "select user from dual";

    let mut statement = connection
        .statement(sql)?
        .prefetch_rows(1)
        .fetch_array_size(10)
        .build()?;

    println!("SQL: {}", statement.sql());
    println!("Bind names: {:?}", statement.bind_names());
    println!("Is query: {}", statement.is_query());
    println!("Is DML: {}", statement.is_dml());
    println!("Is DDL: {}", statement.is_ddl());
    println!("Is PL/SQL: {}", statement.is_plsql());
    println!("Is DML returning: {}", statement.is_dml_returning());
    println!(
        "Fully parsed before inspection: {}",
        statement.is_fully_parsed()
    );

    statement.ensure_fully_parsed()?;

    println!("Fully parsed after inspection: {}", statement.is_fully_parsed());

    for (index, metadata) in statement.out_metadata().iter().enumerate() {
        println!(
            "Column {index}: name={}, type={}",
            metadata.name(),
            metadata.data_type()
        );
    }

    let row = statement.query_row(&[])?;
    let result: String = row.get(0)?;

    println!("Connected user: {result}");

    Ok(())
}
