# <a name="bind"></a> 5. Using Bind Variables

SQL and PL/SQL statements that pass data to and from Oracle Database should
use placeholders in SQL and PL/SQL statements that mark where data is supplied
or returned. A bind variable placeholder is a colon-prefixed identifier or
numeral. For example, ``:dept_id`` and ``:dept_name`` are the two bind variable
placeholders in this SQL statement:

```rust
let sql = "INSERT INTO departments (department_id, department_name)
    VALUES (:dept_id, :dept_name)";

connection.execute(sql, &[&280, &"Facility"])?;
```

As part of execution, the supplied bind variable values *280* and *"Facility"*
are substituted for the placeholders by the database. This is called binding.

Using bind variables is important for scalability and security. They help avoid
SQL Injection security problems because data is never treated as part of an
executable statement when it is parsed.

Bind variables reduce parsing and execution costs when statements are executed
more than once with different data values. If you do not use bind variables,
Oracle must reparse and cache multiple statements. When using bind variables,
Oracle Database may be able to reuse the statement execution plan and context.

**IMPORTANT**: Never concatenate or interpolate user data into SQL statements:

``` rust
let did = 280;
let dnm = "Facility";

// !! Never do this !!
let sql = format!("insert into departments (department_id, department_name)
          values ({did}, '{dnm}')");
connection.execute(sql, &[])?;
```

This is a security risk and can impact performance and scalability.

Bind variables can be used to substitute data, but cannot be used to substitute
the text of the statement. You cannot, for example, use a bind variable
placeholder where a column name or a table name is required. Bind variable
placeholders also cannot be used in Data Definition Language (DDL) statements,
such as CREATE TABLE or ALTER statements.

## <a name="binding"></a> 5.1 Binding by Name or Position

Binding can be done "by name" or "by position".

### <a name="bindbyname"></a> 5.1.1 Bind by Name

A named bind is performed when the bind variables in the Rust statement use the
names of placeholders in the SQL or PL/SQL statement. To execute SQL statements
with named bind parameters, use
[Connection::execute_named()](crate::Connection::execute_named). For example:

```rust
connection.execute_named(
    "INSERT INTO departments (department_id, department_name)
     VALUES (:dept_id, :dept_name)",
    &[
        ("dept_id", &280),
        ("dept_name", &"Facility"),
    ],
)?;
```

The advantages of named binding are that the order of the bind values in the
`execute_named()` method is not important, the names can be meaningful, and the
placeholder names can be repeated while still only supplying the value once in
the application.

An example of reusing a bind variable placeholder is:

```rust
let sql = "UPDATE departments SET department_id = :dept_id + 10 WHERE
          department_id = :dept_id";

connection.execute_named(
    sql,
    &[("dept_id", &280)],
)?;
```

### <a name="bindbyposition"></a> 5.1.2 Bind by Position

Positional binding occurs when a slice of bind values is passed to
[Connection::execute()](crate::Connection::execute). For example:

```rust
connection.execute("INSERT INTO departments (department_id, department_name)
     VALUES (:1, :2)", &[&280, &"Facility"])?;
```

The following example (which changes the order of the bind placeholder names)
has exactly the same behavior. The value used to substitute the placeholder
":2" will be the first element of the list and ":1" will be replaced by the
second element. Bind by position works from left to right and pays no attention
to the name of the bind variable:

```rust
connection.execute("INSERT INTO departments (department_id, department_name)
     VALUES (:2, :1)", &[&280, &"Facility"])?;
```

The following example is also bind by position despite the bind placeholders
having alphabetic names. The actual process of binding uses the list positions
of the input data to associate the data with the placeholder locations:

```rust
connection.execute("INSERT INTO departments (department_id, department_name)
     VALUES (:dept_id, :dept_name)", &[&280, &"Facility"])?;
```

If only a single bind placeholder is used in the SQL or PL/SQL statement, the
bind values can be supplied as a slice containing one value. For example:

```rust
connection.execute("DELETE FROM departments WHERE department_id = :1", &[&280])?;
```

## <a name="dupbindplaceholders"></a> 5.2 Duplicate Bind Variable Placeholders

