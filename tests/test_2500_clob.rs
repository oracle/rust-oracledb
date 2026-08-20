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
// test_2500_clob()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;
use std::io::{Read, Write};

#[rstest]
/// Validates reading a CLOB locator using 'Read::read_to_string()'.
fn test_2500(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let text = "Hello CLOB - café".to_string();
    let row = conn
        .statement("select to_clob(:1) from dual")?
        .fetch_lobs()
        .query_row(&[&text])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    let mut read_back = String::new();
    lob.read_to_string(&mut read_back)?;
    assert_eq!(read_back, text);
    Ok(())
}

#[rstest]
/// Validates reading an NCLOB locator using 'Read::read_to_string()'.
fn test_2501(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let text = "Hello NCLOB - 世界".to_string();
    let row = conn
        .statement("select to_nclob(:1) from dual")?
        .fetch_lobs()
        .query_row(&[&text])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    let mut read_back = String::new();
    lob.read_to_string(&mut read_back)?;
    assert_eq!(read_back, text);
    Ok(())
}

#[rstest]
/// Validates writing to a persistent CLOB locator using 'Write::write_all()'.
fn test_2502(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2502", "data clob")?;
    let payload = "Hello CLOB - café".to_string();
    conn.execute("insert into test_2502 values (empty_clob())", &[])?;

    let row = conn
        .statement("select data from test_2502")?
        .fetch_lobs()
        .query_row(&[])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    lob.write_all(payload.as_bytes())?;

    let row = conn
        .statement("select data from test_2502")?
        .fetch_lobs()
        .query_row(&[])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    let mut read_back = String::new();
    lob.read_to_string(&mut read_back)?;
    assert_eq!(read_back, payload);
    Ok(())
}

#[rstest]
/// Validates writing to a persistent NCLOB locator using 'Write::write_all()'.
fn test_2503(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2503", "data nclob")?;
    let payload = "Hello NCLOB - 世界".to_string();
    conn.execute("insert into test_2503 values (empty_clob())", &[])?;

    let row = conn
        .statement("select data from test_2503")?
        .fetch_lobs()
        .query_row(&[])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    lob.write_all(payload.as_bytes())?;

    let row = conn
        .statement("select data from test_2503")?
        .fetch_lobs()
        .query_row(&[])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    let mut read_back = String::new();
    lob.read_to_string(&mut read_back)?;
    assert_eq!(read_back, payload);
    Ok(())
}

#[rstest]
/// Validates null CLOB/NCLOB locator handling when fetch_lobs() is enabled.
fn test_2504(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row = conn
        .statement("select to_clob(null), to_nclob(null) from dual")?
        .fetch_lobs()
        .query_row(&[])?;
    let clob: Option<oracledb::Lob> = row.get(0)?;
    let nclob: Option<oracledb::Lob> = row.get(1)?;
    assert!(clob.is_none());
    assert!(nclob.is_none());
    Ok(())
}

#[rstest]
/// Validates repeated reads from separately fetched CLOB locators.
fn test_2505(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let text = "repeatable CLOB read - café".to_string();
    for _ in 0..2 {
        let row = conn
            .statement("select to_clob(:1) from dual")?
            .fetch_lobs()
            .query_row(&[&text])?;
        let mut lob: oracledb::Lob = row.get(0)?;
        let mut read_back = String::new();
        lob.read_to_string(&mut read_back)?;
        assert_eq!(read_back, text);
    }
    Ok(())
}

#[rstest]
/// Validates NCLOB fetch as String when LOB locators are not requested.
fn test_2506(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let text = "NCLOB as String - 世界".to_string();
    let row = conn.query_row("select to_nclob(:1) from dual", &[&text])?;
    let fetched: String = row.get(0)?;
    assert_eq!(fetched, text);
    Ok(())
}

#[rstest]
/// Validates DBMS_LOB.SUBSTR() on CLOB data returns the requested text.
fn test_2507(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2507", "data clob")?;
    let payload = "abcdefghij".to_string();
    conn.execute("insert into test_2507 values (:1)", &[&payload])?;
    let row = conn
        .query_row("select dbms_lob.substr(data, 4, 3) from test_2507", &[])?;
    let substr: String = row.get(0)?;
    assert_eq!(substr, "cdef");
    Ok(())
}

#[rstest]
/// Validates fetching the same CLOB column twice in one row.
fn test_2508(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2508", "data clob")?;
    let payload = "same CLOB twice".to_string();
    conn.execute("insert into test_2508 values (:1)", &[&payload])?;
    let row = conn.query_row("select data, data from test_2508", &[])?;
    let first: String = row.get(0)?;
    let second: String = row.get(1)?;
    assert_eq!(first, payload);
    assert_eq!(second, payload);
    Ok(())
}

#[rstest]
/// Validates that invalid UTF-8 is rejected before a CLOB write is sent.
fn test_2509(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2510", "data clob")?;
    conn.execute("insert into test_2510 values (empty_clob())", &[])?;
    let row = conn
        .statement("select data from test_2510")?
        .fetch_lobs()
        .query_row(&[])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    let error = lob
        .write(&[0xff])
        .expect_err("invalid UTF-8 must be rejected");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    Ok(())
}

#[rstest]
/// Validates that a CLOB read with an undersized byte buffer fails without
/// consuming data, so the same UTF-8 character can be read successfully.
fn test_2510(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let row = conn
        .statement("select to_clob(:1) from dual")?
        .fetch_lobs()
        .query_row(&[&"é"])?;
    let mut lob: oracledb::Lob = row.get(0)?;

    let mut small_buffer = [0_u8; 1];
    let error = lob.read(&mut small_buffer).expect_err(
        "a one-byte buffer cannot hold a two-byte UTF-8 character",
    );
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);

    let mut buffer = [0_u8; 2];
    let bytes_read = lob.read(&mut buffer)?;
    assert_eq!(bytes_read, 2);
    assert_eq!(&buffer, "é".as_bytes());
    assert_eq!(lob.read(&mut buffer)?, 0);
    Ok(())
}

#[rstest]
/// Validates trimming a written CLOB updates the locator length and stored
/// data consistently.
fn test_2511(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(&conn, "test_2511", "data clob")?;
    conn.execute("insert into test_2511 values (empty_clob())", &[])?;

    let row = conn
        .statement("select data from test_2511")?
        .fetch_lobs()
        .query_row(&[])?;
    let mut lob: oracledb::Lob = row.get(0)?;
    lob.write_all(b"abcdef")?;
    assert_eq!(lob.get_size()?, 6);
    lob.trim(3)?;
    assert_eq!(lob.get_size()?, 3);

    let row = conn.query_row("select data from test_2511", &[])?;
    let fetched: String = row.get(0)?;
    assert_eq!(fetched, "abc");
    Ok(())
}
