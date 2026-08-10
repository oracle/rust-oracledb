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
// vectors_sparse.rs
//
// Shows inserting and querying SPARSE VECTOR columns.
//-----------------------------------------------------------------------------

use oracledb::{SparseVector, Vector, VectorData};

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
        .execute("drop table if exists vector_sparse_table purge", &[])?;

    connection.execute(
        "create table vector_sparse_table (
            v32 vector(25, float32, sparse),
            v64 vector(30, float64, sparse),
            v8  vector(35, int8, sparse)
        )",
        &[],
    )?;

    let float32_val = Vector::Sparse(SparseVector::new(
        25,
        vec![6, 10, 18],
        VectorData::Float32(vec![26.25, 129.625, 579.875]),
    ));

    let float64_val = Vector::Sparse(SparseVector::new(
        30,
        vec![9, 16, 24],
        VectorData::Float64(vec![19.125, 78.5, 977.375]),
    ));

    let int8_val = Vector::Sparse(SparseVector::new(
        35,
        vec![10, 20, 30],
        VectorData::Int8(vec![26, 125, -37]),
    ));

    connection.execute(
        "insert into vector_sparse_table values (:1, :2, :3)",
        &[&float32_val, &float64_val, &int8_val],
    )?;

    connection.commit()?;

    let cursor = connection.query("select * from vector_sparse_table", &[])?;

    for row_result in cursor {
        let row = row_result?;

        let v32: Vector = row.get(0)?;
        let v64: Vector = row.get(1)?;
        let v8: Vector = row.get(2)?;

        println!("v32 = {v32:?}");
        println!("v64 = {v64:?}");
        println!("v8 = {v8:?}");
    }

    Ok(())
}
