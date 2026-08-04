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
// drop_test_schema.rs
//
// Drops the schema used for testing the library.
//-----------------------------------------------------------------------------

extern crate oracledb;

fn main() -> Result<(), oracledb::Error> {
    let test_config = oracledb::get_test_config();
    println!("Dropping test schema {}...", test_config.user);
    let admin_conn = oracledb::connect(test_config.admin_config.clone())?;
    let drop_statement = format!("drop user {} cascade", test_config.user);
    if let Err(err) = admin_conn.execute(&drop_statement, &[]) {
        if err.to_string().starts_with("ORA-01918") {
            println!("Schema already dropped.");
        } else {
            panic!("{err}");
        }
    } else {
        println!("Schema successfully dropped.");
    }
    Ok(())
}
