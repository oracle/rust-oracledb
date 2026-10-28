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
// Common module for testing the oracledb crate.
//-----------------------------------------------------------------------------

use rstest::*;

pub struct PackageGuard<'a> {
    conn: &'a oracledb::Connection,
    name: &'a str,
}

impl<'a> PackageGuard<'a> {
    /// Creates the package with the given definition.
    fn create_package(&self, definition: &str) -> Result<(), oracledb::Error> {
        let sql =
            format!("create package {} as {} end;", self.name, definition);
        self.conn.execute(&sql, &[])?;
        Ok(())
    }

    /// Drops the package from the database but ignores the error if the
    /// package does not exist.
    fn drop_package(&self) -> Result<(), oracledb::Error> {
        let sql = format!("drop package {}", self.name);
        let result = self.conn.execute(&sql, &[]);
        if let Err(err) = result
            && let oracledb::ErrorKind::DbError(db_error) = err.kind()
            && db_error.code() != 4043
        {
            return Err(err);
        }
        Ok(())
    }
}

impl Drop for PackageGuard<'_> {
    fn drop(&mut self) {
        let _ = self.drop_package();
    }
}

pub struct TableGuard<'a> {
    conn: &'a oracledb::Connection,
    table_name: &'a str,
}

impl TableGuard<'_> {
    /// Creates the table with the given definition.
    fn create_table(
        &self,
        definition: &str,
        options: &str,
    ) -> Result<(), oracledb::Error> {
        let sql = format!(
            "create table {} ({}) {}",
            self.table_name, definition, options
        );
        self.conn.execute(&sql, &[])?;
        Ok(())
    }

    /// Drops the table from the database but ignores the error if the table
    /// does not exist.
    fn drop_table(&self) -> Result<(), oracledb::Error> {
        let sql = format!("drop table {} purge", self.table_name);
        let result = self.conn.execute(&sql, &[]);
        if let Err(err) = result
            && let oracledb::ErrorKind::DbError(db_error) = err.kind()
            && db_error.code() != 942
        {
            return Err(err);
        }
        Ok(())
    }
}

impl Drop for TableGuard<'_> {
    fn drop(&mut self) {
        let _ = self.drop_table();
    }
}

pub struct TypeGuard<'a> {
    conn: &'a oracledb::Connection,
    type_name: &'a str,
}

impl<'a> TypeGuard<'a> {
    /// Creates the type with the given definition.
    fn create_type(&self, definition: &str) -> Result<(), oracledb::Error> {
        let sql = format!("create type {} as {}", self.type_name, definition);
        self.conn.execute(&sql, &[])?;
        Ok(())
    }

    /// Drops the type from the database but ignores the error if the type
    /// does not exist.
    fn drop_type(&self) -> Result<(), oracledb::Error> {
        let sql = format!("drop type {} force", self.type_name);
        let result = self.conn.execute(&sql, &[]);
        if let Err(err) = result
            && let oracledb::ErrorKind::DbError(db_error) = err.kind()
            && db_error.code() != 4043
        {
            return Err(err);
        }
        Ok(())
    }
}

impl Drop for TypeGuard<'_> {
    fn drop(&mut self) {
        let _ = self.drop_type();
    }
}

/// Returns a boolean indicating if the database to which the connection has
/// been established is at the specified minimum version.
fn is_db_min_version(
    conn: &oracledb::Connection,
    major_version: usize,
    minor_version: usize,
) -> bool {
    let version = conn.version().unwrap();
    version.0 > major_version
        || (version.0 == major_version && version.1 >= minor_version)
}

#[allow(dead_code)]
#[fixture]
/// Fixture that returns a connection to use for performing tests.
pub fn conn() -> oracledb::Connection {
    let config = oracledb::get_test_config().config.clone();
    let conn = oracledb::connect(config).unwrap();
    conn.execute("alter session set time_zone = 'UTC'", &[]).unwrap();
    conn
}

#[allow(dead_code)]
/// Creates the package with the given name and definition.
pub fn create_package<'a>(
    conn: &'a oracledb::Connection,
    name: &'a str,
    definition: &str,
) -> Result<PackageGuard<'a>, oracledb::Error> {
    let guard = PackageGuard { conn, name };
    guard.drop_package()?;
    guard.create_package(definition)?;
    Ok(guard)
}

