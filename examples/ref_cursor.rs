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
// ref_cursor.rs
//
// Shows how to call a PL/SQL procedure to get a REF cursor and fetch from it.
//-----------------------------------------------------------------------------

mod common;

fn main() -> Result<(), oracledb::Error> {
    let config = common::get_sample_config()?;
    let connection = oracledb::connect(config)?;

    connection.execute(
        r#"
        create or replace procedure rso_examples_ref_cursor (
            a_NumVal number,
            a_RefCursor out sys_refcursor
        ) as
        begin
            open a_RefCursor for
            select level * 100
            from dual connect by level <= a_NumVal;
        end;
        "#,
        &[],
    )?;

    let mut result = connection.execute(
        "begin rso_examples_ref_cursor(:1, :2); end;",
        &[&3, &&oracledb::DB_TYPE_CURSOR],
    )?;

    let cursor: oracledb::Cursor = result.out_bind_data().take(0)?;
    for row_result in cursor {
        let row = row_result?;
        let value: usize = row.get(0)?;
        println!("Fetched {value}");
    }

    Ok(())
}
