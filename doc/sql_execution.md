# <a name="sqlexecution"></a> 3. Executing SQL

Executing SQL statements is the primary way in which a Rust application
communicates with Oracle Database. Statements include queries, Data
Manipulation Language (DML), and Data Definition Language (DDL). A few other
[specialty statements] can also be executed.

SELECT statements are executed using one of these methods
[Connection::query()](crate::Connection::query) or
[Connection::query_row()](crate::Connection::query_row()).

All other statements are executed using
[Connection::execute()](crate::Connection::execute)

PL/SQL statements are discussed in [PL/SQL](#plsqlexecution).

To help you create SQL and PL/SQL statements, Oracle hosts
[FreeSQL.com][freesql] which gives you an online editor immediately connected
to a database. It also has tutorials and a library of statements.

**SQL Statement Syntax**

SQL statements executed in rust-oracledb should not contain a trailing
semicolon (";") or forward slash ("/"). This will fail:

```rust
connection.query_row("select * from MyTable;")   // fails due to semicolon
```

This is correct:

```rust
connection.query_row("select * from MyTable")
```

**IMPORTANT**: Interpolating or concatenating user data with SQL statements,
for example
`connection.query(&format!("SELECT * FROM mytab WHERE mycol = '{myvar}'"), &[])`,
is a security risk and impacts performance. Use [bind variables](#bind) instead,
for example
`connection.query("SELECT * FROM mytab WHERE mycol = :1", &[&myvar])`.

## <a name="sqlqueries"></a> 3.1 SELECT Statements

Queries (statements beginning with SELECT or WITH) can be executed using
the method [Connection::query()](crate::Connection::query) or
[Connection::query_row()](crate::Connection::query_row()).

### <a name="fetchsinglerow"></a> 3.1.1 Fetching a Single Row

To fetch a single row, use
[Connection::query_row()](crate::Connection::query_row()). This method returns
a row directly. For example:

```rust
use oracledb;

fn main() -> Result<(), oracledb::Error> {

    let config = oracledb::Config::default()
       .set_credentials("hr", "password")
       .set_connect_string("localhost:1521/orclpdb")?;

    // establish standalone connection to the database
    let connection = oracledb::connect(config)?;

    // perform a query and display the results of that query
    let row = connection.query_row("select user from dual", &[])?;
    let result: String = row.get(0)?;
    println!("Connected user is {}", result);

    Ok(())
}
```

This prints the following output:

```text
Connected user is HR
```

### <a name="fetchmultiplerows"></a> 3.1.2 Fetching Multiple Rows

If your SQL statement may return more than one row, use
[Connection::query()](crate::Connection::query) to execute the statement. This
method returns a Cursor, which allows you to iterate over the results one row
at a time. A cursor represents the result set of a query and provides access to
rows returned by the database. Cursors are automatically closed when they go
out of scope.

An example of fetching multiple rows is shown below:


```rust
use oracledb;

fn main() -> Result<(), oracledb::Error> {

    let cursor = connection.query(
        "SELECT first_name FROM employees ORDER BY employee_id",
        &[],
    )?;

    for row in cursor {
        let row = row?;
        let result: String = row.get(0)?;
        println!("{}", result);
    }

    Ok(())
}
```

### <a name="defaultfetchtypes"></a> 3.1.3 Fetch Data Types

The following table lists Oracle Database types that rust-oracledb can fetch,
the corresponding oracledb database type, and common Rust types that values can
be fetched as. The application specifies the Rust type, for example with
`row.get()` or by assigning the result to a typed variable.

The first column lists the Oracle Database Type. The second column displays the
oracledb Database Type. The third column displays the type of Rust object that
is fetched.

| Oracle Database Type           | oracledb Database Type | Supported Rust Fetch Types  |
| ------------------------------ | ---------------------- | ----------------------------|
| CHAR                           | DB_TYPE_CHAR           | `String`                    |
| VARCHAR2                       | DB_TYPE_VARCHAR        | `String`                    |
| NCHAR                          | DB_TYPE_NCHAR          | `String`                    |
| NVARCHAR2                      | DB_TYPE_NVARCHAR       | `String`                    |
| NUMBER                         | DB_TYPE_NUMBER         | [OracleNumber](crate::OracleNumber), `i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `u8`, `u16`, `u32`, `u64`, `u128` and `usize`                         |
| BINARY_FLOAT                   | DB_TYPE_BINARY_FLOAT   | `f32`                       |
| BINARY_DOUBLE                  | DB_TYPE_BINARY_DOUBLE  | `f64`                       |
| DATE                           | DB_TYPE_DATE           | [OracleTimestamp](crate::OracleTimestamp) |
| TIMESTAMP                      | DB_TYPE_TIMESTAMP      | [OracleTimestamp](crate::OracleTimestamp) |
| TIMESTAMP WITH TIME ZONE       | DB_TYPE_TIMESTAMP_TZ   | [OracleTimestamp](crate::OracleTimestamp) |
| TIMESTAMP WITH LOCAL TIME ZONE | DB_TYPE_TIMESTAMP_LTZ  | [OracleTimestamp](crate::OracleTimestamp) |
| CLOB                           | DB_TYPE_CLOB           | `String`                    |
| NCLOB                          | DB_TYPE_NCLOB          | `String`                    |
| BLOB                           | DB_TYPE_BLOB           | `Vec<u8>`                   |
| RAW                            | DB_TYPE_RAW            | `Vec<u8>`                   |
| LONG                           | DB_TYPE_LONG           | `String`                    |
| LONG RAW                       | DB_TYPE_LONG_RAW       | `Vec<u8>`                   |
| JSON                           | DB_TYPE_JSON           | [JsonValue](crate::JsonValue)        |
| VECTOR                         | DB_TYPE_VECTOR         | [Vector](crate::Vector)          |

When fetching NUMBER values as Rust integer types, the value must be in range
for the requested Rust type. Binary types are represented as byte arrays
(`Vec<u8>`).

### <a name="rowlimit"></a> 3.1.4 Limiting Rows

Query data is commonly broken into one or more sets:

- To give an upper bound on the number of rows that a query has to process,
  which can help improve database scalability.
- To perform 'Web pagination' that allows moving from one set of rows to a
  next, or previous, set on demand.
- For fetching of all data in consecutive small sets for batch processing. This
  happens because the number of records is too large for Rust to handle at one
  time.

The latter can be handled by calling
[Connection::query()](crate::Connection::query) with one execution of the SQL
query.

'Web pagination' and limiting the maximum number of rows are detailed in this
section. For each 'page' of results, a SQL query is executed to get the
appropriate set of rows from a table. Since the query may be executed more than
once, ensure to use [bind variables](#bind) for row numbers and row limits.

Oracle Database 12c SQL introduced an `OFFSET` / `FETCH` clause which is
similar to the `LIMIT` keyword of MySQL. In Rust, you can fetch a set of rows
using:

```rust
let myoffset: i32 = 0;       # do not skip any rows (start at row 1)
let mymaxnumrows: i32 = 20   # get 20 rows

let sql = r#"
        SELECT last_name
        FROM employees
        ORDER BY last_name
        OFFSET :offset ROWS FETCH NEXT :maxnumrows ROWS ONLY
    "#;

let mut stmt = conn
        .statement(sql)
        .fetch_array_size(mymaxnumrows as usize)
        .build()?;

for row_result in stmt.query_as_named::<String>(&[
        ("offset", &myoffset),
        ("maxnumrows", &mymaxnumrows),
    ])? {
        let last_name = row_result?;
        println!("{last_name}");
    }
```

In applications where the SQL query is not known in advance, this method
sometimes involves appending the `OFFSET` clause to the 'real' user
query. Be very careful to avoid SQL injection security issues.

Ensure to use [bind variables](#bind) for the upper and lower limit values.

## <a name="dml"></a> 3.2 INSERT and UPDATE Statements

SQL Data Manipulation Language statements (DML) such as INSERT and
UPDATE can easily be executed with rust-oracledb by using
[Connection::execute()](crate::Connection::execute). For example:

```rust
connection.execute("insert into MyTable values (:idbv, :nmbv)", &[&1, &"Fredico"])?;
```

Do not concatenate or interpolate user data into SQL statements. See
[Using Bind Variables](#bind) instead.

When handling multiple data values, use
[Connection::execute_batch()](crate::Connection::execute_batch). See
[Batch Statement Operations](#batchstmnt).

By default data is not committed to the database and other users will
not be able to see your changes until your connection commits them by
calling [Connection::commit()](crate::Connection::commit). You can optionally
rollback changes by calling
[Connection::rollback()](crate::Connection::rollback). An implicit rollback
will occur if your application finishes and does not explicitly commit any
work.

To commit your changes, call:

```rust
connection.commit()?;
```

Note that the commit occurs on the connection.

See [Managing Transactions](#txnmgmnt) for best practices on committing and
rolling back data changes.

## <a name="validatingsql"></a> 3.3 Dynamic SQL Construction and Validation

When dynamically building SQL statements, you can use the methods
[oracledb::enquote_name()](crate::utils::enquote_name()),
[oracledb::enquote_literal()](crate::utils::enquote_literal()),
[oracledb::is_qualified_sql_name()](crate::utils::is_qualified_sql_name()), and
[oracledb::is_simple_sql_name()](crate::utils::is_simple_sql_name()) to help
prevent SQL injection when processing user input.

**IMPORTANT**: When constructing SQL statements dynamically, do not concatenate
or interpolate data values into SQL text. Instead, use bind variables for all
data values. See [Using bind variables](#bind).

### <a name="quotenames"></a> 3.3.1 Quoting SQL Identifiers

[oracledb::enquote_name()](crate::utils::enquote_name()) is used to safely
quote SQL identifiers such as table names or column names. This can be used
when you need to dynamically include identifiers in your SQL statement. For
example, if your application allows users to provide an arbitrary column name
to filter query results, you could ensure that the column name supplied by the
user is validated with `enquote_name()`. For the data value itself, you would
continue to use [bind variable](#bind) syntax:

```rust
let col = "DEPARTMENT_NAME";
let val = "SALES";

let col = oracledb::enquote_name(col)?;
let sql = "select * from departments where {col} = :1";
let cursor = connection.query(&sql, &[&val])?;
```

### <a name="quoteliterals"></a> 3.3.2 Quoting Literals

When including literal values dynamically in SQL statements, it is important
to quote them properly so that SQL interprets them correctly. This can be done
by using [oracledb::enquote_literal()](crate::utils::enquote_literal()). Note
that quoting literals should only be done when [bind variables](#bind) cannot
be used.

An example of quoting literals is shown below:

```rust
let val = oracledb::enquote_literal("O'Reilly");
let sql = ["select * from employees where last_name = ", &val].concat();

println!("{sql}");
```

This prints:

```text
select * from employees where last_name = 'O''Reilly'
```

Note how the single quote in "O'Reilly" is automatically escaped (''), so the
SQL remains valid.

### <a name="validatesimplesqlnames"></a> 3.3.3 Validating Simple SQL Names

[oracledb::is_simple_sql_name()](crate::utils::is_simple_sql_name()) checks
whether the input value contains a valid SQL name. If the value is not quoted,
the first character must be alphabetic and the remaining characters must be
alphanumeric or contain the characters '_', '$', or '#'. A quoted name may not
contain embedded quotes and no characters other than whitespace are allowed
outside the quotes. Some valid and invalid SQL names are shown in the following
example:

```rust
// Valid Simple SQL Names
println!("{}", oracledb::is_simple_sql_name("employee_id"))  // true
println!("{}", oracledb::is_simple_sql_name("Salary"))       // true
println!("{}", oracledb::is_simple_sql_name("dept2"))        // true
println!("{}", oracledb::is_simple_sql_name(" \"EMP\" "))    // true

// Invalid Simple SQL Names
println!("{}", oracledb::is_simple_sql_name("123column"))  // false (starts with a number)
println!("{}", oracledb::is_simple_sql_name("first-name")) // false (contains hyphen)
println!("{}", oracledb::is_simple_sql_name("first name")) // false (contains space)
println!("{}", oracledb::is_simple_sql_name(""))           // false (empty string)
println!("{}", oracledb::is_simple_sql_name(" \"EMP\"X ")) // false (characters outside quotes)
```

### <a name="validatequalifiedsqlnames"></a> 3.3.4 Validating Qualified SQL Names

[oracledb::is_qualified_sql_name()](crate::utils::is_qualified_sql_name())
checks whether the input value contains a valid qualified SQL name. The name
must be one or more simple SQL names separated by periods (and any amount of
whitespace), optionally followed by the '@' symbol and one or more simple SQL
names referring to a database link name. Some valid and invalid SQL names are
shown in the following example:

```rust
// Valid Qualified SQL Names
println!("{}", oracledb::is_qualified_sql_name("HR.employees"))     // true
println!("{}", oracledb::is_qualified_sql_name("SALES.Order"))      // true
println!("{}", oracledb::is_qualified_sql_name("MYSCHEMA.MyTable")) // true

// Invalid Qualified SQL Names
println!("{}", oracledb::is_qualified_sql_name("HR..Employees"))  // false (contains double dot)
println!("{}", oracledb::is_qualified_sql_name("HR.123Orders"))   // false (object name
                                                                 // starts with number)
println!("{}", oracledb::is_qualified_sql_name("HR.Orders-2026")) // false (contains hyphen)
```

[specialty statements]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-E1749EF5-2264-44DF-99EF-AEBEB943BED6
[freesql]: https://freesql.com/
