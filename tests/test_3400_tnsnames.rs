//-----------------------------------------------------------------------------
// Copyright (c) 2026, Oracle and/or its affiliates.
//
// This software is dual-licensed to you under the Universal Permissive License
// (UPL) 1.0 as shown at https://oss.oracle.com/licenses/upl and Apache License
// 2.0. You may choose either license.
//
// If you elect to use the software under the Apache License, Version 2.0,
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
// test_3400_tnsnames()
//-----------------------------------------------------------------------------

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_config_dir() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "oracledb-tnsnames-test-{}-{suffix}",
        std::process::id()
    ));
    fs::create_dir(&path).unwrap();
    path
}

#[test]
/// Tests case-insensitive aliases, comma-separated aliases, and IFILE
/// inclusion.
fn test_3400() {
    let dir = make_config_dir();
    let main_connect_string = "host_3400:1522/service_name_3400";
    let main_descriptor = "(DESCRIPTION=(ADDRESS=(PROTOCOL=tcp)\
        (HOST=host_3400)(PORT=1522))\
        (CONNECT_DATA=(SERVICE_NAME=service_name_3400)))";
    let child_descriptor = "(DESCRIPTION=(ADDRESS=(PROTOCOL=tcp)\
        (HOST=child_host_3400)(PORT=1521))\
        (CONNECT_DATA=(SERVICE_NAME=child_service_name_3400)))";
    let child_file_name = "child.ora";
    fs::write(
        dir.join("tnsnames.ora"),
        format!(
            "# primary aliases\nMAIN, MAIN_ALIAS = {main_connect_string}\n\
                 IFILE = {child_file_name}\n"
        ),
    )
    .unwrap();
    fs::write(
        dir.join("child.ora"),
        format!("CHILD = {child_descriptor}\n"),
    )
    .unwrap();

    let dir_str = dir.to_str().unwrap();
    let main = oracledb::Config::default()
        .set_config_dir(dir_str)
        .set_connect_string("main_alias")
        .unwrap();
    assert_eq!(main.get_connect_descriptor(), main_descriptor);
    let child = oracledb::Config::default()
        .set_config_dir(dir_str)
        .set_connect_string("ChIlD")
        .unwrap();
    assert_eq!(child.get_connect_descriptor(), child_descriptor);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
/// Tests missing aliases and IFILE cycles.
fn test_3401() {
    let missing_dir = make_config_dir();
    fs::write(missing_dir.join("tnsnames.ora"), "MAIN = db/service\n")
        .unwrap();
    let missing = oracledb::Config::default()
        .set_config_dir(missing_dir.to_str().unwrap())
        .set_connect_string("other");
    assert!(matches!(
        missing,
        Err(err) if matches!(
            err.kind(),
            oracledb::ErrorKind::TnsAliasNotFound(_, _)
        )
    ));
    fs::remove_dir_all(missing_dir).unwrap();

    let cycle_dir = make_config_dir();
    fs::write(cycle_dir.join("tnsnames.ora"), "IFILE = child.ora\n").unwrap();
    fs::write(cycle_dir.join("child.ora"), "IFILE = tnsnames.ora\n").unwrap();
    let cycle = oracledb::Config::default()
        .set_config_dir(cycle_dir.to_str().unwrap())
        .set_connect_string("main");
    assert!(matches!(
        cycle,
        Err(err) if matches!(
            err.kind(),
            oracledb::ErrorKind::IfileCycleDetected(_, _)
        )
    ));
    fs::remove_dir_all(cycle_dir).unwrap();

    let missing_file_dir = make_config_dir();
    let missing_file = oracledb::Config::default()
        .set_config_dir(missing_file_dir.to_str().unwrap())
        .set_connect_string("main");
    assert!(missing_file.is_err());
    fs::remove_dir_all(missing_file_dir).unwrap();

    let invalid_entry_dir = make_config_dir();
    fs::write(
        invalid_entry_dir.join("tnsnames.ora"),
        "MAIN = invalid connect string\n",
    )
    .unwrap();
    let invalid_entry = oracledb::Config::default()
        .set_config_dir(invalid_entry_dir.to_str().unwrap())
        .set_connect_string("main");
    assert!(matches!(
        invalid_entry,
        Err(err) if matches!(
            err.kind(),
            oracledb::ErrorKind::InvalidConnectString(_, _)
        )
    ));
    fs::remove_dir_all(invalid_entry_dir).unwrap();
}
