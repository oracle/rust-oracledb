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
// vectors_binary.rs
//
// Shows inserting and querying BINARY VECTOR columns.
//-----------------------------------------------------------------------------

use oracledb::{Vector, VectorData};

mod common;

fn main() -> Result<(), oracledb::Error> {
    let config = common::get_sample_config()?;
    let connection = oracledb::connect(config)?;

    // Check the Oracle Database version
    let version = connection.version()?;
    if version.0 < 23 {
        println!("VECTOR columns require Oracle AI Database 26ai or later.");
        return Ok(());
    }

    connection
        .execute("drop table if exists vector_binary_table purge", &[])?;

    connection.execute(
        "create table vector_binary_table (
            embedding vector(24, binary)
        )",
        &[],
    )?;

    let vector_data_vb =
        Vector::Dense(VectorData::Binary(vec![180, 150, 100]));

    connection.execute(
        "insert into vector_binary_table values (:1)",
        &[&vector_data_vb],
    )?;

    connection.commit()?;

    let cursor = connection.query("select * from vector_binary_table", &[])?;

    for row_result in cursor {
        let row = row_result?;
        let vector: Vector = row.get(0)?;

        println!("{vector:?}");
    }

    Ok(())
}
