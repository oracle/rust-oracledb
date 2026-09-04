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
// json.rs
//
// Shows how to bind and fetch JSON values.
//-----------------------------------------------------------------------------

use std::collections::HashMap;

mod common;

fn main() -> Result<(), oracledb::Error> {
    let config = common::get_sample_config()?;
    let connection = oracledb::connect(config)?;

    // native JSON columns require Oracle Database 21 or later.
    let version = connection.version()?;
    if version.0 < 21 {
        println!("JSON columns require Oracle Database 21 or later.");
        return Ok(());
    }

    // Create a table with a native JSON column.
    let _guard = common::create_table(
        &connection,
        "rso_examples_json",
        "id number primary key, data json",
    )?;

    // Bind a JSON value from Rust and insert it into the JSON column.
    let mut obj = HashMap::new();

    obj.insert(
        "department_name".to_string(),
        oracledb::JsonValue::String("Sales".to_string()),
    );

    let json = oracledb::JsonValue::JsonObject(obj);
    let id = 1;

    connection.execute(
        "insert into rso_examples_json (id, data) values (:1, :2)",
        &[&id, &json],
    )?;

    connection.commit()?;

    // fetch a JSON column directly into an oracledb::JsonValue.
    let row = connection.query_row(
        "select data from rso_examples_json where id = :1",
        &[&id],
    )?;

    let data: oracledb::JsonValue = row.get(0)?;
    println!("{data:#?}");

    Ok(())
}
