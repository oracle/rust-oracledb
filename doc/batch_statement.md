# <a name="batchstmnt"></a> 6. Executing Batch Statements and Bulk Loading

Rust-oracledb is perfect for large ETL ("Extract, Transform, Load") data
operations.

This chapter focuses on efficient data ingestion. Rust-oracledb lets you
easily optimize batch insertion, and also allows "noisy" data (values not in a
suitable format) to be filtered for review while other, correct, values are
inserted.

## <a name="batchstmntexec"></a> 6.1 Batch Statement Execution

Inserting, updating or deleting multiple rows can be performed efficiently with
[`Connection::execute_batch()`](crate::Connection::execute_batch), making it
easy to work with large data sets with rust-oracledb. This method can
significantly outperform repeated calls to
[`Connection::execute()`](crate::Connection::execute) by reducing network
transfer costs and database overheads. The
[`Connection::execute_batch()`](crate::Connection::execute_batch) method can
also be used to execute a PL/SQL statement multiple times in one call.

The following tables will be used in the samples that follow:

```sql
create table ParentTable (
    ParentId              number(9) not null,
    Description           varchar2(60) not null,
    constraint ParentTable_pk primary key (ParentId)
);

create table ChildTable (
    ChildId               number(9) not null,
    ParentId              number(9) not null,
    Description           varchar2(60) not null,
    constraint ChildTable_pk primary key (ChildId),
    constraint ChildTable_fk foreign key (ParentId)
            references ParentTable
);
```

### <a name="batchstmntexecsql"></a> 6.1.1 Batch Execution of SQL

The following example inserts five rows into the table ParentTable:

```rust
connection.execute_batch(
    "insert into ParentTable values (:1, :2)",
    &[
        &[&10, &"Parent 10"],
        &[&20, &"Parent 20"],
        &[&30, &"Parent 30"],
        &[&40, &"Parent 40"],
        &[&50, &"Parent 50"],
    ],
)?;
```

Each inner slice contains the bind values for one row.

