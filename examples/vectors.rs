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
// vectors.rs
//
// Shows inserting and querying VECTOR columns.
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

    connection.execute("drop table if exists vector_table purge", &[])?;

    connection.execute(
        "create table vector_table (
            v32 vector(3, float32),
            v64 vector(3, float64),
            v8  vector(3, int8)
        )",
        &[],
    )?;

    let v32 = Vector::Dense(VectorData::Float32(vec![1.625, 1.5, 1.0]));
    let v64 = Vector::Dense(VectorData::Float64(vec![11.25, 11.75, 11.5]));
    let v8 = Vector::Dense(VectorData::Int8(vec![1, 2, 3]));

    connection.execute(
        "insert into vector_table (v32, v64, v8) values (:1, :2, :3)",
        &[&v32, &v64, &v8],
    )?;

    connection.commit()?;

    let row =
        connection.query_row("select v32, v64, v8 from vector_table", &[])?;

    let fetched_v32: Vector = row.get(0)?;
    let fetched_v64: Vector = row.get(1)?;
    let fetched_v8: Vector = row.get(2)?;

    println!("v32 = {fetched_v32:?}");
    println!("v64 = {fetched_v64:?}");
    println!("v8 = {fetched_v8:?}");

    Ok(())
}
