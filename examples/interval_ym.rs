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
// interval_ym.rs
//
// Shows inserting and querying INTERVAL YEAR TO MONTH columns.
//-----------------------------------------------------------------------------

mod common;

fn main() -> Result<(), oracledb::Error> {
    let config = common::get_sample_config()?;
    let connection = oracledb::connect(config)?;

    let _guard = common::create_table(
        &connection,
        "rso_examples_interval_ym",
        "value INTERVAL YEAR TO MONTH",
    )?;

    // Create an INTERVAL YEAR TO MONTH value
    let interval_value = oracledb::OracleIntervalYM::new(2, 3);

    connection.execute(
        "insert into rso_examples_interval_ym (value) values (:1)",
        &[&interval_value],
    )?;

    connection.commit()?;

    let row = connection
        .query_row("select value from rso_examples_interval_ym", &[])?;

    let value: oracledb::OracleIntervalYM = row.get(0)?;

    // Print interval fields
    println!("{value:?}");

    Ok(())
}
