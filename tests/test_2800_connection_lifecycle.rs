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
// test_2800_connection_lifecycle()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;

#[rstest]
/// Tests connection information, a healthy ping and explicit close behavior.
fn test_2800(mut conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    conn.ping()?;
    assert!(!conn.db_name()?.is_empty());
    assert!(!conn.instance_name()?.is_empty());
    assert!(!conn.service_name()?.is_empty());
    assert!(conn.session_id()? > 0);
    assert!(conn.serial_num()? > 0);
    assert!(conn.max_identifier_length()? >= 30);
    assert!(conn.max_open_cursors()? > 0);
    assert_eq!(conn.call_timeout()?, None);
    assert_eq!(conn.last_warning()?, None);

    conn.close()?;
    for result in [conn.ping(), conn.commit(), conn.rollback(), conn.close()] {
        let err =
            result.expect_err("operation on a closed connection must fail");
        assert!(matches!(err.kind(), oracledb::ErrorKind::NotConnected));
    }
    Ok(())
}
