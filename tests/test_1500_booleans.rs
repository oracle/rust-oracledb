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
// test_1500_booleans()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;

#[rstest]
/// test query with BOOLEAN
fn test_1500(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_boolean_supported(&conn) {
        return Ok(());
    }
    let cursor = conn.query("select true from dual", &[])?;
    for row in cursor {
        let row = row?;
        let value: bool = row.get(0)?;
        assert_eq!(value, true);
    }
    Ok(())
}

#[rstest]
/// test binding BOOLEAN
fn test_1501(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_boolean_supported(&conn) {
        return Ok(());
    }
    let options = [true, false, false, true];
    for val in options {
        let cursor = conn.query("select :1 from dual", &[&val])?;
        for row in cursor {
            let row = row?;
            let fetched_val: bool = row.get(0)?;
            assert_eq!(fetched_val, val);
        }
    }
    Ok(())
}

#[rstest]
/// test fetching null values for BOOLEAN
fn test_1502(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_boolean_supported(&conn) {
        return Ok(());
    }
    let cursor = conn.query("select cast(null as boolean) from dual", &[])?;
    for row in cursor {
        let row = row?;
        let fetched_val: Option<bool> = row.get(0)?;
        assert!(fetched_val.is_none());
    }
    Ok(())
}
