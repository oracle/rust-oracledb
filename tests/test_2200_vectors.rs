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
// test_2200_vectors()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;

/// Test querying a DENSE FLOAT32 VECTOR
#[rstest]
fn test_2200(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let sql = "select vector('[1.0, 2.5, -3.5, 4.0, 0.5]', 5, float32)";
    let row = conn.query_row(sql, &[])?;
    match row.get(0)? {
        oracledb::Vector::Dense(oracledb::VectorData::Float32(values)) => {
            assert_eq!(values, vec![1.0, 2.5, -3.5, 4.0, 0.5]);
        }
        _ => panic!("Expected Dense Float32 vector"),
    }
    Ok(())
}

/// Test querying a DENSE FLOAT64 VECTOR
#[rstest]
fn test_2201(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let sql = "select vector('[1.5, -2.5, 0.125]', 3, float64)";
    let row = conn.query_row(sql, &[])?;
    match row.get(0)? {
        oracledb::Vector::Dense(oracledb::VectorData::Float64(values)) => {
            assert_eq!(values, vec![1.5, -2.5, 0.125]);
        }
        _ => panic!("Expected Dense Float64 vector"),
    }
    Ok(())
}

/// Test querying a DENSE INT8 VECTOR
#[rstest]
fn test_2202(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let sql = "select vector('[-1, 0, 15, 120]', 4, int8)";
    let row = conn.query_row(sql, &[])?;
    match row.get(0)? {
        oracledb::Vector::Dense(oracledb::VectorData::Int8(values)) => {
            assert_eq!(values, vec![-1, 0, 15, 120]);
        }
        _ => panic!("Expected Dense Int8 vector"),
    }
    Ok(())
}

/// Test querying a DENSE BINARY VECTOR
#[rstest]
fn test_2203(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let sql =
        "select vector('[1,0,1,0,1,0,1,0, 0,1,0,1,0,1,0,1]', 128, binary)";
    let row = conn.query_row(sql, &[])?;
    match row.get(0)? {
        oracledb::Vector::Dense(oracledb::VectorData::Binary(values)) => {
            assert_eq!(
                values,
                vec![1, 0, 1, 0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, 1]
            );
        }
        _ => panic!("Expected Dense Binary vector"),
    }
    Ok(())
}

/// Test querying a SPARSE FLOAT32 VECTOR
#[rstest]
fn test_2204(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_sparse_vectors_supported(&conn) {
        return Ok(());
    }
    let sql = "select vector('[1000, [0,500,999], [1.5,2.5,3.5]]', 1000, float32, sparse)";
    let row = conn.query_row(sql, &[])?;
    match row.get(0)? {
        oracledb::Vector::Sparse(sparse) => {
            assert_eq!(sparse.num_dimensions(), 1000);
            assert_eq!(sparse.indices(), vec![0, 500, 999]);
            match sparse.values() {
                oracledb::VectorData::Float32(values) => {
                    assert_eq!(*values, vec![1.5, 2.5, 3.5]);
                }
                _ => panic!("Expected Float32 values"),
            }
        }
        _ => panic!("Expected Sparse vector"),
    }
    Ok(())
}

/// Test querying a SPARSE FLOAT64 VECTOR
#[rstest]
fn test_2205(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_sparse_vectors_supported(&conn) {
        return Ok(());
    }
    let sql = "select vector('[1000, [0,500,999], [1.5,2.5,3.5]]', 1000, float64, sparse)";
    let row = conn.query_row(sql, &[])?;
    match row.get(0)? {
        oracledb::Vector::Sparse(sparse) => {
            assert_eq!(sparse.num_dimensions(), 1000);
            assert_eq!(sparse.indices(), vec![0, 500, 999]);
            match sparse.values() {
                oracledb::VectorData::Float64(values) => {
                    assert_eq!(*values, vec![1.5, 2.5, 3.5]);
                }
                _ => panic!("Expected Float64 values"),
            }
        }
        _ => panic!("Expected Sparse vector"),
    }
    Ok(())
}

