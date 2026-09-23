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
// rowid_urowid.rs
//
// Shows how to fetch ROWID and UROWID values and use them in binds.
//-----------------------------------------------------------------------------

mod common;

fn main() -> Result<(), oracledb::Error> {
    let config = common::get_sample_config()?;
    let connection = oracledb::connect(config)?;

    let _guard_rowid = common::create_table(
        &connection,
        "rso_examples_rowid",
        "id number primary key, value varchar2(100)",
    )?;

    // Index-organized tables use logical rowids, returned as UROWID
    let _guard_urowid = common::create_table_with_options(
        &connection,
        "rso_examples_urowid",
        r#"
        id number not null,
        value varchar2(100),
        constraint rso_examples_urowid_pk primary key (id)
        "#,
        "organization index",
    )?;

    connection.execute(
        "insert into rso_examples_rowid values (:1, :2)",
        &[&1, &"Alice"],
    )?;
    connection.execute(
        "insert into rso_examples_urowid values (:1, :2)",
        &[&1, &"Carol"],
    )?;

    println!("Regular table ROWID example");

    // Fetch the ROWID and use it to locate the same row later
    let row = connection.query_row(
        "select rowid, value from rso_examples_rowid where id = :1",
        &[&1],
    )?;
    let rowid: String = row.get(0)?;
    println!("  fetched {}: {rowid}", row.columns()[0].data_type());
    let value: String = row.get(1)?;
    println!("  value before ROWID update: {value}");

    // Bind the fetched ROWID in the update
    connection.execute_named(
        "update rso_examples_rowid set value = :value where rowid = :rid",
        &[("value", &"Bob"), ("rid", &rowid)],
    )?;
    let row = connection.query_row(
        "select value from rso_examples_rowid where rowid = :1",
        &[&rowid],
    )?;
    let value: String = row.get(0)?;
    println!("  value after ROWID update: {value}");

    // Fetch the UROWID from the index-organized table
    println!("Index-organized table UROWID example");
    let row = connection.query_row(
        "select rowid, value from rso_examples_urowid where id = :1",
        &[&1],
    )?;
    let urowid: String = row.get(0)?;
    println!("  fetched {}: {urowid}", row.columns()[0].data_type());
    let value: String = row.get(1)?;
    println!("  value before UROWID update: {value}");

    // Bind the fetched UROWID in the update
    connection.execute_named(
        "update rso_examples_urowid set value = :value where rowid = :rid",
        &[("value", &"Dave"), ("rid", &urowid)],
    )?;

    let row = connection.query_row(
        "select value from rso_examples_urowid where rowid = :1",
        &[&urowid],
    )?;
    let value: String = row.get(0)?;
    println!("  value after UROWID update: {value}");

    Ok(())
}