#[allow(dead_code)]
/// Creates the table with the given name and definition.
pub fn create_table<'a>(
    conn: &'a oracledb::Connection,
    table_name: &'a str,
    definition: &str,
) -> Result<TableGuard<'a>, oracledb::Error> {
    create_table_with_options(conn, table_name, definition, "")
}

#[allow(dead_code)]
/// Creates the table with the given name and definition and options.
pub fn create_table_with_options<'a>(
    conn: &'a oracledb::Connection,
    table_name: &'a str,
    definition: &str,
    options: &str,
) -> Result<TableGuard<'a>, oracledb::Error> {
    let guard = TableGuard { conn, table_name };
    guard.drop_table()?;
    guard.create_table(definition, options)?;
    Ok(guard)
}

#[allow(dead_code)]
/// Creates the type with the given name and definition.
pub fn create_type<'a>(
    conn: &'a oracledb::Connection,
    type_name: &'a str,
    definition: &str,
) -> Result<TypeGuard<'a>, oracledb::Error> {
    let guard = TypeGuard { conn, type_name };
    guard.drop_type()?;
    guard.create_type(definition)?;
    Ok(guard)
}

#[allow(dead_code)]
/// Returns whether the connection is to an Oracle Cloud database.
pub fn is_on_oracle_cloud(
    conn: &oracledb::Connection,
) -> Result<bool, oracledb::Error> {
    let row = conn.query_row(
        "select sys_context('userenv', 'cloud_service') from dual",
        &[],
    )?;
    let service_name: Option<String> = row.get(0)?;
    Ok(service_name.is_some())
}

/// Prints the reason the test is being skipped and returns a boolean as a
/// convenience to the caller. The word SKIPPED is prepended to the message
/// before it is printed so that it can easily be read from the test output.
pub fn skip_test(reason: &str) -> bool {
    println!("SKIPPED: {reason}");
    true
}

#[allow(dead_code)]
/// Returns a boolean indicating if the connection can be used for testing
/// domains and annotations on table columns.
pub fn skip_unless_domains_supported(conn: &oracledb::Connection) -> bool {
    if is_db_min_version(conn, 23, 0) {
        false
    } else {
        skip_test("database does not support domains (requires 23+)")
    }
}

#[allow(dead_code)]
/// Returns a boolean indicating if the connection can be used for performing
/// tests with native booleans (requires Oracle Database 23 or higher).
pub fn skip_unless_native_boolean_supported(
    conn: &oracledb::Connection,
) -> bool {
    if is_db_min_version(conn, 23, 0) {
        false
    } else {
        skip_test("database does not support native boolean (requires 23+)")
    }
}

#[allow(dead_code)]
/// Returns a boolean indicating if the connection can be used for performing
/// tests with native JSON (requires Oracle Database 21 or higher).
pub fn skip_unless_native_json_supported(conn: &oracledb::Connection) -> bool {
    if is_db_min_version(conn, 21, 0) {
        false
    } else {
        skip_test("database does not support native JSON (requires 21+)")
    }
}

#[allow(dead_code)]
/// Returns a boolean indicating if the connection can be used for performing
/// tests with sparse vectors (requires Oracle Database 23.7 or higher).
pub fn skip_unless_sparse_vectors_supported(
    conn: &oracledb::Connection,
) -> bool {
    if is_db_min_version(conn, 23, 7) {
        false
    } else {
        skip_test("database does not support sparse vectors (requires 23.7+)")
    }
}

#[allow(dead_code)]
/// Returns a boolean indicating if the connection can be used for performing
/// tests with vectors (requires Oracle Database 23.4 or higher).
pub fn skip_unless_vectors_supported(conn: &oracledb::Connection) -> bool {
    if is_db_min_version(conn, 23, 4) {
        false
    } else {
        skip_test("database does not support vectors (requires 23.4+)")
    }
}

#[allow(dead_code)]
/// Returns the number of temporary LOBs held by this session.
pub fn temporary_lob_count(
    conn: &oracledb::Connection,
) -> Result<i64, oracledb::Error> {
    let sid: String = conn
        .query_row("select sys_context('USERENV', 'SID') from dual", &[])?
        .get(0)?;
    conn.query_row(
        r#"
        select cache_lobs + nocache_lobs + abstract_lobs
        from v$temporary_lobs
        where sid = :1
        "#,
        &[&sid],
    )?
    .get(0)
}
