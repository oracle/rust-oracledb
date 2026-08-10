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
// test_2400_blob()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;
use std::io::{Read, Write};

/// Create a vector with a deterministic pattern of bytes.
fn make_patterned_bytes(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    for i in 0..size {
        data.push((i % 251) as u8);
    }
    data
}

#[rstest]
/// Basic CRUD for BLOB: insert and fetch to verify byte parity.
fn test_2400(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2400", "data blob")?;
    let data = b"test_2400_basic".to_vec();
    conn.execute("insert into test_2400 values (:1)", &[&data])?;
    let cursor = conn.query("select data from test_2400", &[])?;
    for row in cursor {
        let row = row?;
        let fetched: Vec<u8> = row.get(0)?;
        assert_eq!(fetched, data);
    }
    Ok(())
}

#[rstest]
/// Null handling for BLOB: insert and fetch NULL value.
fn test_2401(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2401", "data blob")?;
    let data: Option<Vec<u8>> = None;
    conn.execute("insert into test_2401 values (:1)", &[&data])?;
    let cursor = conn.query("select data from test_2401", &[])?;
    for row in cursor {
        let row = row?;
        let fetched: Option<Vec<u8>> = row.get(0)?;
        assert!(fetched.is_none());
    }
    Ok(())
}

#[rstest]
/// Empty BLOB: insert and fetch zero-length payload.
fn test_2402(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2402", "data blob")?;
    let data: Vec<u8> = Vec::new();
    conn.execute("insert into test_2402 values (:1)", &[&data])?;
    let cursor = conn.query("select data from test_2402", &[])?;
    for row in cursor {
        let row = row?;
        let fetched: Option<Vec<u8>> = row.get(0)?;
        assert!(fetched.is_none());
    }
    Ok(())
}

#[rstest]
/// Large BLOB: insert and fetch payload larger than typical buffers.
fn test_2403(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2403", "data blob")?;
    let data = make_patterned_bytes(1024 * 1024);
    conn.execute("insert into test_2403 values (:1)", &[&data])?;
    let cursor = conn.query("select data from test_2403", &[])?;
    for row in cursor {
        let row = row?;
        let fetched: Vec<u8> = row.get(0)?;
        assert_eq!(fetched, data);
    }
    Ok(())
}

#[rstest]
/// Multiple BLOB columns: fetch two BLOBs in one row.
fn test_2404(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard =
        common::create_table(&conn, "test_2404", "data1 blob, data2 blob")?;
    let data1 = b"test_2404_data1".to_vec();
    let data2 = b"test_2404_data2".to_vec();
    conn.execute("insert into test_2404 values (:1, :2)", &[&data1, &data2])?;
    let cursor = conn.query("select data1, data2 from test_2404", &[])?;
    for row in cursor {
        let row = row?;
        let fetched1: Vec<u8> = row.get(0)?;
        let fetched2: Vec<u8> = row.get(1)?;
        assert_eq!(fetched1, data1);
        assert_eq!(fetched2, data2);
    }
    Ok(())
}

#[rstest]
/// Update/overwrite BLOB: ensure replacement does not corrupt data.
fn test_2405(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2405", "data blob")?;
    let data1 = b"test_2405_before".to_vec();
    let data2 = b"test_2405_after".to_vec();
    conn.execute("insert into test_2405 values (:1)", &[&data1])?;
    conn.execute("update test_2405 set data = :1", &[&data2])?;
    let cursor = conn.query("select data from test_2405", &[])?;
    for row in cursor {
        let row = row?;
        let fetched: Vec<u8> = row.get(0)?;
        assert_eq!(fetched, data2);
    }
    Ok(())
}

/// Validates that enabling 'fetch_lobs()' returns a 'Lob' locator for a BLOB
/// column and that 'Read::read_to_end()' can fetch the original bytes via the
/// TTC LOB op message.
#[rstest]
fn test_2406(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let payload: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let row = conn
        .statement("select to_blob(:1) from dual")?
        .fetch_lobs()
        .query_row(&[&payload])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    let mut read_back = Vec::new();
    lob.read_to_end(&mut read_back)?;
    assert_eq!(read_back, payload);
    Ok(())
}

/// Validates 'open()'/'close()'/'is_open()' for a persistent BLOB locator.
#[rstest]
fn test_2407(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2407", "data blob")?;
    let payload: Vec<u8> = vec![1, 2, 3, 4];
    conn.execute("insert into test_2407 values (:1)", &[&payload])?;
    let row = conn
        .statement("select data from test_2407")?
        .fetch_lobs()
        .query_row(&[])?;
    let mut lob: oracledb::Lob = row.get(0)?;

    // Basic lifecycle test
    assert!(!lob.is_open()?, "LOB should initially be closed");

    // Keep the transaction open (no commit) because LOB open/write needs
    // an active transaction.
    lob.open()?;
    assert!(lob.is_open()?, "LOB should be open after open()");

    lob.close()?;
    assert!(!lob.is_open()?, "LOB should be closed after close()");

    Ok(())
}

