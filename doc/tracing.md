# <a name="tracingsql"></a> 16. Tracing rust-oracledb

## <a name="applntracing"></a> 16.1 Application Tracing

There are multiple approaches for application tracing and monitoring:

- [End-to-end database tracing](#endtoendtracing) methods such as
  [Connection.set_action()](crate::Connection::set_action) and
  [Connection.set_module()](crate::Connection::set_module) are supported in
  rust-oracledb. Using these methods is recommended since they aid application
  monitoring and troubleshooting.
- The unique connection identifiers that appear in connection error
  messages, and in Oracle Database traces and logs, can be used to
  resolve connectivity errors. See
  [Using Connection Identifiers](#connectionid).

### <a name="endtoendtracing"></a> 16.1.1 Oracle Database End-to-End Tracing

Oracle Database end-to-end application tracing simplifies diagnosing
application code flow and performance problems in multi-tier or multi-user
environments.

The connection methods
[Connection::set_client_identifier()](crate::Connection::set_client_identifier),
[Connection::set_client_info()](crate::Connection::set_client_info),
[Connection::set_db_op()](crate::Connection::set_db_op),
[Connection::set_module()](crate::Connection::set_module), and
[Connection::set_action()](crate::Connection::set_action) set metadata for
end-to-end tracing. The values can be queried from data dictionary and
dynamic performance views to monitor applications, or you can use
tracing utilities. Values may appear in logs and audit trails.

The
[Connection::set_client_identifier()](crate::Connection::set_client_identifier)
method is typically set to the name (or identifier) of the actual end user
initiating a query. This allows the database to distinguish, and trace, end
users for applications that connect using a common database username. It can
also be used by [Oracle Virtual Private Database (VPD)] policies to
automatically limit data access. Oracle Database's [DBMS_MONITOR] package can
take advantage of the client identifier to enable statistics and tracing at an
individual level.

The [Connection::set_module()](crate::Connection::set_module), and
[Connection::set_action()](crate::Connection::set_action) methods can be set to
user-chosen, descriptive values identifying your code architecture.

After methods are set, the values are sent to the database when the next
[round-trip](#roundtrips) to the database occurs, for example when the next SQL
statement is executed.

The end-to-end tracing values will remain set in connections released back to a
connection pool. When the application re-acquires a connection from the pool, it
should set the desired values before using that connection.

The example below shows setting the action, module, and client
identifier attributes on a connection object, and then querying a view
to see the recorded values. The example both sets and queries the
values, but typically monitoring is done externally to the application.

```rust
connection.set_client_identifier("rustuser")?;
connection.set_module("End-to-end Demo")?;
connection.set_action("Query Session tracing parameters")?;

let cursor = connection.query(
    r#"
    select username, client_identifier, module, action
    from v$session
    where sid = sys_context('USERENV', 'SID')
    "#,
    &[],
)?;

for row_result in cursor {
    let row = row_result?;

    let username: String = row.get(0)?;
    let client_identifier: Option<String> = row.get(1)?;
    let module: Option<String> = row.get(2)?;
    let action: Option<String> = row.get(3)?;

    println!(
        "({}, {:?}, {:?}, {:?})",
        username, client_identifier, module, action
    );
}
```

The output will be:

```text
("HR", Some("rustuser"), Some("End-to-end Demo"), Some("Query Session tracing parameters"))
```

The values can also be manually set by calling [DBMS_APPLICATION_INFO]
procedures or [DBMS_SESSION.SET_IDENTIFIER]. These incur round-trips to the
database which reduces application scalability:

```sql
BEGIN
    DBMS_SESSION.SET_IDENTIFIER('rustuser');
    DBMS_APPLICATION_INFO.set_module('End-to-End Demo');
    DBMS_APPLICATION_INFO.set_action(action_name => 'Query Session tracing parameters');
END;
```

The [Connection::set_db_op()](crate::Connection::set_db_op) method can be used
for Real-Time SQL Monitoring, see [Monitoring Database Operations]. The value
is shown in the DBOP_NAME column of the [V$SQL_MONITOR] view.

```rust
connection.set_db_op("my op")?;

let cursor = connection.query(
    r#"
    select dbop_name
    from v$sql_monitor
    where sid = sys_context('USERENV', 'SID')
    "#,
    &[],
)?;

for row_result in cursor {
    let row = row_result?;
    let dbop_name: Option<String> = row.get(0)?;

    println!("{dbop_name:?}");
}
```

### <a name="connectionid"></a> 16.1.2 Using Connection Identifiers

A unique connection identifier (`CONNECTION_ID`) is generated for each
connection to the Oracle Database. The connection identifier is shown in
some Oracle Network error messages and logs, which helps in better
tracing and diagnosing of connection failures.

Depending on the Oracle Database version in use, the information that is
shown in logs varies.

You can define a prefix value which is added to the beginning of the
`CONNECTION_ID` value. This prefix aids in identifying the connections
from a specific application.

Rust example using `CONNECTION_ID_PREFIX` in the connect descriptor:

```rust
let config = oracledb::Config::default()
    .set_credentials("hr", "password")
    .set_connect_string(
        r#"
        (DESCRIPTION=
            (ADDRESS=(PROTOCOL=tcp)(HOST=localhost)(PORT=1521))
            (CONNECT_DATA=
                (SERVICE_NAME=orclpdb)
                (CONNECTION_ID_PREFIX=MYAPP)
            )
        )
        "#,
    )?;
```

If the connection fails, the error may include:

```text
(CONNECTION_ID=MYAPPm0PfUY6hYSmWPcgrHZCQIQ)
```

### <a name="tracingbind"></a> 16.1.3 Tracing Bind Values

Several methods for tracing bind variable values can be used. When tracing bind
variable values, be careful not to leak information and create a security
problem.

In Oracle Database, the view [V$SQL_BIND_CAPTURE] can capture bind information.
Tracing with Oracle Database's [DBMS_MONITOR] package may also be useful.

### <a name="dbviews"></a> 16.1.4 Database Views for Tracing rust-oracledb

This section shows some of the Oracle Database views useful for tracing and
monitoring rust-oracledb. Other views and columns not described here also
contain useful information, such as the the views discussed in
[End-to-end Tracing](#endtoendtracing) and [Tracing Bind Values](#tracingbind).

#### <a name="vsession"></a> 16.1.4.1 V$SESSION

The following table shows sample values for some [V$SESSION] columns. You may
see other values if you change the defaults by using connection configuration
methods before connecting, or if you set end-to-end tracing values such as
[Connection::set_module()](crate::Connection::set_module).

The table below shows the database column name in the first column and the
sample rust-oracledb value in the second column.

| Column | Sample rust-oracledb value |
| --- | --- |
| `MACHINE` | The host name of the machine running the Rust application, such as `myhost`, or the value specified by calling [Config::set_machine()](crate::Config::set_machine) |
| `MODULE` | The value set with [Connection::set_module()](crate::Connection::set_module), if set |
| `OSUSER` | The operating system user running the Rust application, such as `myusername`, or the value specified by calling [Config::set_osuser()](crate::Config::set_osuser) |
| `PROGRAM` | The executable name of the Rust application or the value specified by calling [Config::set_program()](crate::Config::set_program) |
| `TERMINAL` | `unknown`, unless set with [Config::set_terminal()](crate::Config::set_terminal) |

#### <a name="vsessionconninfo"></a> 16.1.4.2 V$SESSION_CONNECT_INFO

The following table shows sample values for some `V$SESSION_CONNECT_INFO`
columns. You may see other values if you set equivalent rust-oracledb
connection configuration values before connecting, such as
[Config::set_driver_name()](crate::Config::set_driver_name) or
[Config::set_osuser()](crate::Config::set_osuser).

The sample `V$SESSION_CONNECT_INFO` column values table is shown below where
the first column displays the database column name and the second column
displays the sample rust-oracledb value.

| Column | Sample rust-oracledb value |
| --- | --- |
| `CLIENT_DRIVER` | `rust-oracledb : 0.1.0`, or the value set with `Config::set_driver_name()` |
| `CLIENT_VERSION` | The rust-oracledb version, such as `0.1.0.0.0` |
| `OSUSER` | The operating system user running the Rust application, such as `myusername`, or the value set with `Config::set_osuser()` |

## <a name="lowleveltracing"></a> 16.2 Low Level Rust-oracledb Driver Tracing

Low level tracing is mostly useful to maintainers of rust-oracledb.

For rust-oracledb, packets can be traced by setting the RSO_DEBUG_PACKETS
environment variable in your terminal window before running the application.

For example, on Linux or macOS, you might use:

```shell
export RSO_DEBUG_PACKETS=1
```

On Windows you might set the variable like:

```shell
set RSO_DEBUG_PACKETS=1
```

Alternatively, the variable can be set in the application:

```rust
use std::env;

fn main() -> Result<(), oracledb::Error> {
    env::set_var("RSO_DEBUG_PACKETS", "1");
}
```

The output goes to stdout. The information logged is roughly similar to an
Oracle Net client trace, see [Oracle Net Services TRACE_LEVEL_CLIENT].

[DBMS_APPLICATION_INFO]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-14484F86-44F2-4B34-B34E-0C873D323EAD
[DBMS_MONITOR]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-951568BF-D798-4456-8478-15FEEBA0C78E
[DBMS_SESSION.SET_IDENTIFIER]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-988EA930-BDFE-4205-A806-E54F05333562
[Monitoring Database Operations]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-C941CE9D-97E1-42F8-91ED-4949B2B710BF
[Oracle Net Services TRACE_LEVEL_CLIENT]: https://www.oracle.com/pls/topic/lookup?ctx=dblatestid=GUID-1CC6424E-B3B5-4D55-A605-0C558496CBE0
[Oracle Virtual Private Database (VPD)]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-06022729-9210-4895-BF04-6177713C65A7
[V$SESSION]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-28E2DC75-E157-4C0A-94AB-117C205789B9
[V$SESSION_CONNECT_INFO]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-9F0DCAEA-A67E-4183-89E7-B1555DC591CE
[V$SQL_BIND_CAPTURE]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-D353F4BE-5943-4F5B-A99B-BC9505E9579C
[V$SQL_MONITOR]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-79E97A84-9C27-4A5E-AC0D-C12CB3E748E6
