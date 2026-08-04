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
// test_3000_pool_config()
//-----------------------------------------------------------------------------

use std::time::Duration;

#[test]
/// Tests documented PoolConfig defaults and explicit overrides.
fn test_3000() {
    let config = oracledb::PoolConfig::default();
    assert_eq!(config.min_connections(), 1);
    assert_eq!(config.max_connections(), 2);
    assert_eq!(config.connection_increment(), 1);
    assert_eq!(config.ping_interval(), Some(Duration::from_secs(60)));
    assert_eq!(config.ping_timeout(), Duration::from_secs(5));
    assert_eq!(config.stmtcachesize(), 20);

    let config = oracledb::PoolConfig::default()
        .set_min_connections(2)
        .set_max_connections(7)
        .set_connection_increment(3)
        .set_ping_interval(None)
        .set_ping_timeout(Duration::ZERO)
        .set_stmtcachesize(0);
    assert_eq!(config.min_connections(), 2);
    assert_eq!(config.max_connections(), 7);
    assert_eq!(config.connection_increment(), 3);
    assert_eq!(config.ping_interval(), None);
    assert_eq!(config.ping_timeout(), Duration::ZERO);
    assert_eq!(config.stmtcachesize(), 0);
}

#[test]
/// Tests pool-size validation occurs before connection configuration is used.
fn test_3001() {
    let err = match oracledb::create_pool(
        oracledb::PoolConfig::default()
            .set_min_connections(2)
            .set_max_connections(1),
    ) {
        Ok(_) => panic!("max_connections below min_connections must fail"),
        Err(err) => err,
    };
    assert!(matches!(
        err.kind(),
        oracledb::ErrorKind::PoolMaxLessThanMin
    ));

    let err = match oracledb::create_pool(
        oracledb::PoolConfig::default()
            .set_min_connections(1)
            .set_max_connections(2)
            .set_connection_increment(0),
    ) {
        Ok(_) => panic!("zero increment with a variable-sized pool must fail"),
        Err(err) => err,
    };
    assert!(matches!(err.kind(), oracledb::ErrorKind::PoolIncrementZero));
}
