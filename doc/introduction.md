# <a name="introduction"></a> 1. Rust-oracledb Driver for Oracle Database

The rust-oracledb driver is an open source Rust module that enables quick and
easy access to Oracle Database for Rust applications without the use of Oracle
Client libraries. It is lightweight and high-performance. The module is
maintained by Oracle.

This is a pre-release of rust-oracledb, intended to provide early access to
the driver and gather user feedback. The APIs and functionalities are subject
to change as development continues.

Rust-oracledb has a rich feature set which is easy to use. It gives you control
over SQL and PL/SQL statement execution, and also has security features.

The module is available from the standard package repository [crates.io]. The
source code is hosted at [github.com/oracle/rust-oracledb].

This module is currently tested with Rust 1.89 against Oracle Database versions
26ai, 21c, and 19c.

Changes in rust-oracledb releases can be found in the
[release notes](#releasenotes).

## <a name="architecture"></a> 1.1 Architecture

The rust-oracledb driver allows Rust applications to connect directly to
Oracle Database. This driver does not need Oracle Client libraries.

![Diagram illustrating the architecture of the rust-oracledb driver. On the
left is the Users icon which communicates bidirectionally to a block labelled
Rust process. The block contains two smaller blocks labeled Rust and
rust-oracledb module. The Rust process block communicates bidirectionally to
the Oracle Database icon on the right side. Bidirectional arrows indicate
request and response flow between users, the Rust process, and the database
.](../../../../doc/images/rust-oracledb-arch.png)

*Figure 1: Architecture of the rust-oracledb driver*

The figure shows the architecture of rust-oracledb. Users interact with a Rust
application, for example by making web requests. The application program makes
calls to rust-oracledb functions. The connection from rust-oracledb to Oracle
Database is established directly over the Oracle Net protocol. The database can
be on the same machine as Rust, or it can be remote.

The behavior of Oracle Net can optionally be configured with application
settings, or by using a `tnsnames.ora` file, see
[Optional Oracle Net Configuration file](#optnetfile).

## <a name="installing"></a> 1.2 Installing rust-oracledb

Rust-oracledb is typically installed from the package repository [crates.io].

### <a name="instreq"></a> 1.2.1 Installation Requirements

To use rust-oracledb, you need:

- [Rust environment] (Rust 1.89 or later)
- [Cargo]
- An Oracle Database version 19 or later which can be either local or remote,
  on-premises or in the Cloud

### <a name="installation"></a> 1.2.2 Installation

To install rust-oracledb for your application:

```shell
cargo install oracledb
```

This automatically adds an entry like this to your `Cargo.toml` file:

```text
[dependencies]
oracledb = "0.1.0"
```

Runnable examples are in the [GitHub samples directory].

## <a name="featurehighlights"></a> 1.3 Feature Highlights of rust-oracledb

The rust-oracledb feature highlights are:

- Easy installation from cargo
- Support for multiple Oracle Database versions
- Execution of SQL and PL/SQL statements
- Extensive Oracle data type support, including JSON, VECTOR, and large
  objects (CLOB and BLOB)
- Connection management, including connection pooling
- Full use of Oracle Network Service infrastructure, including
  encrypted network traffic

See [Oracle Database Features Supported by rust-oracledb](#appendixa) for more
information.

[Cargo]: https://doc.rust-lang.org/cargo/
[crates.io]: https://crates.io
[github.com/oracle/rust-oracledb]: https://github.com/oracle/rust-oracledb
[GitHub samples directory]: https://github.com/oracle/rust-oracledb/tree/main/samples
[oracledb]: https://crates.io/crates/oracledb/
[Rust environment]: https://rust-lang.org/tools/install/