/// Test querying a SPARSE INT8 VECTOR
#[rstest]
fn test_2206(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_sparse_vectors_supported(&conn) {
        return Ok(());
    }
    let sql =
        "select vector('[100, [0,50,99], [-128, 0, 127]]', 100, int8, sparse)";
    let row = conn.query_row(sql, &[])?;
    match row.get(0)? {
        oracledb::Vector::Sparse(sparse) => {
            assert_eq!(sparse.num_dimensions(), 100);
            assert_eq!(sparse.indices(), vec![0, 50, 99]);
            match sparse.values() {
                oracledb::VectorData::Int8(values) => {
                    assert_eq!(*values, vec![-128, 0, 127]);
                }
                _ => panic!("Expected Int8 values"),
            }
        }
        _ => panic!("Expected Sparse vector"),
    }
    Ok(())
}

/// Roundtrip test for DENSE FLOAT32 vector
#[rstest]
fn test_2207(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let values = vec![1.0, 2.5, -3.5, 4.5, 0.5];
    let vector =
        oracledb::Vector::Dense(oracledb::VectorData::Float32(values.clone()));
    let row = conn.query_row("select :1", &[&vector])?;
    match row.get(0)? {
        oracledb::Vector::Dense(oracledb::VectorData::Float32(
            fetched_values,
        )) => {
            assert_eq!(fetched_values, values);
        }
        _ => panic!("Expected Dense Float32 vector"),
    }
    Ok(())
}

/// Roundtrip test for DENSE FLOAT64
#[rstest]
fn test_2208(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let values = vec![1.5, -2.5, 0.125];
    let vector =
        oracledb::Vector::Dense(oracledb::VectorData::Float64(values.clone()));
    let row = conn.query_row("select :1", &[&vector])?;
    match row.get(0)? {
        oracledb::Vector::Dense(oracledb::VectorData::Float64(
            fetched_values,
        )) => {
            assert_eq!(fetched_values, values);
        }
        _ => panic!("Expected Dense Float64 vector"),
    }
    Ok(())
}

/// Roundtrip test for DENSE INT8 vector
#[rstest]
fn test_2209(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let values = vec![-128, -1, 0, 1, 127];
    let vector =
        oracledb::Vector::Dense(oracledb::VectorData::Int8(values.clone()));
    let row = conn.query_row("select :1", &[&vector])?;
    match row.get(0)? {
        oracledb::Vector::Dense(oracledb::VectorData::Int8(
            fetched_values,
        )) => {
            assert_eq!(fetched_values, values);
        }
        _ => panic!("Expected Dense Int8 vector"),
    }
    Ok(())
}

/// Roundtrip test for DENSE BINARY vector
#[rstest]
fn test_2210(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let values = vec![0b10101010, 0b01010101];
    let vector =
        oracledb::Vector::Dense(oracledb::VectorData::Binary(values.clone()));
    let row = conn.query_row("select :1", &[&vector])?;
    match row.get(0)? {
        oracledb::Vector::Dense(oracledb::VectorData::Binary(
            fetched_values,
        )) => {
            assert_eq!(fetched_values, values);
        }
        _ => panic!("Expected Dense Binary vector"),
    }
    Ok(())
}

/// Roundtrip test for SPARSE FLOAT32 vector
#[rstest]
fn test_2211(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_sparse_vectors_supported(&conn) {
        return Ok(());
    }
    let indices = vec![0, 500, 999];
    let values = vec![1.5, 2.5, 3.5];
    let vector = oracledb::Vector::Sparse(oracledb::SparseVector::new(
        1000,
        indices.clone(),
        oracledb::VectorData::Float32(values.clone()),
    ));
    let row = conn.query_row("select :1", &[&vector])?;
    match row.get(0)? {
        oracledb::Vector::Sparse(sparse) => {
            assert_eq!(sparse.num_dimensions(), 1000);
            assert_eq!(sparse.indices(), indices);
            match sparse.values() {
                oracledb::VectorData::Float32(fetched_values) => {
                    assert_eq!(*fetched_values, values);
                }
                _ => panic!("Expected Float32 values"),
            }
        }
        _ => panic!("Expected Sparse vector"),
    }
    Ok(())
}

