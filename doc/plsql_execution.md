# <a name="plsqlexecution"></a> 4. Executing PL/SQL

PL/SQL is a procedural language used for creating user-defined procedures,
functions, and anonymous blocks. PL/SQL program units are compiled and run
inside Oracle Database, letting them efficiently work on data. Procedures and
functions can be stored in the database, encapsulating business logic for reuse
in other applications.

PL/SQL code can be stored in the database, and executed using rust-oracledb.

Examples in this chapter show single invocations using
[Connection::execute()](crate::Connection::execute). Examples of repeated
calls using [Connection::execute_batch()](crate::Connection::execute_batch) are
shown in [Executing Batch Statements](#batchplsql).

## <a name="plsqlproc"></a> 4.1 PL/SQL Stored Procedures

PL/SQL procedures can be called by executing an anonymous PL/SQL block.

If a procedure with the following definition exists:

```sql
create or replace procedure myproc (
    a_Value1 number,
    a_Value2 out number
) as
begin
    a_Value2 := a_Value1 * 2;
end;
```

then the following Rust code can be used to call it:

```rust
let mut result = connection.execute(
    "begin myproc(:1, :2); end;",
    &[&123, &0],
)?;

let returned_data = result.returned_data();
let out_val: i32 = returned_data[0].get(0)?;

println!("{out_val}"); // will print 246
```

The OUT bind value is returned from
[ExecResult::returned_data()](crate::ExecResult::returned_data).

See [Using Bind Variables](#bind) for information on binding.

## <a name="plsqlfunc"></a> 4.2 PL/SQL Stored Functions

PL/SQL functions can be called by executing an anonymous PL/SQL block with
[Connection::execute()](crate::Connection::execute).

If a PL/SQL function with the following definition exists:

``` sql
create or replace function myfunc (
    a_StrVal varchar2,
    a_NumVal number,
    a_Date out date
) return number as
begin
    select sysdate into a_Date from dual;
    return length(a_StrVal) + a_NumVal * 2;
end;
```

then the following Rust code can be used to call it:

```rust
use oracledb::OracleTimestamp;

let date_hint = OracleTimestamp::new_date(1970, 1, 1);

let mut result = connection.execute(
    "begin :1 := myfunc(:2, :3, :4); end;",
    &[&0, &"a string", &15, &date_hint],
)?;

let returned_data = result.returned_data();

let return_val: i32 = returned_data[0].get(0)?;
let out_date: OracleTimestamp = returned_data[0].get(1)?;

println!("{return_val}");
println!("{out_date}");
```

This prints the following output:

```text
Return value: 38
OUT date: 2026-05-11T13:11:39.000000000Z
```

See [Using Bind Variables](#bind) for information on binding.

## <a name="anonplsql"></a> 4.3 Anonymous PL/SQL Blocks

An [anonymous PL/SQL block] can be called as shown:

```rust
let mut result = connection.execute_named(
    r#"
    begin
        :out_val := length(:in_val);
    end;
    "#,
    &[
        ("in_val", &"A sample string"),
        ("out_val", &0),
    ],
)?;

let returned_data = result.returned_data();
let out_val: i32 = returned_data[0].get(0)?;

println!("{out_val}"); // will print 15
```

See [Using Bind Variables](#bind) for information on binding.

## <a name="plsqlnull"></a> 4.4 Passing NULL values to PL/SQL

Oracle Database requires a type, even for null values. In rust-oracledb,
scalar NULL values can be bound by using `Option<T>`, where `T` determines
the Oracle Database type. For example:

```rust
let value: Option<i32> = None;

connection.execute(
    "begin myproc(:1); end;",
    &[&value],
)?;
```

## <a name="storedprocpkg"></a> 4.5 Creating Stored Procedures and Packages

To create PL/SQL stored procedures and packages, use
[Connection::execute()](crate::Connection::execute) with a CREATE command.

```rust
connection.execute(
    r#"
    create or replace procedure myprocedure
    (p_in in number, p_out out number) as
    begin
        p_out := p_in * 2;
    end;
    "#,
    &[],
)?;
```

### <a name="plsqlwarning"></a> 4.5.1 PL/SQL Compilation Warnings

When creating PL/SQL procedures, functions, or types in rust-oracledb, the
statement may succeed without returning an error, but Oracle Database may still
return informational messages. These are sometimes known as "success with info"
messages.

If an application needs to display these messages, check
[Connection::last_warning()](crate::Connection::last_warning) after executing
the CREATE statement. A subsequent query from a table such as `USER_ERRORS`
can show more details.

```rust
connection.execute(
    r#"
    create or replace procedure myprocedure as
    begin
        invalid_statement;
    end;
    "#,
    &[],
)?;

if let Some(warning) = connection.last_warning()? {
    println!("Warning: {warning}");
}

let cursor = connection.query(
    r#"
    select line, position, text
    from user_errors
    where name = :1 and type = :2
    order by sequence
    "#,
    &[&"MYPROCEDURE", &"PROCEDURE"],
)?;

for row_result in cursor {
    let row = row_result?;
    let line: i32 = row.get(0)?;
    let position: i32 = row.get(1)?;
    let text: String = row.get(2)?;

    println!("Line {line}, position {position}: {text}");
}
```

The output would be:

```text
Warning: creation succeeded with compilation errors
Line 3, position 13: PLS-00201: identifier 'INVALID_STATEMENT' must be declared
Line 3, position 13: PL/SQL: Statement ignored
```

## <a name="dbmsoutput"></a> 4.6 Using DBMS_OUTPUT

The standard way to print output from PL/SQL is with the [DBMS_OUTPUT] package.

Note, PL/SQL code that uses DBMS_OUTPUT runs to completion before any output
is available to the user. Also, other database connections cannot access the
buffer.

To use DBMS_OUTPUT:

- Call the PL/SQL procedure `DBMS_OUTPUT.ENABLE()` to enable output to
  be buffered for the connection.
- Execute some PL/SQL that calls `DBMS_OUTPUT.PUT_LINE()` to put text in
  the buffer.
- Call `DBMS_OUTPUT.GET_LINE()` or `DBMS_OUTPUT.GET_LINES()` repeatedly
  to fetch the text from the buffer until there is no more output.

For example:

```rust
// Enable DBMS_OUTPUT buffering for this database connection
connection.execute("begin dbms_output.enable(null); end;", &[])?;

connection.execute(
    r#"
    begin
        dbms_output.put_line('Hello from PL/SQL');
        dbms_output.put_line('This line was buffered by DBMS_OUTPUT');
    end;
    "#,
    &[],
)?;

// Keep fetching lines until DBMS_OUTPUT reports that there is no more output
loop {
    // Allocate a string large enough for DBMS_OUTPUT.GET_LINE's line OUT bind
    let line_hint = " ".repeat(32767);

    let mut result = connection.execute(
        "begin dbms_output.get_line(:1, :2); end;",
        &[&line_hint, &0],
    )?;

    // Get the OUT bind values returned by the PL/SQL call
    let returned_data = result.returned_data();

    // Read the line OUT bind. It can be NULL when no line is returned
    let line: Option<String> = returned_data[0].get(0)?;

    // Read the status OUT bind. 0 means a line was returned; 1 means no more
    // lines
    let status: i32 = returned_data[0].get(1)?;

    if status != 0 {
        break;
    }

    println!("{}", line.unwrap_or_default());
}
```

This will produce the following output:

```text
Hello from PL/SQL
This line was buffered by DBMS_OUTPUT
```

## <a name="ebr"></a> 4.7 Edition-Based Redefinition (EBR)

Oracle Database's [Edition-Based Redefinition] feature enables upgrading of the
database component of an application while it is in use, thereby minimizing or
eliminating down time. This feature allows multiple versions of views,
synonyms, PL/SQL objects, and SQL Translation profiles to be used concurrently.
Different versions of the database objects are associated with an "edition".

The edition can be set by executing the SQL statement:

```sql
alter session set edition = <edition name>;
```

[anonymous PL/SQL block]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-826B070B-4888-4398-889B-61A3C6B91349
[DBMS_OUTPUT]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-C1400094-18D5-4F36-A2C9-D28B0E12FD8C
[Edition-Based Redefinition]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-58DE05A0-5DEF-4791-8FA8-F04D11964906
