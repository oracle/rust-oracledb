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
// plsql_function.rs
//
// Shows how to call a PL/SQL function and get its return value.
//-----------------------------------------------------------------------------

mod common;

fn main() -> Result<(), oracledb::Error> {
    let config = common::get_sample_config()?;
    let connection = oracledb::connect(config)?;

    connection.execute(
        r#"
        create or replace function rso_examples_func (
            a_NumVal number
        ) return number as
        begin
            return a_NumVal * 2;
        end;
        "#,
        &[],
    )?;

    let mut result = connection.execute(
        "begin :1 := rso_examples_func(:2); end;",
        &[&oracledb::DB_TYPE_NUMBER, &19],
    )?;

    let return_val: i32 = result.out_bind_data().get(0)?;

    println!("Return value: {return_val}");

    Ok(())
}
