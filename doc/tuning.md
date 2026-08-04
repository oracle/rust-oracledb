# <a name="tuning"></a> 8. Tuning rust-oracledb

Some general tuning tips are:

- Tune your application architecture.

  A general application goal is to reduce the number of
  [round-trips](#roundtrips) between rust-oracledb and the database.

  For multi-user applications, make use of connection pooling. Create the pool
  once during application initialization. Do not oversize the pool, see
  [connection pooling](#connpooling).

  Make use of efficient rust-oracledb functions. For example, to insert
  multiple rows use
  [Connection::execute_batch()](crate::Connection::execute_batch) instead of
  [Connection::execute()](crate::Connection::execute).

- Tune your SQL statements. See the [SQL Tuning Guide].

  Use [bind variables](#bind) to avoid statement reparsing.

- Tune statement options created by
  [Connection::statement()](crate::Connection::statement), including
  `fetch_array_size()` and `prefetch_rows()`, for each SELECT query, see
  [Tuning Fetch Performance](#tuningfetch).

  Do simple optimizations like [limiting the number of rows](#rowlimit) and
  avoiding selecting columns not used in the application.

  It may be faster to work with simple scalar relational values than to use
  Oracle Database object types.

  Make good use of PL/SQL to avoid executing many individual statements from
  rust-oracledb.

  Tune the [Statement Cache](#stmtcache).

- Tune your database. See the [Database Performance Tuning Guide].

- Tune your network. For example, when inserting or retrieving a large number
  of rows (or for large data), or when using a slow network, then tune the
  Oracle Network Session Data Unit (SDU) and socket buffer sizes, see
  [Configuring Session Data Unit] and [Oracle Net Services: Best Practices for
  Database Performance and High Availability][Oracle Net Services].

  The SDU size may optionally be set in the connection
  [Easy Connect string](#easyconnect) or [connect descriptor](#conndescriptor).
  The SDU size that will actually be used is negotiated down to the lower of
  application-side value and the database network SDU configuration value.

- Do not commit or rollback unnecessarily.

## <a name="roundtrips"></a> 8.1 Database Round-trips

A round-trip is defined as the travel of a message from rust-oracledb to the
database and back. Calling each rust-oracledb function, or accessing each
attribute, will require zero or more round-trips. For example, inserting a
simple row involves sending data to the database and getting a success response
back. This is a round-trip. Along with tuning an application's architecture
and [tuning its SQL statements][SQL Tuning Guide], a general performance and
scalability goal is to minimize [round-trips] because they impact application
performance and overall system scalability.

Some general tips for reducing round-trips are:

- Tune statement options created by
  [Connection::statement()](crate::Connection::statement) including
  `fetch_array_size` and `prefetch_rows` for each SELECT query.
- Use [Connection.execute_batch()](crate::Connection::execute_batch) for
  optimal DML execution.
- Only commit when necessary.
- Make use of PL/SQL procedures which execute multiple SQL statements instead
  of executing them individually from rust-oracledb.
- Use scalar types instead of Oracle Database object types.
- Avoid overuse of [Connection::ping()](crate::Connection::ping).
- Avoid setting
  [PoolConfig::set_ping_interval()](crate::PoolConfig::set_ping_interval) to
  *Some(Duration::ZERO)* or a small value.

### <a name="numroundtrips"></a> 8.1.1 Finding the Number of Round-Trips

Oracle's [Automatic Workload Repository] (AWR) reports show 'SQL*Net
round-trips to/from client' and are useful for finding the overall behavior of
a system.

Sometimes you may wish to find the number of round-trips used for a specific
application. Snapshots of the V$SESSTAT view taken before and after doing some
work can be used for this:

```rust
use oracledb;

fn get_round_trips(system_conn: &Connection, sid: usize) -> Result<usize, Error> {
    let sql = r#"
        select ss.value
        from v$sesstat ss, v$statname sn
        where ss.sid = :sid
          and ss.statistic# = sn.statistic#
          and sn.name like '%roundtrip%client%'
    "#;

    let row = system_conn.query_row(sql, &[&sid])?;
    row.get(0)
}

fn main() -> Result<(), Error> {
    let system_config = Config::default()
        .set_credentials("system", "system_password")
        .set_connect_string("localhost/orclpdb")?;

    let user_config = Config::default()
        .set_credentials("hr", "hr")
        .set_connect_string("localhost/orclpdb")?;

    let system_conn = oracledb::connect(system_config)?;
    let connection = oracledb::connect(user_config)?;

    let sid = connection.session_id()?;
    let round_trips_before = get_round_trips(&system_conn, sid)?;

    let cursor = connection.query("select level from dual connect by level <= 10", &[])?;
    let rows: Result<Vec<_>, Error> = cursor.collect();
    let rows = rows?;

    let round_trips_after = get_round_trips(&system_conn, sid)?;

    println!("Fetched {} rows.", rows.len());
    println!(
        "Round-trips required for query: {}",
        round_trips_after - round_trips_before
    );

    Ok(())
}
```

## <a name="stmtcache"></a> 8.2 Statement Caching

Rust-oracledb uses statement caching to make repeated execution of statements
efficient. Calls such as [Connection::execute()](crate::Connection::execute),
[Connection::execute_batch()](crate::Connection::execute_batch),
[Connection::query()](crate::Connection::query), and statements created with
[Connection::statement()](crate::Connection::statement) can reuse cached
statements, avoiding unnecessary statement reparsing and reducing metadata
transfer between rust-oracledb and the database. Performance and scalability
are improved.

Each standalone connection and pooled connection has its own statement cache
with a default size of *20*. The default size of the statement cache can be
changed using [Config::set_stmtcachesize()](crate::Config::set_stmtcachesize).
The size can be set before creating standalone connections or pools.

In general, set the statement cache size to the working set of statements
executed by the application. To manually tune the cache, monitor the general
application load and [Automatic Workload Repository] (AWR) statistics such as
"bytes sent via SQL*Net to client". This statistic can benefit when cached
statements avoid repeated statement metadata transfer between rust-oracledb and
the database. Adjust the statement cache size to suit the application workload.

### <a name="setstmtcache"></a> 8.2.1 Setting the Statement Cache

The statement cache size can be set by using
[Config::set_stmtcachesize()](crate::Config::set_stmtcachesize):

```rust
let config = Config::default()
    .set_credentials("hr", "hr")
    .set_stmtcachesize(40)
    .set_connect_string("dbhost.example.com/orclpdb")?;
```

### <a name="tunestmtcache"></a> 8.2.2 Tuning the Statement Cache

In general, set the statement cache to the size of the working set of
statements being executed by the application.

For manual tuning use views like V$SYSSTAT:

```sql
SELECT value FROM V$SYSSTAT WHERE name = 'parse count (total)';
```

Find the value before and after running application load to give the number of
statement parses during the load test. Alter the statement cache size and
repeat the test until you find a minimal number of parses.

If you have Automatic Workload Repository (AWR) reports you can monitor
general application load and the "bytes sent via SQL*Net to client" values.
The latter statistic should benefit from not shipping statement metadata to
rust-oracledb. Adjust the statement cache size and re-run the test to find
the best cache size.

### <a name="disablestmtcache"></a> 8.2.3 Disabling the Statement Cache

Statement caching can be disabled by setting the cache size to 0 in
[Config::set_stmtcachesize()](crate::Config::set_stmtcachesize):

```rust
let config = oracledb::Config::default()
    .set_credentials("hr", "hr")
    .set_stmtcachesize(0)
    .set_connect_string("dbhost.example.com/orclpdb")?;
```

Disabling the cache may be beneficial when the quantity or order of statements
causes cache entries to be flushed before they get a chance to be
reused. For example if there are more distinct statements than cache
slots, and the order of statement execution causes older statements to
be flushed from the cache before the statements are re-executed.

Disabling the statement cache may also be helpful in test and development
environments. The statement cache can become invalid if connections remain
open and database schema objects are recreated. Applications can then receive
errors such as ``ORA-3106``. However, after a statement execution error is
returned once to the application, rust-oracledb automatically drops that
statement from the cache. This lets subsequent re-executions of the statement
on that connection to succeed.

When an application wants to reuse statement text together with statement
options, create a statement with
[Connection::statement()](crate::Connection::statement) and then call methods
such as `execute()`, `execute_batch()`, or `query()` on the returned statement.
Statements are eligible for statement caching by default.

To prevent a statement from being cached, call
[Statement::exclude_from_cache()](crate::Statement::exclude_from_cache)
on the statement before executing it. If the same SQL text is already present
in the cache, using `exclude_from_cache()` removes it from the cache for that
execution; otherwise, the statement is simply not cached. This feature can
prevent a rarely executed statement from replacing a more frequently executed
statement in a full cache. For example, if a statement will only be executed
once, create it with [Connection::statement()](crate::Connection::statement)
and call `exclude_from_cache()` before execution:

```rust
let row = connection
    .statement("select user from dual")?
    .exclude_from_cache()
    .query_row(&[])?;
```

Statements created with [Connection::statement()](crate::Connection::statement)
are cached by default, unless `exclude_from_cache()` is called before
execution.

[Automatic Workload Repository]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-56AEF38E-9400-427B-A818-EDEC145F7ACD
[Configuring Session Data Unit]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-86D61D6F-AD26-421A-BABA-77949C8A2B04
[Database Performance Tuning Guide]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=TGDBA
[Oracle Net Services]: https://static.rainfocus.com/oracle/oow19/sess/1553616880266001WLIh/PF/OOW19_Net_CON4641_1569022126580001esUl.pdf
[SQL Tuning Guide]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=TGSQL
[round-trips]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-9B2F05F9-D841-
4493-A42D-A7D89694A2D1
