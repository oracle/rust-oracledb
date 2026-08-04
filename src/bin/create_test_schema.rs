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
// create_test_schema.rs
//
// Creates the schema used for testing the library.
//-----------------------------------------------------------------------------

extern crate oracledb;

fn main() -> Result<(), oracledb::Error> {
    let test_config = oracledb::get_test_config();
    println!("Creating test schema {}...", test_config.user);
    let admin_conn = oracledb::connect(test_config.admin_config.clone())?;
    let drop_statement = format!("drop user {} cascade", test_config.user);
    let _ = admin_conn.execute(&drop_statement, &[]);
    let create_statement = format!(
        "create user {} identified by {}",
        test_config.user, test_config.password
    );
    admin_conn.execute(&create_statement, &[])?;
    let grant_statement = format!(
        r#"
        grant
            create session,
            create table,
            create procedure,
            create type,
            create view,
            select any dictionary,
            change notification,
            unlimited tablespace,
            ctxapp
        to {}
        "#,
        test_config.user
    );
    admin_conn.execute(&grant_statement, &[])?;
    println!("Schema successfully created.");
    Ok(())
}