/// Roundtrip test for SPARSE FLOAT64 vector
#[rstest]
fn test_2212(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_sparse_vectors_supported(&conn) {
        return Ok(());
    }
    let indices = vec![0, 500, 999];
    let values = vec![1.5, 2.5, 3.5];
    let vector = oracledb::Vector::Sparse(oracledb::SparseVector::new(
        1000,
        indices.clone(),
        oracledb::VectorData::Float64(values.clone()),
    ));
    let row = conn.query_row("select :1", &[&vector])?;
    match row.get(0)? {
        oracledb::Vector::Sparse(sparse) => {
            assert_eq!(sparse.num_dimensions(), 1000);
            assert_eq!(sparse.indices(), indices);
            match sparse.values() {
                oracledb::VectorData::Float64(fetched_values) => {
                    assert_eq!(*fetched_values, values);
                }
                _ => panic!("Expected Float64 values"),
            }
        }
        _ => panic!("Expected Sparse vector"),
    }
    Ok(())
}

/// Roundtrip test for SPARSE INT8 vector
#[rstest]
fn test_2213(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_sparse_vectors_supported(&conn) {
        return Ok(());
    }
    let indices = vec![0, 50, 99];
    let values = vec![-128, 0, 127];
    let vector = oracledb::Vector::Sparse(oracledb::SparseVector::new(
        100,
        indices.clone(),
        oracledb::VectorData::Int8(values.clone()),
    ));
    let row = conn.query_row("select :1", &[&vector])?;
    match row.get(0)? {
        oracledb::Vector::Sparse(sparse) => {
            assert_eq!(sparse.num_dimensions(), 100);
            assert_eq!(sparse.indices(), indices);
            match sparse.values() {
                oracledb::VectorData::Int8(fetched_values) => {
                    assert_eq!(*fetched_values, values);
                }
                _ => panic!("Expected Int8 values"),
            }
        }
        _ => panic!("Expected Sparse vector"),
    }
    Ok(())
}

/// Test querying a single-element DENSE FLOAT32 VECTOR
#[rstest]
fn test_2214(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let sql = "select vector('[42.0]', 1, float32)";
    let row = conn.query_row(sql, &[])?;
    match row.get(0)? {
        oracledb::Vector::Dense(oracledb::VectorData::Float32(values)) => {
            assert_eq!(values, vec![42.0]);
        }
        _ => panic!("Expected Dense float32 vector"),
    }
    Ok(())
}

/// Test querying extreme-values DENSE FLOAT32 VECTOR
#[rstest]
fn test_2215(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let sql = format!(
        "select vector('[{}, {}, {}, {}]', 4, float32)",
        f32::MAX,
        f32::MIN,
        -f32::MAX,
        -f32::MIN,
    );
    let row = conn.query_row(&sql, &[])?;
    match row.get(0)? {
        oracledb::Vector::Dense(oracledb::VectorData::Float32(values)) => {
            assert_eq!(values[0], f32::MAX);
            assert_eq!(values[1], f32::MIN);
            assert_eq!(values[2], -f32::MAX);
            assert_eq!(values[3], -f32::MIN);
        }
        _ => panic!("Expected Dense Float32 vector"),
    }
    Ok(())
}