This code requires only one [round-trip](#roundtrips) from the client to the
database instead of the five round-trips that would be required for repeated
calls to [`Connection::execute()`](crate::Connection::execute).

To insert a single column with
[`Connection::execute_batch()`](crate::Connection::execute_batch), pass one
inner slice for each execution. Each inner slice contains the bind values for
that execution.

```rust
connection.execute_batch(
    "insert into mytable (mycol) values (:1)",
    &[
        &[&10],
        &[&20],
        &[&30],
    ],
)?;
```

### <a name="batchplsql"></a> 6.1.2 Batch Execution of PL/SQL

Using [`Connection::execute_batch()`](crate::Connection::execute_batch) can
improve performance when the same PL/SQL block needs to be executed multiple
times with different bind values. The method returns an
[`ExecBatchResult`](crate::ExecBatchResult), which provides the total number of
affected rows and any output bind values through
`ExecBatchResult::out_bind_data()`.

**IN Binds**

An example using [bind by position](#bindbyposition) for IN binds is:

```rust
connection.execute_batch(
    "begin mypkg.create_parent(:1, :2); end;",
    &[
        &[&10, &"Parent 10"],
        &[&20, &"Parent 20"],
        &[&30, &"Parent 30"],
        &[&40, &"Parent 40"],
        &[&50, &"Parent 50"],
    ],
)?;
```

**OUT Binds**

PL/SQL OUT bind variables are supported. Applications do not set the bind
direction explicitly. During execution, rust-oracledb reads the bind direction
reported by Oracle Database; any bind reported as non-input is returned through
`ExecBatchResult::out_bind_data()`.

For batch execution, `out_bind_data()` returns a `Vec<Row>`. Each row contains
the output bind values for one execution, in the same order as the input bind
values.

For an OUT parameter, pass a value of the desired Rust type as a placeholder.
The placeholder is used to determine the bind type and buffer metadata.

```sql
create or replace procedure myproc(p1 in number, p2 out number) as
begin
    p2 := p1 * 2;
end;
```

This can be called in rust-oracledb using positional binds like:

```rust
for p1 in [100, 200, 300] {
    let mut result = connection.execute(
        "begin myproc(:1, :2); end;",
        &[&p1, &0],
    )?;

    let p2: i32 = result.out_bind_data().get(0)?;

    println!("{p2}");
}
```

This prints the following output:

```text
200
400
600
```

The equivalent code using named binds is:

```rust
let data = [100, 200, 300];

for p1 in data {
    let mut result = connection.execute_named(
        "begin myproc(:p1, :p2); end;",
        &[
            ("p1", &p1),
            ("p2", &0),
        ],
    )?;

    let p2: i32 = result.out_bind_data().get(0)?;

    println!("{p2}");
}
```

This prints the following output:

```text
200
400
600
```

When the same PL/SQL block is executed with multiple sets of bind values,
`Connection::execute_batch()` returns an
[`ExecBatchResult`](crate::ExecBatchResult). Output bind values can be read
with `ExecBatchResult::out_bind_data()`, which returns a `Vec<Row>` containing
one row for each execution.

```rust
let params = oracledb::BindParameters::Slice(&[
    &[&100, &0],
    &[&200, &0],
    &[&300, &0],
]);

let mut result = connection.execute_batch(
    "begin rso_examples_batch_proc(:1, :2); end;",
    params,
)?;

for row in result.out_bind_data() {
    let p2: i32 = row.get(0)?;
    println!("{p2}");
}
```

This prints the following output:

```text
200
400
600
```

**IN/OUT Binds**

PL/SQL IN/OUT bind variables are also supported. Pass the initial value as
the bind value. The modified value is not written back to the original Rust
variable; read it from `ExecResult::out_bind_data()` after execution.

``` sql
create or replace procedure myproc2 (p1 in number, p2 in out varchar2) as
begin
    p2 := p2 || ' ' || p1;
end;
```

This can be called in rust-oracledb using positional binds as shown in the
example below:

```rust
let data = [(440, "Gregory"), (550, "Haley"), (660, "Ian")];
let mut outvals = Vec::new();

for (p1, p2) in data {
    let mut result = connection.execute(
        "begin myproc2(:1, :2); end;",
         &[&p1, &p2],
    )?;

    let outval: String = result.out_bind_data().get(0)?;

    outvals.push(outval);
}

println!("{outvals:?}");
```

This prints the following output:

```text
["Gregory 440", "Haley 550", "Ian 660"]
```

The equivalent code using named binds is:

```rust
let data = [(440, "Gregory"), (550, "Haley"), (660, "Ian")];
let mut outvals = Vec::new();

for (p1, p2) in data {
    let mut result = connection.execute_named(
        "begin myproc2(:p1, :p2); end;",
        &[
            ("p1", &p1),
            ("p2", &p2),
        ],
    )?;

    let outval: String = result.out_bind_data().get(0)?;

    outvals.push(outval);
}

println!("{outvals:?}");
```

## <a name="identifyaffectedrows"></a> 6.2 Identifying Affected Rows

When executing a DML statement with
[Connection::execute()](crate::Connection::execute), the number of affected
rows can be examined with
[ExecResult::rows_affected()](crate::ExecResult::rows_affected).

When performing batch execution with
[Connection::execute_batch()](crate::Connection::execute_batch), the row count
returned by `rows_affected()` is the total number of rows affected by the batch:

```rust
let parent_ids_to_delete = [20, 30, 50];

for parent_id in parent_ids_to_delete {
    let result = connection.execute(
        "delete from ChildTable where ParentId = :1",
        &[&parent_id],
    )?;

    println!(
        "Parent ID: {} deleted {} rows.",
        parent_id,
        result.rows_affected()
    );
}
```

The output is:

```text
Parent ID: 20 deleted 3 rows.
Parent ID: 30 deleted 2 rows.
Parent ID: 50 deleted 4 rows.
```

## <a name="dmlreturning"></a> 6.3 DML RETURNING

DML statements like INSERT, UPDATE, DELETE, and MERGE can return values by
using the DML RETURNING syntax. A bind variable can be created to accept this
data. See [Using Bind Variables](#bind) for more information.

DML RETURNING values can be collected either with repeated
[Connection::execute()](crate::Connection::execute) calls or with
[Connection::execute_batch()](crate::Connection::execute_batch).

For a single execution, `ExecResult::returned_data()` returns a `Vec<Row>`.
For batch execution, `ExecBatchResult::returned_data()` returns a
`Vec<Vec<Row>>`, grouping returned rows by batch execution.

The following example uses repeated `Connection::execute()` calls. For each
execution, `ExecResult::returned_data()` returns a `Vec<Row>` containing the
rows returned by that DML statement. If, instead of merely deleting the rows
as shown in the previous example, you also wanted to know some information
about each of the rows that were deleted, you can use the following code:

```rust
let parent_ids_to_delete = [20, 30, 50];

for parent_id in parent_ids_to_delete {
    let mut result = connection.execute(
        "delete from ChildTable
         where ParentId = :1
         returning ChildId into :2",
        &[&parent_id, &0],
    )?;

    let rows = result.returned_data();

    if rows.is_empty() {
        println!(
            "Child IDs deleted for parent ID {} are []",
            parent_id
        );
    } else {
        let child_ids: Vec<i32> = rows
            .iter()
            .map(|r| r.get(0).unwrap())
            .collect();

        println!(
            "Child IDs deleted for parent ID {} are {:?}",
            parent_id,
            child_ids
        );
    }
}
```

The output will be:

```text
Child IDs deleted for parent ID 20 are [1, 2, 3]
Child IDs deleted for parent ID 30 are []
Child IDs deleted for parent ID 50 are [4, 5]
```

The "&0" bind is a type hint for the returned ChildId values. The actual
returned values are read from `result.returned_data()`.

DML RETURNING can also be used with
[Connection::execute_batch()](crate::Connection::execute_batch). The returned
values are available through
[`ExecBatchResult::returned_data()`](crate::ExecBatchResult::returned_data).
For batch execution, `returned_data()` returns a `Vec<Vec<Row>>`. The outer
vector contains one entry for each execution in the batch, in input order. Each
inner vector contains the rows returned by that execution and may be empty when
the execution affects no rows.

The following example uses `Connection::execute_batch()` with the same
`DELETE ... RETURNING` statement. `ExecBatchResult::returned_data()` returns
the returned rows grouped by batch execution.

```rust
let parent_ids = [20, 30, 50];

let params = oracledb::BindParameters::Slice(&[
    &[&20, &0],
    &[&30, &0],
    &[&50, &0],
]);

let mut result = connection.execute_batch(
    "delete from ChildTable
     where ParentId = :1
     returning ChildId into :2",
    params,
)?;

for (parent_id, rows) in parent_ids.into_iter().zip(result.returned_data()) {
    let child_ids: Vec<i32> = rows
        .into_iter()
        .map(|row| row.get(0))
        .collect::<Result<_, _>>()?;

    println!(
        "Child IDs deleted for parent ID {parent_id} are {child_ids:?}"
    );
}
```

The output is:

```text
Child IDs deleted for parent ID 20 are [1, 2, 3]
Child IDs deleted for parent ID 30 are []
Child IDs deleted for parent ID 50 are [4, 5]
```
