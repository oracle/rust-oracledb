# <a name="connhandling"></a> 2. Connecting to Oracle Database

Connections between rust-oracledb and Oracle Database are used for
executing [SQL](#sqlexecution) and [PL/SQL](#plsqlexecution).

There are two ways to create a connection to Oracle Database using
rust-oracledb:

- **Standalone connections**: [Standalone connections](#standaloneconnection)
  are useful when the application needs a single connection to a database.
  Connections are created by calling [oracledb::connect()](crate::connect).
- **Pooled connections**: [Connection pooling](#connpooling) is important for
  performance when applications frequently connect and disconnect from the
  database. Pools support Oracle's [high availability](#highavailability)
  features and are recommended for applications that must be reliable. Small
  pools can also be useful for applications that want a few connections
  available for infrequent use. Pools are created with
  [oracledb::create_pool()](crate::create_pool()), and then
  [Pool::acquire()](crate::Pool::acquire) can be called to obtain a connection
  from a pool.

Many connection behaviors can be controlled by rust-oracledb
connection options. Other settings can be configured in an
[Optional Oracle Net Configuration file](#optnetfile). These include limiting
the amount of time that opening a connection can take, or enabling
[network encryption](#netencrypt).

## <a name="basicconfig"></a> 2.1 Create a Basic Configuration

The basic configuration can be stored with a [Config](crate::Config) object,
which acts as a central place for connection details. You can set:

- A database username
- The database password for that user
- A connect string, see [Connection Strings](#connstr)

For information on authentication methods supported in rust-oracledb, see
[Authentication Options](#authentication).

An example of defining a configuration object for your connection is shown
below:

```rust
use oracledb;

fn main() -> Result<(), oracledb::Error> {

    // create configuration
    let config = oracledb::Config::default()
        .set_credentials("hr", "password")
        .set_connect_string("localhost:1521/orclpdb")?;
}
```

## <a name="connstr"></a> 2.2 Oracle Net Services Connection Strings

The [Config::set_connect_string()](crate::Config::set_connect_string) function
is used to set the Oracle Database Oracle Net Services Connection string
(commonly abbreviated as "connection string") that identifies which database
service to connect to. The value of the ``connect_string`` used in this method
can be one of Oracle Database's naming methods:

- An Oracle [Easy Connect string](#easyconnect)
- A [Connect Descriptor](#conndescriptor)
- A [TNS Alias](#netservice) mapping to an Easy Connect string or Connect
  Descriptor stored in a [tnsnames.ora](#optnetfile) file

For more information about naming methods, see the
[Database Net Services Administrator's Guide][configure-naming-methods].

**Note**: Creating a connection in rust-oracledb always requires a connection
string to be specified. Rust-oracledb cannot use "bequeath" connections and
does not reference Oracle environment variables `ORACLE_SID`, `TWO_TASK`, or
`LOCAL`.

### <a name="easyconnect"></a> 2.2.1 Easy Connect Syntax for Connection Strings

An [Easy Connect][easy-connect] string is often the simplest connection string
to use in the
[Config::set_connection_string()](crate::Config::set_connect_string) function.

Using Easy Connect strings means that an external [tnsnames.ora](#optnetfile)
configuration file is not needed.

The Easy Connect syntax in rust-oracledb is:

```text
[[protocol:]//]host[:port]/service_name[:server][/instance_name]
```

See the
[Database Net Services Administrator's Guide][support-easy-connect-plus].

For example, to connect to the Oracle Database service `orclpdb` that is
running on the host `dbhost.example.com` with the default Oracle
Database port 1521, use:

```rust
let config = oracledb::Config::default()
    .set_credentials("hr", "password")
    .set_connect_string("dbhost.example.com/orclpdb")?;
```

If the database is using a non-default port, it must be specified:

```rust
let config = oracledb::Config::default()
    .set_credentials("hr", "password")
    .set_connect_string("dbhost.example.com:1984/orclpdb")?;
```

The Easy Connect syntax supports Oracle Database service names. It cannot be
used with the older System Identifiers (SID).

### <a name="conndescriptor"></a> 2.2.2 Connect Descriptors

Connect Descriptors can be embedded directly in rust-oracledb applications.

An example of direct use is:

```rust
let config = oracledb::Config::default()
    .set_credentials("hr", "password")
    .set_connect_string("(DESCRIPTION=
        (FAILOVER=on)
        (ADDRESS_LIST=(ADDRESS=(PROTOCOL=tcp)(HOST=sales1-svr)(PORT=1521)))
        (CONNECT_DATA=(SERVICE_NAME=sales.example.com)))
")?;
```

Syntax is shown in the [Database Net Services Reference][conn-descriptor-desc].

Only connect descriptor parameters recognized by rust-oracledb are parsed and
used. Unrecognized `DESCRIPTION`, `CONNECT_DATA`, `ADDRESS`, `ADDRESS_LIST`,
and `SECURITY` parameters are ignored and are not passed to the database.

### <a name="netservice"></a> 2.2.3 TNS Aliases for Connection Strings

[Connect Descriptors](#conndescriptor) are commonly stored in a
[tnsnames.ora](#optnetfile) file and associated with a TNS Alias. This alias
can be used directly with
[Config::set_connection_string()](crate::Config::set_connect_string). For
example, given a file `/opt/oracle/config/tnsnames.ora` with the following
content:

```text
ORCLPDB =
    (DESCRIPTION =
      (ADDRESS = (PROTOCOL = TCP)(HOST = dbhost.example.com)(PORT = 1521))
      (CONNECT_DATA =
        (SERVER = DEDICATED)
        (SERVICE_NAME = orclpdb)
      )
    )
```

Rust-oracledb locates `tnsnames.ora` from the default configuration directory.
Set `TNS_ADMIN` to the directory containing `tnsnames.ora`, or set
`ORACLE_HOME` so the file can be found in `$ORACLE_HOME/network/admin`.

For example, with `TNS_ADMIN=/opt/oracle/config`:

```rust
let config = oracledb::Config::default()
    .set_credentials("hr", "password")
    .set_connect_string("ORCLPDB")?;
```

More options for how rust-oracledb locates [tnsnames.ora](#optnetfile) files
are detailed in [Optional Oracle Net Configuration File](#optnetfile).

For more information about Net Service Names, see
[Database Net Services Reference][overview-local-naming-parameters].

### <a name="optnetfile"></a> 2.2.4 Optional Oracle Net Configuration File

An optional Oracle Net configuration file may be read when creating standalone
connections or pooled connections. This file affects connection behavior.
Rust-oracledb can use a file called `tnsnames.ora` file that contains
database aliases and their related connection configuration information used
for establishing connections. See
[TNS Aliases for Connection Strings](#netservice).

If you use a `tnsnames.ora` file to configure your connections, then put the
file in a directory accessible to rust-oracledb.

Rust-oracledb can read a `tnsnames.ora` file when a [TNS Alias](#netservice)
is used in [Config::set_connect_string()](crate::Config::set_connect_string).
The alias is resolved when `set_connect_string()` is called. The resulting
configuration can then be used for standalone connections with
[oracledb::connect()](crate::connect), or for pooled connections with
[oracledb::create_pool()](crate::create_pool).

Rust-oracledb looks for a file named `tnsnames.ora` in the configured Oracle
Net configuration directory. Only one `tnsnames.ora` file is read. Entries from
`IFILE` directives in that file are also parsed. If the TNS alias cannot be
found, then `set_connect_string()` returns an error and the connection or pool
cannot be created.

In rust-oracledb, you should explicitly specify the directory because some
traditional "default" locations such as `$ORACLE_BASE/homes/XYZ/network/admin/`
(in a read-only Oracle Database home) or the Windows registry are not
automatically used. The directory can be set by using
[Config::set_config_dir()](crate::Config::set_config_dir).

Rust-oracledb does not read other Oracle Net configuration files such as
`sqlnet.ora` and `oraaccess.xml`.

### <a name="jdbcconnstring"></a> 2.2.5 JDBC and Oracle SQL Developer Connection Strings

The rust-oracledb connection string syntax is different from Java JDBC and the
common Oracle SQL Developer syntax. If these JDBC connection strings reference
a service name like:

```text
jdbc:oracle:thin:@hostname:port/service_name
```

For example:

```text
jdbc:oracle:thin:@dbhost.example.com:1521/orclpdb
```

then use Oracle's Easy Connect syntax in rust-oracledb:

```rust
let config = oracledb::Config::default()
    .set_credentials("hr", "password")
    .set_connect_string("dbhost.example.com/orclpdb")?;

let connection = oracledb::connect(config)?;
```

You may need to remove JDBC-specific parameters from the connection
string and use rust-oracledb alternatives.

If a JDBC connection string uses an old-style Oracle Database SID
"system identifier", and the database does not have a service name:

```text
jdbc:oracle:thin:@hostname:port:sid
```

For example:

```text
jdbc:oracle:thin:@dbhost.example.com:1521:orcl
```

then either [embed the Connect Descriptor](#conndescriptor):

```rust
let config = oracledb::Config::default()
    .set_credentials("hr", "password")
    .set_connect_string("(DESCRIPTION=
        (ADDRESS=(PROTOCOL=tcp)(HOST=dbhost.example.com)(PORT=1521))
        (CONNECT_DATA=(SID=orcl)))")?;

let connection = oracledb::connect(config)?;
```

Or create a [Net Service Name](#netservice), for example:

```text
finance =
    (DESCRIPTION =
      (ADDRESS = (PROTOCOL = TCP)(HOST = dbhost.example.com)(PORT = 1521))
      (CONNECT_DATA =
        (SID = ORCL)
      )
    )
```

This can be referenced in rust-oracledb:

```rust
let config = oracledb::Config::default()
    .set_credentials("hr", "password")
    .set_connect_string("finance")?;

let connection = oracledb::connect(config)?;
```

## <a name="standaloneconnection"></a> 2.3 Standalone Connections

Standalone connections are database connections that do not use a rust-oracledb
connection pool. They are useful for simple applications that use a single
connection to a database. Simple connections are created by calling
[oracledb::connect()](crate::connect) and passing the
[configuration object](#basicconfig).

### <a name="createstandaloneconnection"></a> 2.3.1 Creating a Standalone Connection

Standalone connections are created by calling
[oracledb::connect()](crate::connect).

A simple standalone connection example:

```rust
use oracledb;

fn main() -> Result<(), oracledb::Error> {

    // create basic configuration
    let config = oracledb::Config::default()
        .set_credentials("hr", "password")
        .set_connect_string("localhost:1521/orclpdb")?;

    // establish standalone connection to the database
    let connection = oracledb::connect(config)?;
}
```

### <a name="closeconnection"></a> 2.3.2 Closing Connections

Connections are closed automatically when they are dropped. In most cases, you
do not need to explicitly call [Connection::close()](crate::Connection::close).
Call `close()` only when you want to release the database connection before the
connection object goes out of scope.

## <a name="authentication"></a> 2.4 Authenticating to Oracle Database

When connecting to Oracle Database, authentication plays a key role in
establishing an authorized connection. Rust-oracledb supports various
Oracle Database authentication methods.

### <a name="dbauthentication"></a> 2.4.1 Database Authentication

Database Authentication is the most basic authentication method that allows
users to connect to Oracle Database by using a valid database username and
their associated password. Oracle Database verifies the username and
password specified in the rust-oracledb connection method with the
information stored in the database. See [Database Authentication of Users] for
more information.

[Standalone connections](#standaloneconnection) and
[pooled connections](#connpooling) can be created in rust-oracledb using
database authentication. This can be done by specifying the database username
and the associated password in the
[Config::set_credentials()](crate::Config::set_credentials) method. An example
is:

```rust
// create configuration
let config = oracledb::Config::default()
    .set_credentials("hr", "password");
```

### <a name="proxyauth"></a> 2.4.2 Proxy Authentication

Proxy authentication allows a user (the "session user") to connect to Oracle
Database using the credentials of a "proxy user". Statements will run as the
session user. Proxy authentication is generally used in three-tier applications
where one user owns the schema while multiple end-users access the data. For
more information about proxy authentication, see the
[Oracle documentation][client access through a proxy].

An alternative to using proxy users is to set
[Connection::set_client_identifier](crate::Connection::set_client_identifier)
after connecting and use its value in statements and in the database, for
example for [monitoring](#endtoendtracing).

The following proxy examples use these schemas. The `mysessionuser`
schema is granted access to use the password of `myproxyuser`:

``` sql
CREATE USER myproxyuser IDENTIFIED BY myproxyuserpw;
GRANT CREATE SESSION TO myproxyuser;

CREATE USER mysessionuser IDENTIFIED BY itdoesntmatter;
GRANT CREATE SESSION TO mysessionuser;

ALTER USER mysessionuser GRANT CONNECT THROUGH myproxyuser;
```

After connecting to the database, the following query can be used to
show the session and proxy users:

``` sql
SELECT NVL(SYS_CONTEXT('USERENV', 'PROXY_USER'), 'None'),
       SYS_CONTEXT('USERENV', 'SESSION_USER')
FROM DUAL;
```

Standalone connection examples:

```rust
// Basic authentication without a proxy
let config = Config::default()
    .set_credentials("myproxyuser", "myproxyuserpw")
    .set_connect_string("localhost:1521/orclpdb")?;

let connection = oracledb::connect(config)?;
let row = connection.query_row(user_query, &[])?;
let proxy_user: String = row.get(0)?;
let session_user: String = row.get(1)?;
println!("PROXY_USER:   {}", proxy_user);   // PROXY_USER:   None
println!("SESSION_USER: {}", session_user); // SESSION_USER: MYPROXYUSER

// Basic authentication with a proxy
let config = Config::default()
    .set_credentials("myproxyuser[mysessionuser]", "myproxyuserpw")
    .set_connect_string("localhost:1521/orclpdb")?;

let connection = oracledb::connect(config)?;
let row = connection.query_row(user_query, &[])?;
let proxy_user: String = row.get(0)?;
let session_user: String = row.get(1)?;
println!("PROXY_USER:   {}", proxy_user);   // PROXY_USER:   MYPROXYUSER
println!("SESSION_USER: {}", session_user); // SESSION_USER: MYSESSIONUSER
```

Pooled connection examples:

```rust
// Basic authentication without a proxy
let pool_config = PoolConfig::default()
    .set_credentials("myproxyuser", "myproxyuserpw")
    .set_connect_string("localhost:1521/orclpdb")?;

let pool = oracledb::create_pool(pool_config)?;

let connection = pool.acquire()?;
let row = connection.query_row(user_query, &[])?;
let proxy_user: String = row.get(0)?;
let session_user: String = row.get(1)?;
println!("PROXY_USER:   {}", proxy_user);   // PROXY_USER:   None
println!("SESSION_USER: {}", session_user); // SESSION_USER: MYPROXYUSER

// Basic authentication with a proxy
let pool_config = PoolConfig::default()
    .set_credentials("myproxyuser[mysessionuser]", "myproxyuserpw")
    .set_connect_string("localhost:1521/orclpdb")?;

let pool = oracledb::create_pool(pool_config)?;

let connection = pool.acquire()?;
let row = connection.query_row(user_query, &[])?;
let proxy_user: String = row.get(0)?;
let session_user: String = row.get(1)?;
println!("PROXY_USER:   {}", proxy_user);   // PROXY_USER:   MYPROXYUSER
println!("SESSION_USER: {}", session_user); // SESSION_USER: MYSESSIONUSER
```

## <a name="connpooling"></a> 2.5 Connection Pooling

Connection pooling can significantly improve application performance and
scalability by allowing resource sharing. Pools also let applications use
optional advanced Oracle High Availability features.

Opening a connection to a database can be expensive: the connection string must
be parsed, a network connection must be established, the Oracle Database
network listener needs to be invoked, user authentication must be performed, a
database server process must be created, and session memory must be allocated
(and then the process is destroyed when the connection is closed). Connection
pools remove the overhead of repeatedly opening and closing
[standalone connections](#standaloneconnection) by establishing a pool of open
connections that can be reused throughout the life of an application process.

Various Oracle Database authentication methods are supported in
rust-oracledb, see [Authentication Options](#authenticationmethods).

Rust-oracledb's driver connection pooling lets applications create and maintain
a pool of open connections to the database. Connection pooling is important for
performance and scalability when applications need to handle a large number of
users who do database work for short periods of time but have relatively long
periods when the connections are not needed. The high availability features of
pools also make small pools useful for applications that want a few connections
available for infrequent use and requires them to be immediately usable when
acquired.

**Note:** Rust-oracledb driver connection pools must be created, used, and
closed within the same process. Sharing pools or connections across
processes has unpredictable behavior.

Using connection pools in multi-threaded architectures is supported.

#### <a name="basicpoolconfig"></a> 2.5.1 Create a Basic Pool Configuration

Before creating a connection pool, define the basic configuration details and
pool details in a [PoolConfig](crate::PoolConfig) object. The basic
configuration details include database credentials and the
[connect string](#connstr). The pool details include connection limits and
settings that control how the pool grows under load. Choosing appropriate pool
settings helps applications use database resources efficiently and maintain
stable performance.

For example, to create a pool configuration that initially contains one
connection but can grow up to five connections:

```rust
use oracledb;

fn main() -> Result<(), oracledb::Error> {

    // Create a pool configuration
    let pool_config = oracledb::PoolConfig::default()
        .set_credentials("hr", "password")
        .set_connect_string("localhost/orclpdb")
        .set_min_connections(1)         // minimum connections in the pool
        .set_max_connections(5)         // maximum pool size
        .set_connection_increment(2)?;  // how many connections to add when growing
}
```

#### <a name="createconnpool"></a> 2.5.2 Creating a Connection Pool

A driver connection pool is created by calling
[oracledb::create_pool()](crate::create_pool()). For example:


```rust
let mut pool = oracledb::create_pool(pool_config)?;
```

#### <a name="getconnpool"></a> 2.5.3 Getting Connections from a Pool

After a pool has been created, your application can get a connection
from it by calling [Pool::acquire()](crate::Pool::acquire()):

```rust
let connection = pool.acquire()?;
```

These connections can be used in the same way that
[standalone connections](#standaloneconnection) are used.

By default, [Pool::acquire()](crate::Pool::acquire()) calls wait for a
connection to be available before returning to the application. A connection
will be available if the pool currently has idle connections, when another
user returns a connection to the pool, or after the pool grows. Waiting allows
applications to be resilient to temporary spikes in connection load. Users may
have to wait a brief time to get a connection but will not experience
connection failures.

#### <a name="returnconnpool"></a> 2.5.4 Returning Connections to a Pool

Pooled connections are returned to the pool automatically when they are
dropped. In most cases you do not need to explicitly call
[Connection::close()](crate::Connection::close) but you may do so if you wish
the connection returned to the pool before it is dropped, thus making it
available for other users sooner.

#### <a name="closeconnpool"></a> 2.5.5 Closing a Connection Pool

Connection pools are closed when they are dropped. In most cases, you do not
need to explicitly call [Pool::close()](crate::Pool::close). Call
`close()` only when you want to release the database connections in the
connection pool before it goes out of scope.

### <a name="connpoolsize"></a> 2.5.6 Connection Pool Sizing

The Oracle Real-World Performance Group's recommendation is to use
fixed size connection pools. The values specified in
[PoolConfig::set_min_connections()](crate::PoolConfig::set_min_connections) and
[PoolConfig::set_max_connections()](crate::PoolConfig::set_max_connections)
should be the same.

Fixed size pools avoid connection storms on the database which can
decrease throughput. See
[Guideline for Preventing Connection Storms][use-static-pools],
which contains more details about sizing of pools. Having a fixed size
will also guarantee that the database can handle the upper pool size.
For example, if a dynamically sized pool needs to grow but the database
resources are limited, then [Pool::acquire()](crate::Pool::acquire) may return
errors such as [ORA-28547][ORA-28547]. With a fixed pool size, these errors
are more likely to occur when the pool is created, allowing you to change the
pool size or reconfigure the database before the application begins operating.
With a dynamically growing pool, new connections are created as needed while
the application is running, so connection creation errors may instead occur
later, while the pool is growing and the application is serving requests.

The Real-World Performance Group also recommends keeping pool sizes
small because they often can perform better than larger pools. The pool
attributes should be adjusted to handle the desired workload within the
bounds of available resources in rust-oracledb and the database.

#### <a name="connpoolgrowth"></a> 2.5.6.1 Connection Pool Growth

At pool creation, the
[PoolConfig::set_min_connections()](crate::PoolConfig::set_min_connections) is
used to set the number of connections that is to be established to the
database. When a pool needs to grow, new connections are created automatically
limited by the maximum pool size defined using
[PoolConfig::set_max_connections()](crate::PoolConfig::max_connections). This
value restricts the number of application users that can do work in parallel on
the database.

The number of connections opened by a pool can be seen with the
[Pool::open_count()](crate::Pool::open_count). The number of connections that
the application has obtained with [Pool::acquire()](crate::Pool::acquire) can
be shown with [Pool::busy_count()](crate::Pool::busy_count). The difference in
values is the number of connections unused or 'idle' in the pool. These idle
connections may be candidates for the pool to close, depending on the pool
configuration.

Pool growth is normally initiated when [Pool::acquire()](crate::Pool::acquire)
is called and there are no idle connections in the pool that can be returned
to the application. The number of new connections created internally will be
the value returned by the method
[PoolConfig::set_connection_increment()](crate::PoolConfig::set_connection_increment).

A connection pool can shrink back to its minimum size
[PoolConfig::set_min_connections()](crate::PoolConfig::set_min_connections)
when connections opened by the pool are not used by the application. This frees
up database resources while allowing pools to retain open connections for
active users.

### <a name="poolhealth"></a> 2.5.7 Pool Connection Health

Before [Pool::acquire()](crate::Pool::acquire) returns an idle pooled
connection, rust-oracledb checks whether the connection has been marked as
requiring close.

These checks will not detect all cases where a connection has become unusable,
such as when the network connection is silently closed, the database session is
terminated by a DBA, or a database resource manager quota is reached. To
help in those cases, [Pool::acquire()](crate::Pool::acquire) will also do a
full [round-trip](#roundtrips) database ping similar to
[Connection::ping()](crate::Connection::ping) when it is about to return a
connection that was idle in the pool (that is, not acquired by the application)
for [Pool::ping_interval()](crate::PoolConfig::ping_interval) seconds. If the
ping fails, the connection will be discarded and another one obtained before
[Pool::acquire()](crate::Pool::acquire) returns to the application.

Because this full ping is time based and may not occur for each
[Pool::acquire()](crate::Pool::acquire), the application may still get an
unusable connection. Also, network timeouts and session termination may occur
between the calls to [Pool::acquire()](crate::Pool::acquire) and
[Connection::execute()](crate::Connection::execute). To handle these cases,
applications need to check for errors after each
[Connection::execute()](crate::Connection::execute) and make
application-specific decisions about retrying work if there was a
connection failure.

You can explicitly initiate a full round-trip ping at any time with
[Connection::ping()](crate::Connection::ping) to check connection liveness but
overuse will impact performance and scalability. To avoid pings hanging due to
network errors, use
[Connection::set_call_timeout()](crate::Connection::set_call_timeout) or
[PoolConfig::set_ping_timeout()](crate::PoolConfig::set_ping_timeout) to limit
the amount of time [Connection::ping()](crate::Connection::ping) is allowed to
take.

Connection pool health can be impacted by [firewalls](#hanetwork),
[resource managers][resource-managers] or user profile [IDLE_TIME][idletime]
values. For best efficiency, ensure these do not expire idle sessions
since this will require connections to be recreated which will impact
performance and scalability.

A pool's internal connection re-establishment after lightweight and full pings
can mask performance-impacting configuration issues such as firewalls
terminating connections. You should monitor [AWR] reports for an unexpectedly
large connection rate.

## <a name="deepdatasecurity"></a> 2.6 Deep Data Security

Oracle Deep Data Security is a database-enforced data authorization framework
which enables you to specify application-level security requirements directly
at the database layer. Deep Data Security ensures fine-grained and end-to-end
user access control at the row, column, and cell levels. Deep Data Security
requires Oracle Database 26ai.

With Deep Data Security, an application sends a specific set of identity and
authorization details to the database called end-user security context payload.
The details that can be defined in an end-user security context payload are an
end-user identity, a database-access token, data roles, and end-user context
attributes. The end-user security context payload can be defined on a
connection. Once defined, the database uses these end user details to authorize
and grant access to the data. See [Oracle Deep Data Security] Configuration
Guide for more information.

### <a name="createendusersecctx"></a> 2.6.1 Creating an End-User Security Context Payload

An End-User Security Context payload can be created for an end-user managed by
an external Identity and Access Management (IAM) system, such as Oracle Cloud
Infrastructure (OCI) IAM or Microsoft Entra ID, or for an end-user locally
managed in Oracle Database.

An End-User Security Context payload can contain the following values:

- **End-user token**: An end-user token issued by an external Identity and
  Access Management (IAM) system for the application end user. This value must
  be specified for users managed by external IAM systems.

- **Database access token**: A token that allows the database to validate and
  use the End-User Security Context payload. This value must be specified for
  users managed by external IAM systems, and Oracle Database.

- **Name**: The name of an end-user locally managed in Oracle Database. This
  value must be specified for users managed by Oracle Database.

- **Key**: An optional end-user context identifier, which the database uses as
  the look-up key for the database-managed end-user security context. This
  value can be optionally specified for users managed by Oracle Database.

- **Data roles**: The names of data roles granted to the application. These
  optional data roles are created with a ``CREATE DATA ROLE`` statement in the
  database and granted to an application identity created with a
  ``CREATE APPLICATION IDENTITY`` statement in the database. During end-user
  security context creation, the database determines the application identity
  based on the database-access token provided by the client in the end-user
  security context payload.

  For external IAM systems, the data roles created in the database are mapped
  to the corresponding roles managed in your IAM system.

  For local database users, these data roles can be used to distinguish
  sessions for the same local user.

- **Attributes**: Attribute name-value pairs provided by the application for an
  END USER CONTEXT declared in the database.

  The name-value pairs for each context must conform to the JSON schema of that
  END USER CONTEXT. These pairs are associated with fully qualified
  END USER CONTEXT names, using the format ``{schema}.{name}``, where
  ``schema`` is the database schema in which the context is declared and
  ``name`` is the name of the END USER CONTEXT. The database does not recognize
  unqualified END USER CONTEXT names. The attribute values can be referenced at
  runtime by authorization policies, for example in data grant predicates, and
  application logic.

To create an End-User Security Context payload, call
[EndUserSecurityContext::new()](crate::EndUserSecurityContext::new) or
[EndUserSecurityContext::builder()](crate::EndUserSecurityContext::builder)
with an [EndUserIdentity](crate::EndUserIdentity) value and a database access
token.

For an end-user managed by an external IAM system, use
[EndUserIdentity::Token](crate::EndUserIdentity::Token). The token must be
issued by the external identity provider. For example:

```rust
let context = oracledb::EndUserSecurityContext::new(
    oracledb::EndUserIdentity::Token(end_user_token),
    database_access_token,
)?;
```

For an end-user locally managed in Oracle Database, use
[EndUserIdentity::DatabaseUser](crate::EndUserIdentity::DatabaseUser), specifying
the end-user name with `name` and, optionally, an end-user context identifier
with `key`. For example:

```rust
let context = oracledb::EndUserSecurityContext::new(
    oracledb::EndUserIdentity::DatabaseUser {
        name: "app_end_user".to_string(),
        key: None,
    },
    database_access_token,
)?;
```

The optional key value can be used to pass the end-user context identifier:

```rust
let context = oracledb::EndUserSecurityContext::new(
    oracledb::EndUserIdentity::DatabaseUser {
        name: "app_end_user".to_string(),
        key: Some("end_user_context_id".to_string()),
    },
    database_access_token,
)?;
```

If the end-user security context payload needs to include data roles and
attributes, then use
[EndUserSecurityContext::builder()](crate::EndUserSecurityContext::builder).
Data roles are passed as a `Vec<String>`. Attributes are passed as a
`HashMap<String, JsonValue>`, where each key is the attribute name and each
value is the attribute value to include in the payload. For example:

```rust
use std::collections::HashMap;

// Define optional end-user attributes for the security context payload
let mut attributes = HashMap::new();

attributes.insert(
    "department".to_string(),
    oracledb::JsonValue::String("finance".to_string()),
);

// Create an end-user security context payload with an IAM-managed
// end-user token, optional data roles, and optional attributes
let context = oracledb::EndUserSecurityContext::builder(
    oracledb::EndUserIdentity::Token(end_user_token),
    database_access_token,
)
.data_roles(vec![
    "hcm_role".to_string()
])
.attributes(attributes)
.build()?;
```

### <a name="setendusersecctx"></a> 2.6.2 Setting an End-User Security Context Payload

After creating an [EndUserSecurityContext](crate::EndUserSecurityContext),
attach it to a connection by calling
[Connection::set_end_user_security_context()](crate::Connection::set_end_user_security_context).
This must be called after connection creation.

```rust
let config = oracledb::Config::default()
    .set_credentials("app_user", "password")
    .set_connect_string("tcps://dbhost.example.com:1522/service_name")?;

let connection = oracledb::connect(config)?;

connection.set_end_user_security_context(context)?;
```

Once
[Connection::set_end_user_security_context()](crate::Connection::set_end_user_security_context)
is called, the specified context applies to all subsequent database operations
executed on that connection for that end user.

### <a name="clearendusersecctx"></a> 2.6.3 Clearing an End-User Security Context Payload

To clear an end-user security context payload on a connection, use
[Connection::clear_end_user_security_context()](crate::Connection::clear_end_user_security_context). For example:

```rust
connection.clear_end_user_security_context()?;
```

## <a name="privilegedconn"></a> 2.7 Privileged Connections

The [Config::set_auth_mode()](crate::Config::set_auth_mode) method can be used
to specify the database privilege that you want to associate with the user.

The example below shows how to connect to Oracle Database as SYSDBA:

```rust
let config = oracledb::Config::default()
    .set_credentials("hr", "password")
    .set_connect_string("dbhost.example.com/orclpdb")?
    .set_auth_mode(oracledb::AUTH_MODE_SYSDBA)?;
```

## <a name="netencrypt"></a> 2.8 Securely Encrypting Network Traffic to Oracle Database

Rust-oracledb supports TLS connections when using TCPS-style connection strings
or descriptors. For encrypted traffic, use TCPS/TLS configuration where
supported.

See the [Oracle Database Security Guide][oracle-db-security-guide] for more
configuration information.

## <a name="resetpassword"></a> 2.9 Resetting Passwords

After connecting to Oracle Database, passwords can be changed by calling
[Connection::change_password()](crate::Connection::change_password):

```rust
connection.changepassword("oldpwd", "newpwd");
```

When a password has expired and you cannot connect directly, you can connect
and change the password in one operation by using the
[Config::set_new_password()](crate::Config::set_new_password) method:

```rust
let config = oracledb::Config::default()
    .set_credentials("hr", "oldpassword")
    .set_new_password("newpassword")?;
```

## <a name="autonomousdb"></a> 2.10 Connecting to Oracle Cloud Autonomous Databases

Rust applications can connect to Oracle Autonomous Database (ADB) in
Oracle Cloud using one-way TLS (Transport Layer Security) or mutual TLS
(mTLS), depending on how the database instance is configured. One-way
TLS and mTLS provide enhanced security for authentication and
encryption.

A database username and password are still required for your application
connections. Refer to the relevant Oracle Cloud documentation, for example see
[Create Database Users][create-db-users].

### <a name="onewaytls"></a> 2.10.1 One-way TLS Connection to Oracle Autonomous Database

With one-way TLS, the rust-oracledb host machine must be in the Access
Control List (ACL) of the ADB instance. Applications then connect to
Oracle ADB by passing the database username, password, and appropriate
connection string. A wallet is not used.

#### <a name="allowonewaytls"></a> 2.10.1.1 Allowing One-way TLS Access to Oracle Autonomous Database

To create an ADB instance that allows one-way TLS, choose the access setting
*Secure access from allowed IPs and VCNs only* in the Oracle Cloud console
during instance creation. Then specify the IP addresses, hostnames, CIDR
blocks, Virtual Cloud networks (VCN), or Virtual Cloud network OCIDs where Rust
will be running. The ACL limits access to only the resources that have been
defined and blocks all other incoming traffic.

Alternatively, to enable one-way TLS on an existing database, complete the
following steps in the Oracle Cloud console in the **Autonomous Database
Information** section of the ADB instance:

1. Click the **Edit** link next to *Access Control List* to update the Access
   Control List (ACL).
2. In the displayed **Edit Access Control List** dialog box, select the type of
   address list entries and the corresponding values. You can include the IP
   addresses, hostnames, CIDR blocks, Virtual Cloud networks (VCN), or Virtual
   Cloud network OCIDs where Rust will be running.
3. Navigate back to the ADB instance details page and click the **Edit** link
   next to *Mutual TLS (mTLS) Authentication*.
4. In the displayed **Edit Mutual TLS Authentication** dialog box, deselect the
   **Require mutual TLS (mTLS) authentication** check box to disable the mTLS
   requirement on Oracle ADB and click **Save Changes**.

#### <a name="connectonewaytls"></a> 2.10.1.2 Connecting with One-way TLS

When your database has been enabled to allow one-way TLS, you can connect with
rust-oracledb by following these steps:

1. Navigate to the ADB instance details page on the Cloud console and click
   **Database connection** at the top of the page.
2. In the displayed **Database Connection** dialog box, select TLS from the
   **Connection Strings** drop-down list.
3. Copy the appropriate Connection String for the connection service level you
   want.

Applications can connect using database credentials and the copied
[connection string](#conndescriptor). Do *not* pass wallet parameters. For
example, to connect as the ADMIN user:

```rust
let descriptor = r#"(
    DESCRIPTION=
        (ADDRESS=(PROTOCOL=TCPS)(HOST=adb.example.oraclecloud.com)(PORT=1522))
        (CONNECT_DATA=(SERVICE_NAME=mydb_high.adb.oraclecloud.com))
        (SECURITY=(SSL_SERVER_DN_MATCH=yes))
)"#;

let config = oracledb::Config::default()
    .set_credentials("admin", "password")
    .set_connect_string(descriptor)?;

let connection = oracledb::connect(config)?;
```

If you prefer to keep connection descriptors out of application code,
you can add the descriptor with a [TNS Alias](#netservice) to a
[tnsnames.ora](#optnetfile) file, and use the TNS alias in the
[Config::set_connect_string()][crate::Config::set_connect_string].

Not having the ACL correctly configured is a common cause of connection
errors. To aid troubleshooting, remove `(retry_count=20)(retry_delay=3)`
from the connect descriptor so that errors are returned faster. If
network configuration issues are suspected then, for initial
troubleshooting with a disposable database, you can update the ACL to
contain a CIDR block of `0.0.0.0/0`, however this means *anybody* can
attempt to connect to your database so you should recreate the database
immediately after identifying a working, more restrictive ACL.

### <a name="twowaytls"></a> 2.10.2 Mutual TLS (mTLS) Connection to Oracle Autonomous Database

To enable rust-oracledb connections to Oracle Autonomous Database in Oracle
Cloud using mTLS, a wallet needs to be downloaded from the cloud console. mTLS
is sometimes called Two-way TLS.

#### <a name="allowmtls"></a> 2.10.2.1 Allowing mTLS Access to Oracle Autonomous Database

When creating an ADB instance in the Oracle Cloud console, choose the access
setting "Secure access from everywhere".

#### <a name="getwallet"></a> 2.10.2.2 Downloading the Database Wallet

After your Autonomous Database has been enabled to allow mTLS, download its
`wallet.zip` file which contains the certificate and network configuration
files:

1. Navigate to the ADB instance details page on the Oracle Cloud console and
   click **Database connection** at the top of the page.
2. In the displayed **Database Connection** dialog box, select the
   "Download Wallet" button in the *Download client credentials (Wallet)*
   section. The cloud console will ask you to create a wallet password.

**Note**: Keep wallet files in a secure location and only share them and
the password with authorized users.

#### <a name="connectmtls"></a> 2.10.2.3 Connecting with mTLS

For rust-oracledb, unzip the [wallet.zip](#getwallet) file. Only two files from
it are needed:

- `tnsnames.ora` - Maps TNS Aliases used for application connection strings to
  your database services.
- `ewallet.pem` - Enables SSL/TLS connections. Keep this file secure.

If you do not have a PEM file, see [Create a PEM File](#createpem).

Move the two files to a directory that is accessible by your application. In
this example, the files are located in the same directory,
`/opt/OracleCloud/MYDB`.

A connection can be made by using your database credentials and setting the
connect string to the desired [TNS Alias](#netservice) from the
[tnsnames.ora](#optnetfile) file. The TNS alias lookup uses the default
configuration directory from TNS_ADMIN or ORACLE_HOME/network/admin. The
[Config::set_wallet_location()](crate::Config::set_wallet_location) method is
the directory containing the PEM file. The
[Config::set_wallet_password()](crate::Config::set_wallet_password) method
should be used to set the password created in the cloud console when
downloading the wallet. It is not the database user or ADMIN password. For
example, to connect as the ADMIN user using the `mydb_low` TNS Alias:

```rust
let config = oracledb::Config::default()
    .set_credentials("admin", "password") // database user and password for ADMIN
    .set_config_dir("/opt/OracleCloud/MYDB") // directory with tnsnames.ora
    .set_wallet_location("/opt/OracleCloud/MYDB") // directory with ewallet.pem
    .set_wallet_password(&wp) // not a database user password
    .set_connect_string("mydb_low")?; // TNS Alias from tnsnames.ora

// Establish the connection – the driver will load the wallet and use the
// config directory for any TNS alias resolution.
let connection = Connection::connect(config)?;

// Example query – prove that the connection works.
let rows = connection.query("SELECT USER FROM DUAL", &[])?;
for row in rows {
    println!("Connected user: {}", row.get::<String>(0))?;
}
```

#### <a name="easyconnectadb"></a> 2.10.2.4 Using the Easy Connect Syntax with Oracle Autonomous Database

This section discuss the parameters for mTLS connection.

The mapping from the cloud [tnsnames.ora](#optnetfile) entries to an Easy
Connect string is:

```text
protocol://host:port/service_name?wallet_location=/my/dir&retry_count=N&retry_delay=N
```

For example, if your `tnsnames.ora` file had an entry:

```text
cjjson_high = (description=(retry_count=20)(retry_delay=3)
    (address=(protocol=tcps)(port=1522)
    (host=xxx.oraclecloud.com))
    (connect_data=(service_name=abc_cjjson_high.adb.oraclecloud.com))
    (security=(ssl_server_cert_dn="CN=xxx.oraclecloud.com,O=Oracle Corporation,L=Redwood City,ST=California,C=US")))
```

Then your applications can connect using the connection string:

```rust
let cs = "tcps://xxx.oraclecloud.com:1522/abc_cjjson_high.adb.oraclecloud.com?\
    wallet_location=/Users/cjones/Cloud/CJJSON&retry_count=20&retry_delay=3";

let config = Config::default()
    .set_credentials("hr", userpwd)
    .set_connect_string(cs)?;

// Establish the connection
let connection = oracledb::connect(config)?;
```

You must set the
[Config::set_wallet_location()](crate::Config::set_wallet_location) method to
the directory containing the `ewallet.pem` file extracted from the
[wallet.zip](#getwallet) file. The other files, including `tnsnames.ora`, are
not needed when you use the Easy Connect syntax.

The wallet password needs to be passed as a connection parameter.

#### <a name="createpem"></a> 2.10.2.5 Creating a PEM File

For mutual TLS in rust-oracledb mode, the certificate must be Privacy Enhanced
Mail (PEM) format. If you are using Oracle Autonomous Database, your wallet zip
file will already include a PEM file.

If you are using Oracle Autonomous Database and
your wallet zip file does not already include a PEM file, then you can convert
the PKCS12 ``ewallet.p12`` file to PEM format using third party tools. For
example, using OpenSSL:

```text
openssl pkcs12 -in ewallet.p12 -out wallet.pem
```

Once the PEM file has been created, you can use it by passing its directory
location in the
[Config::set_wallet_location()](crate::Config::set_wallet_location).


[AWR]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-56AEF38E-9400-427B-A818-EDEC145F7ACD
[configure-naming-methods]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-E5358DEA-D619-4B7B-A799-3D2F802500F1
[conn-descriptor-desc]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-012BCA50-70FC-4951-9473-B6089718FF1C
[connpool-stop-pool]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-3FF5F327-7BE3-4EA8-844F-29554EE00B5F
[create-db-users]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-B5846072-995B-4B81-BDCB-AF530BC42847
[Database Authentication of Users]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-1F783131-CD1C-4EA0-9300-C132651B0700
[easy-connect]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-59956F00-4996-4943-8D8B-9720DC67AD5D
[easy-connect-plus]: https://download.oracle.com/ocomdocs/global/Oracle-Net-Easy-Connect-Plus.pdf
[https-proxy]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-C672E92D-CE32-4759-9931-92D7960850F7
[https-proxy-port]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-E69D27B7-2B59-4946-89B3-5DDD491C2D9A
[idletime]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-ABC7AE4D-64A8-4EA9-857D-BEF7300B64C3
[local-naming-parameters]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-7F967CE5-5498-427C-9390-4A5C6767ADAA
[ORA-28547]: https://docs.oracle.com/error-help/db/ora-28547/
[oracle-db-security-guide]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-41040F53-D7A6-48FA-A92A-0C23118BC8A0
[Oracle Deep Data Security]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-E239A5C4-0C0D-4FF0-98DD-2E374F79C63C
[oracle-rac]: https://www.oracle.com/database/real-application-clusters/
[overview-local-naming-parameters]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&d=GUID-12C94B15-2CE1-4B98-9D0C-8226A9DDF4CB
[pool-name]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-C2DA6A42-C30A-4E4C-9833-51CB383FE08B
[resource-managers]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-2BEF5482-CF97-4A85-BD90-9195E41E74EF
[support-easy-connect-plus]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-8C85D289-6AF3-41BC-848B-BF39D32648BA
[use-static-pools]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-7DFBA826-7CC0-4D16-B19C-31D168069B54
[v$session-connect-info]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-9F0DCAEA-A67E-4183-89E7-B1555DC591CE
[client access through a proxy]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-D77D0D4A-7483-423A-9767-CBB5854A15CC