/// Test querying multiple types of vectors (DENSE and SPARSE, various data
/// types)
#[rstest]
fn test_2216(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let sql = "
        select
            vector('[1.0, 2.0]', 2, float32) as dense_f32,
            vector('[3.0, 4.0]', 2, float64) as dense_f64,
            vector('[5, 6]', 2, int8) as dense_int8,
            vector('[0, 1, 1, 0, 0, 1, 1, 0]', 64, binary) as dense_binary,
            vector('[5, [0,1], [1.0,2.0]]', 5, float32, sparse) as sparse_f32,
            vector('[5, [0,1], [1.5,2.5]]', 5, float64, sparse) as sparse_f64
    ";
    let row = conn.query_row(sql, &[])?;

    // DENSE FLOAT32
    match row.get(0)? {
        oracledb::Vector::Dense(oracledb::VectorData::Float32(v)) => {
            assert_eq!(v, vec![1.0, 2.0])
        }
        _ => panic!("Expected Dense Float32"),
    }

    // DENSE FLOAT64
    match row.get(1)? {
        oracledb::Vector::Dense(oracledb::VectorData::Float64(v)) => {
            assert_eq!(v, vec![3.0, 4.0])
        }
        _ => panic!("Expected Dense Float64"),
    }

    // DENSE INT8
    match row.get(2)? {
        oracledb::Vector::Dense(oracledb::VectorData::Int8(v)) => {
            assert_eq!(v, vec![5, 6])
        }
        _ => panic!("Expected Dense Int8"),
    }

    // DENSE BINARY
    match row.get(3)? {
        oracledb::Vector::Dense(oracledb::VectorData::Binary(v)) => {
            assert_eq!(v, vec![0, 1, 1, 0, 0, 1, 1, 0])
        }
        _ => panic!("Expected Dense Binary"),
    }

    // SPARSE FLOAT32
    match row.get(4)? {
        oracledb::Vector::Sparse(s) => {
            assert_eq!(s.num_dimensions(), 5);
            assert_eq!(s.indices(), &[0, 1]);
            match s.values() {
                oracledb::VectorData::Float32(vals) => {
                    assert_eq!(*vals, vec![1.0, 2.0])
                }
                _ => panic!("Expected Sparse Float32 values"),
            }
        }
        _ => panic!("Expected Sparse Float32 vector"),
    }

    // SPARSE FLOAT64
    match row.get(5)? {
        oracledb::Vector::Sparse(s) => {
            assert_eq!(s.num_dimensions(), 5);
            assert_eq!(s.indices(), &[0, 1]);
            match s.values() {
                oracledb::VectorData::Float64(vals) => {
                    assert_eq!(*vals, vec![1.5, 2.5])
                }
                _ => panic!("Expected Sparse Float64 values"),
            }
        }
        _ => panic!("Expected Sparse Float64 vector"),
    }
    Ok(())
}

/// Roundtrip test for 1536-dim FLOAT32 vector
#[rstest]
fn test_2217(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    // using power-of-2 fractions
    let expected: Vec<f32> = (0..1536).map(|i| (i as f32) * 0.5).collect();
    let vector = oracledb::Vector::Dense(oracledb::VectorData::Float32(
        expected.clone(),
    ));

    let row = conn.query_row("select :1", &[&vector])?;
    match row.get(0)? {
        oracledb::Vector::Dense(oracledb::VectorData::Float32(
            fetched_values,
        )) => {
            assert_eq!(fetched_values, expected);
        }
        _ => panic!("Expected Dense Float32 vector"),
    }
    Ok(())
}

#[rstest]
/// test VECTOR metadata
fn test_2218(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let cursor = conn.query(
        "select vector('[1, 2, 3]', 3, float32) as vec_col from dual",
        &[],
    )?;
    let columns = cursor.columns();
    assert_eq!(columns[0].name(), "VEC_COL");
    assert_eq!(columns[0].db_type(), &oracledb::DB_TYPE_VECTOR);
    Ok(())
}

#[rstest]
/// Tests NULL VECTOR fetches through Option<Vector>.
fn test_2219(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let row = conn.query_row("select to_vector(null) from dual", &[])?;
    let value: Option<oracledb::Vector> = row.get(0)?;
    assert!(value.is_none());
    Ok(())
}

#[rstest]
/// Tests that a VECTOR dimension mismatch is reported as a database error.
fn test_2220(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_vectors_supported(&conn) {
        return Ok(());
    }
    let error = match conn
        .query("select vector('[1, 2, 3]', 2, float32) from dual", &[])
    {
        Ok(_) => panic!("VECTOR dimension mismatch must fail"),
        Err(error) => error,
    };
    assert!(matches!(error.kind(), oracledb::ErrorKind::DbError(_)));
    Ok(())
}
