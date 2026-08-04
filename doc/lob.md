# <a name="lob"></a> 9. Using CLOB, NCLOB, and BLOB

Oracle Database uses LOB objects to store large data such as text, images,
videos, and other multimedia formats. The maximum size of a LOB (large object)
is limited to the size of the tablespace storing it.

The CLOB type is used for character data and the BLOB type is used for binary
data. NCLOB can hold character data in the database’s alternative national
character set.

Rust-oracledb uses [oracledb::DB_TYPE_CLOB](crate::db_type::DB_TYPE_CLOB),
[oracledb::DB_TYPE_NCLOB](crate::db_type::DB_TYPE_NCLOB), and
[oracledb::DB_TYPE_BLOB](crate::db_type::DB_TYPE_BLOB) to represent CLOB,
NCLOB, and BLOB database types respectively. LOB data can be fetched as
`oracledb::Lob` locators by using
[Statement::fetch_lobs()](crate::Statement::fetch_lobs), or as
native Rust values `String` for CLOB/NCLOB data and `Vec<u8>` for BLOB data.

## <a name="simplelobs"></a> 9.1 Simple Inserting and Querying of LOBs

Consider the following table with a CLOB column:

```sql
create table lob_tbl (id number, c clob);
```

With rust-oracledb, LOB data can be inserted as shown in the example below:

```rust
let text_data = fs::read_to_string("example.txt")?;
let lobid = 10;

connection.execute_named(
    r#"insert into lob_tbl (id, c) values (:lobid, :clobdata)"#,
    &[
        ("lobid", &lobid),
        ("clobdata", &text_data),
    ],
)?;
```

With rust-oracledb, LOB data can be fetched as shown in the example below:

```rust
let row = connection.query_row(
    "select c from lob_tbl where id = :1",
    &[&lobid],
)?;

let fetched_text: String = row.get(0)?;

println!("CLOB: {fetched_text}");
```

## <a name="loblocator"></a> 9.2 Inserting and Querying Using LOB Locator

A LOB locator is a handle to LOB data stored in the database. Instead of
returning the CLOB, NCLOB, or BLOB value directly as a `String` or `Vec<u8>`,
rust-oracledb can return an `oracledb::Lob` object that refers to the database
LOB.

You can use LOB locators when you want to stream data, inspect LOB metadata,
or modify an existing persistent LOB. Using `fetch_lobs()`, you can enable
locator fetching. Without `fetch_lobs()`, CLOB/NCLOB values are fetched
directly as `String`, and BLOB values are fetched directly as `Vec<u8>`.

Consider the following table:

```sql
create table lob_locator_tbl (id number, b blob);
```

You can insert using:

```rust
let id = 1;
let blob_data = vec![10_u8, 20, 30, 40, 255];

connection.execute(
    "insert into lob_locator_tbl (id, b) values (:1, :2)",
    &[&id, &blob_data],
)?;
```

You can fetch with a LOB locator as shown below:

```rust
let row = connection
    .statement("select b from lob_locator_tbl where id = :1")?
    .fetch_lobs()
    .query_row(&[&id])?;
let mut blob: oracledb::Lob = row.get(0)?;
let mut fetched_blob = Vec::new();
blob.read_to_end(&mut fetched_blob)?;
println!("BLOB bytes: {fetched_blob:?}");
```

This prints the following output:

```text
BLOB bytes: [10, 20, 30, 40, 255]
```
