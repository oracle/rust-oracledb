# rust-oracledb

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
oracledb = "26.0.0-beta.1"
```

If you wish to make use of the optional Arrow support, use this instead:

```toml
[dependencies]
oracledb = { version = "26.0.0-beta.1.1", features = ["arrow"] }
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

This replaces the original Rust thin driver produced by Muhammed Duhr. The new
name for that driver is [oraclemcp-driver-cx][oraclemcp-driver-cx].

[ghdiscussions]: https://github.com/oracle/rust-oracledb/discussions
[ghissues]: https://github.com/oracle-samples/rust-oracledb/issues
[oraclemcp-driver-cx]: https://crates.io/crates/oraclemcp-driver-cx
