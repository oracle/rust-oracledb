# rust-oracledb

[![crates.io](https://img.shields.io/crates/v/oracledb.svg)](https://crates.io/crates/oracledb)
[![Documentation](https://docs.rs/oracledb/badge.svg)](https://docs.rs/oracledb)
[![Rust](https://img.shields.io/badge/rust-1.89.0%2B-blue.svg?maxAge=3600)](https://github.com/rust-lang/oracledb)

A pure Rust driver for Oracle Database without any Oracle Client libraries
required, maintained by Oracle.

## Features

- No Oracle Client libraries required
- Support for Rust 1.89 and higher
- Connects to Oracle Database 12, 18, 19, 21 and 26 on-premises or in the Cloud
- SQL and PL/SQL Execution with significant optimizations including compressed
  fetch, pre-fetching, client and server result set caching, and statement
  caching with auto-tuning
- Full use of Oracle Network Service infrastructure, including encrypted
  network traffic and security features
- Extensive Oracle data type support, including VECTOR, JSON and large object
  support (CLOB and BLOB)
- Array operations for efficient INSERT, UPDATE and MERGE execution
- Connection pooling
- Database Resident Connection Pooling (DRCP)
- Privileged Connections
- End-to-end monitoring and tracing
- Support for Oracle AI Database 26ai Deep Data Security
- Support for fetching and inserting Arrow arrays

## Getting Started

Add to your `Cargo.toml`:

```toml
[dependencies]
oracledb = "26.0.0-beta.2"
```

If you wish to make use of the optional Arrow support, use this instead:

```toml
[dependencies]
oracledb = { version = "26.0.0-beta.2", features = ["arrow"] }
```

## Documentation

The documentation can be found [here](https://docs.rs/oracledb).

## Examples

Execute queries and return rows.

```rust,no_run
use oracledb;

fn main() -> Result<(), oracledb::Error> {
    // create configuration and connect to the database
    let config = oracledb::Config::default()
        .set_credentials("user", "password")
        .set_connect_string("server:1521/service_name")?;
    let conn = oracledb::connect(config)?;

    // perform simple query that returns a single row with no bind parameters
    let row = conn.query_row("select user from dual", &[])?;
    let user: String = row.get(0)?;

    // perform query that returns multiple rows with a bind parameter
    let cursor = conn.query(
        "select ename, sal, comm from emp where deptno = :1", &[&30]
    )?;
    for row_result in cursor {
        let row = row_result?;
        let ename: String = row.get(0)?;
        let sal: i32 = row.get(1)?;
        let comm: Option<i32> = row.get(2)?;
    }
}
```

Execute a query and return an Arrow RecordBatch.

```rust,no_run
use arrow_array::{Array, StringArray};
use oracledb;

fn main() -> Result<(), oracledb::Error> {
    // create configuration and connect to the database
    let config = oracledb::Config::default()
        .set_credentials("user", "password")
        .set_connect_string("server:1521/service_name")?;
    let conn = oracledb::connect(config)?;

    // perform simple query that returns an Arrow RecordBatch
    let rb = conn.query_arrow(
        "select user from dual",
        oracledb::BindParameters::default(),
    )?;

    // access a single Arrow column
    let users = rb
        .column(0)
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("Failed to downcast to StringArray");
    for user in users.iter() {
        println!("User = {}", user.unwrap_or("NULL"));
    }
    Ok(())
}
```

## Help

Questions can be asked in [GitHub Discussions][ghdiscussions].

Problem reports can be raised in [GitHub Issues][ghissues].

## Contributing

This project welcomes contributions from the community. Before submitting a
pull request, please [review our contribution guide](./CONTRIBUTING.md).

## Security

Please consult the [security guide](./SECURITY.md) for our responsible security
vulnerability disclosure process.

## License

See [LICENSE](./LICENSE.txt).

## History

This replaces the original Rust thin driver produced by Muhammed Durakovic. The
new name for that driver is [oraclemcp-driver-cx][oraclemcp-driver-cx].

[ghdiscussions]: https://github.com/oracle/rust-oracledb/discussions
[ghissues]: https://github.com/oracle/rust-oracledb/issues
[oraclemcp-driver-cx]: https://crates.io/crates/oraclemcp-driver-cx
