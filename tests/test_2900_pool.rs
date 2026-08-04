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
// test_2900_pool()
//-----------------------------------------------------------------------------

use oracledb;

fn pool_config(
    min_connections: usize,
    max_connections: usize,
    connection_increment: usize,
) -> Result<oracledb::PoolConfig, oracledb::Error> {
    let test_config = oracledb::get_test_config();
    Ok(oracledb::PoolConfig::default()
        .set_credentials(&test_config.user, &test_config.password)
        .set_connect_string(&test_config.connect_string)?
        .set_min_connections(min_connections)
        .set_max_connections(max_connections)
        .set_connection_increment(connection_increment))
}

#[test]
/// Tests pool acquire/release, accounting and close behavior.
fn test_2900() -> Result<(), oracledb::Error> {
    let mut pool = oracledb::create_pool(pool_config(0, 2, 1)?)?;
    assert_eq!(pool.busy_count()?, 0);

    let mut connection = pool.acquire()?;
    assert_eq!(pool.busy_count()?, 1);
    assert!(pool.open_count()? >= 1);
    connection.ping()?;
    connection.close()?;
    assert_eq!(pool.busy_count()?, 0);

    let connection = pool.acquire()?;
    assert_eq!(pool.busy_count()?, 1);
    drop(connection);
    assert_eq!(pool.busy_count()?, 0);

    pool.close()?;
    assert!(pool.acquire().is_err());
    assert!(pool.open_count().is_err());
    assert!(pool.busy_count().is_err());
    assert!(pool.close().is_err());
    Ok(())
}

#[test]
/// Tests pool size validation before the manager thread is created.
fn test_2901() -> Result<(), oracledb::Error> {
    assert!(oracledb::create_pool(pool_config(2, 1, 1)?).is_err());
    assert!(oracledb::create_pool(pool_config(1, 2, 0)?).is_err());
    Ok(())
}

#[test]
/// Tests that a close request with a checked-out connection is rejected and
/// the pool remains usable after the rejection.
fn test_2902() -> Result<(), oracledb::Error> {
    let mut pool = oracledb::create_pool(pool_config(0, 1, 1)?)?;
    let mut connection = pool.acquire()?;

    let err = pool
        .close()
        .expect_err("closing a pool with a checked-out connection must fail");
    assert!(matches!(
        err.kind(),
        oracledb::ErrorKind::PoolHasBusyConnections
    ));
    connection.ping()?;

    connection.close()?;
    assert_eq!(pool.busy_count()?, 0);
    pool.close()?;
    Ok(())
}

#[test]
/// Tests dynamic growth is bounded by max_connections for busy connections.
fn test_2903() -> Result<(), oracledb::Error> {
    let pool = oracledb::create_pool(pool_config(0, 2, 1)?)?;
    let mut first = pool.acquire()?;
    let mut second = pool.acquire()?;

    assert_eq!(pool.busy_count()?, 2);
    assert_eq!(pool.open_count()?, 2);
    first.ping()?;
    second.ping()?;

    first.close()?;
    second.close()?;
    assert_eq!(pool.busy_count()?, 0);
    assert_eq!(pool.open_count()?, 2);
    Ok(())
}