[Binding by name](#bindbyname) is recommended when bind variable placeholder
names are repeated in statements.

When [binding by position](#bindbyposition) for SQL statements, the order of
the bind values must exactly match the order of each bind variable placeholder
and duplicated names must have their values repeated:

```rust
connection.query("SELECT dname FROM dept1 WHERE deptno = :1 UNION ALL
    SELECT dname FROM dept2 WHERE deptno = :1", &[&30, &30])?;
```

When binding by position for PL/SQL calls, the order of the bind values must
exactly match the order of each **unique** placeholder found in the PL/SQL
block and values should not be repeated.

Binding by name does not have these issues.

## <a name="binddir"></a> 5.3 Bind Direction

The caller can supply data to the database (IN), the database can return data
to the caller (OUT) or the caller can supply initial data to the database and
the database can supply the modified data back to the caller (IN/OUT). This is
known as the bind direction.

The examples shown above have all supplied data to the database and are
classified as IN bind variables.

## <a name="bindnull"></a> 5.4 Binding Null Values

To insert a NULL into a character column you can use `Option<T>`. The type `T`
is important because a NULL value has no type by itself. The type
tells rust-oracledb which database type to bind.

For example, with the table:

```sql
create table tab (id number, val varchar2(50));
```

You can use:

``` rust
let val: Option<String> = None;

connection.execute(
    "insert into tab (id, val) values (:1, :2)",
    &[&280, &val]
)?;
```

## <a name="bindrowid"></a> 5.5 Binding ROWID Values

The pseudo-column ROWID uniquely identifies a row in a table. In rust-oracledb,
ROWID values are represented as strings.

## <a name="bindurowid"></a> 5.6 Binding UROWID Values

Universal rowids (UROWID) are used to uniquely identify rows in index
organized tables. In rust-oracledb, UROWID values are represented as
strings.

## <a name="dml-returning-bind"></a> 5.7 DML RETURNING Bind Variables

When a RETURNING clause is used with a DML statement like UPDATE, INSERT, or
DELETE, the values are returned to the application through the use of OUT bind
variables. In rust-oracledb, returned values are read from
[`ExecResult::returned_data()`](crate::ExecResult::returned_data).

Since a DML statement can affect multiple rows, a RETURNING INTO value is
fetched as a `Vec<T>`.

Consider the following example:

```rust
let mut result = connection.execute_named(
    r#"
    update departments set
        location_id = :loc_id
    where department_id = :dept_id
    returning department_name into :dept_name
    "#,
    &[
        ("loc_id", &1700),
        ("dept_id", &50),
        ("dept_name", &" ".repeat(100)),
    ],
)?;

let returned_data = result.returned_data();
let dept_names: Vec<String> = returned_data[0].get(0)?;

println!("{dept_names:?}"); // will print ["Shipping"]
```

In the above example, " ".repeat(100) is the type and size hint for the
returned string value. Since the WHERE clause matches one row, the vector
contains one item. If multiple rows were updated, the vector would contain
one item for each updated row.

## <a name="multiplevalueswherein"></a> 5.8 Binding Multiple Values to a SQL WHERE IN Clause

To bind multiple values in a SQL WHERE IN clause, create one bind placeholder
for each value. A Rust `Vec<T>` cannot be bound directly to a single placeholder
as an IN list.

For example, to use two values in an IN clause your code should be like:

```rust
let items = ["Smith", "Taylor"];

let cursor = connection.query(
    r#"
    select employee_id, first_name, last_name
    from employees
    where last_name in (:1, :2)
    "#,
    &[&items[0], &items[1]],
)?;

for row_result in cursor {
    let row = row_result?;

    let employee_id: i32 = row.get(0)?;
    let first_name: String = row.get(1)?;
    let last_name: String = row.get(2)?;

    println!("{employee_id}, {first_name}, {last_name}");
}
```

This gives the output:

```text
159: Lindsey Smith
171: William Smith
176: Jonathon Taylor
180: Winston Taylor
```

If the query is executed multiple times with different numbers of values,
include one bind variable placeholder in the SQL statement for each of the
maximum possible number of values. If a particular execution has fewer values,
bind *None* for the missing values.

Use *Some(value)* for non-null values and *None* for null values. This makes
each bind value an `Option<T>`, allowing Rust to know the type of NULL values.
For example, if a query is used for up to five values, but only two values are
used in a particular execution, the code could be:

```rust
let items = [
    Some("Smith"),
    Some("Taylor"),
    None,
    None,
    None,
];

let cursor = connection.query(
    r#"
    select employee_id, first_name, last_name
    from employees
    where last_name in (:1, :2, :3, :4, :5)
    "#,
    &[
        &items[0],
        &items[1],
        &items[2],
        &items[3],
        &items[4],
    ],
)?;
```

This prints the following output:

```text
(159, Lindsey, Smith)
(171, William, Smith)
(176, Jonathon, Taylor)
```

Reusing the same SQL statement like this for a variable number of values,
instead of constructing a unique statement for each set of values, allows best
reuse of Oracle Database resources. In rust-oracledb, use `Option<T>` values
such as *Some(value)* and *None* for unused bind positions when a fixed maximum
number of bind placeholders is used.

If other bind variables are required in the statement, adjust the bind
variable placeholder numbers appropriately:

```rust
let employee_id = 120;

let last_names = [
    Some("Smith"),
    Some("Taylor"),
    None,
    None,
    None,
];
let cursor = connection.query(
    r#"
    select employee_id, first_name, last_name
    from employees
    where employee_id > :1
    and last_name in (:2, :3, :4, :5, :6)
    "#,
    &[
        &employee_id,
        &last_names[0],
        &last_names[1],
        &last_names[2],
        &last_names[3],
        &last_names[4],
    ],
)?;

for row_result in cursor {
    let row = row_result?;

    let employee_id: i32 = row.get(0)?;
    let first_name: String = row.get(1)?;
    let last_name: String = row.get(2)?;

    println!("({employee_id}, {first_name}, {last_name})");
}
```

In the above example, the employee_id value is bound to :1. The five last_names
values are bound to :2 through :6. Use *Some(value)* for names you want to search
for, and *None* for unused bind positions.

### <a name="bindinlist"></a> 5.8.1 Binding a Large Number of Items in an IN List

The number of items in an IN list is limited to 65535 in Oracle Database
version 26, and to 1000 in earlier versions. If you exceed the limit, the
database will return an error such as:

```text
ORA-01795: maximum number of expressions in a list is 65535.
```

To use more values in the IN clause list, you can add OR clauses like:

```rust
let sql = r#"
    select ...
    from ...
    where key in (:1, :2, :3, :4, :5)
       or key in (:6, :7, :8, :9, :10)
       or key in (:11, :12, :13, :14, :15)
"#;
```

A more general solution for a larger number of values is to construct a
SQL statement like:

```sql
SELECT ... WHERE col IN ( <something that returns a list of values> )
```

The best way to do the `<something that returns a list of values>` depends on
how the data is initially represented and the number of items. For example, you
might look at using a global temporary table.

## <a name="bindcoltblnames"></a> 5.9 Binding Column and Table Names

Table names cannot be bound in SQL queries. You can concatenate text to
build up a SQL statement, but ensure that you use an Allow List or other
means to validate the data in order to avoid SQL Injection security
issues.

Binding column names can be done either by using an allow list, or by using a
CASE statement. The example below demonstrates binding a column
name in an ORDER BY clause:

```rust
let sql = r#"
    select department_id, department_name, manager_id
    from departments
    order by
        case :1
            when 'DEPARTMENT_ID' then department_id
            else manager_id
        end
"#;

let col_name = get_column_name(); //Obtain a column name from the user

let cursor = connection.query(sql, &[&col_name])?;

for row_result in cursor {
    let row = row_result?;

    let department_id: i32 = row.get(0)?;
    let department_name: String = row.get(1)?;
    let manager_id: Option<i32> = row.get(2)?;

    println!("({department_id}, {department_name}, {manager_id:?})");
}
```

Depending on the name provided by the user, the query results will be
ordered either by the column DEPARTMENT_ID or the column MANAGER_ID.

See [Dynamic SQL Construction and Validation](#validatingsql).
