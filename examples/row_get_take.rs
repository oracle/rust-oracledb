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
// row_get_take.rs
//
// Shows the use of Row::get() and Row::take().
//-----------------------------------------------------------------------------

mod common;

fn main() -> Result<(), oracledb::Error> {
    let config = common::get_sample_config()?;
    let connection = oracledb::connect(config)?;

    let _guard = common::create_table(
        &connection,
        "rso_examples_row_get_take",
        "title varchar2(100)",
    )?;
    connection.execute(
        "insert into rso_examples_row_get_take values (:1)",
        &[&"Learning rust-oracledb"],
    )?;
    connection.commit()?;

    let mut row = connection
        .query_row("select title from rso_examples_row_get_take", &[])?;

    // get() reads a column value without removing it from the row
    let title_from_get: String = row.get("TITLE")?;
    println!("get() title: {title_from_get}");

    // take() moves the column value out of the row
    let title_from_take: String = row.take("TITLE")?;
    println!("take() title: {title_from_take}");

    // after take(), the column value in this row is null
    let title_after_take: Option<String> = row.get("TITLE")?;
    println!("title after take(): {title_after_take:?}");

    Ok(())
}