/// Validates writing to a persistent BLOB locator using 'Write::write_all()'.
#[rstest]
fn test_2408(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2408", "data blob")?;
    let payload = vec![8, 7, 6, 5, 4, 3, 2, 1];
    conn.execute("insert into test_2408 values (empty_blob())", &[])?;

    let row = conn
        .statement("select data from test_2408")?
        .fetch_lobs()
        .query_row(&[])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    lob.write_all(&payload)?;

    let row = conn
        .statement("select data from test_2408")?
        .fetch_lobs()
        .query_row(&[])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    let mut read_back = Vec::new();
    lob.read_to_end(&mut read_back)?;
    assert_eq!(read_back, payload);
    Ok(())
}

/// Validates trimming a persistent BLOB locator.
#[rstest]
fn test_2409(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2409", "data blob")?;
    let payload = vec![1, 2, 3, 4, 5, 6];
    conn.execute("insert into test_2409 values (:1)", &[&payload])?;

    let row = conn
        .statement("select data from test_2409")?
        .fetch_lobs()
        .query_row(&[])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    lob.trim(3)?;

    let row = conn
        .statement("select data from test_2409")?
        .fetch_lobs()
        .query_row(&[])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    let mut read_back = Vec::new();
    lob.read_to_end(&mut read_back)?;
    assert_eq!(read_back, &payload[..3]);
    Ok(())
}

/// Validates reading a BLOB locator using a small caller buffer.
#[rstest]
fn test_2410(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let payload = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let row = conn
        .statement("select to_blob(:1) from dual")?
        .fetch_lobs()
        .query_row(&[&payload])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    let mut read_back = Vec::new();
    let mut buf = [0; 1];
    loop {
        let num_bytes = lob.read(&mut buf)?;
        if num_bytes == 0 {
            break;
        }
        read_back.extend_from_slice(&buf[..num_bytes]);
    }
    assert_eq!(read_back, payload);
    Ok(())
}

/// Validates BLOB locator size and chunk size metadata.
#[rstest]
fn test_2411(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2411", "data blob")?;
    let payload = vec![1, 2, 3, 4, 5];
    conn.execute("insert into test_2411 values (:1)", &[&payload])?;
    let row = conn
        .statement("select data from test_2411")?
        .fetch_lobs()
        .query_row(&[])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    assert_eq!(lob.get_size()?, payload.len());
    assert!(lob.get_chunk_size()? > 0);
    Ok(())
}

/// Validates null BLOB locator handling when fetch_lobs() is enabled.
#[rstest]
fn test_2412(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row = conn
        .statement("select to_blob(null) from dual")?
        .fetch_lobs()
        .query_row(&[])?;
    let fetched: Option<oracledb::Lob> = row.get(0)?;
    assert!(fetched.is_none());
    Ok(())
}

/// Validates repeated reads from separately fetched BLOB locators.
#[rstest]
fn test_2413(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let payload = vec![9, 8, 7, 6, 5];
    for _ in 0..2 {
        let row = conn
            .statement("select to_blob(:1) from dual")?
            .fetch_lobs()
            .query_row(&[&payload])?;
        let mut lob: oracledb::Lob = row.get(0)?;
        let mut read_back = Vec::new();
        lob.read_to_end(&mut read_back)?;
        assert_eq!(read_back, payload);
    }
    Ok(())
}

/// Validates DBMS_LOB.SUBSTR() on BLOB data returns the requested bytes.
#[rstest]
fn test_2414(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2414", "data blob")?;
    let payload = vec![1, 2, 3, 4, 5, 6, 7, 8];
    conn.execute("insert into test_2414 values (:1)", &[&payload])?;
    let row = conn
        .query_row("select dbms_lob.substr(data, 3, 4) from test_2414", &[])?;
    let substr: Vec<u8> = row.get(0)?;
    assert_eq!(substr, vec![4, 5, 6]);
    Ok(())
}

/// Validates fetching the same BLOB column twice in one row.
#[rstest]
fn test_2415(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2415", "data blob")?;
    let payload = vec![9, 7, 5, 3, 1];
    conn.execute("insert into test_2415 values (:1)", &[&payload])?;
    let row = conn.query_row("select data, data from test_2415", &[])?;
    let first: Vec<u8> = row.get(0)?;
    let second: Vec<u8> = row.get(1)?;
    assert_eq!(first, payload);
    assert_eq!(second, payload);
    Ok(())
}

#[rstest]
/// Validates that a LOB operation fails after its owning connection is closed.
fn test_2416(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let mut conn = conn;
    let payload = vec![1, 2, 3, 4];
    let mut lob: oracledb::Lob = {
        let row = conn
            .statement("select to_blob(:1) from dual")?
            .fetch_lobs()
            .query_row(&[&payload])?;
        row.get(0)?
    };
    conn.close()?;
    let mut buffer = [0_u8; 4];
    assert!(lob.read(&mut buffer).is_err());
    Ok(())
}
