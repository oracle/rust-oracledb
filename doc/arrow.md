# <a name="arrowdata"></a> 13. Using Apache Arrow Data

Rust-oracledb supports [Apache Arrow] for fetching query results and binding
batch DML data as Arrow RecordBatch values. Arrow support is useful for
applications that process data in column-oriented form, exchange data with
Arrow-based libraries, or perform bulk DML from existing Arrow data.

The examples in this section use the following table for Arrow data:

```sql
create table arrow_table (
    id number(9),
    name varchar2(30)
);
```

## <a name="insertingarrowdata"></a> 13.1 Inserting Arrow Data

Rust-oracledb can bind an Arrow RecordBatch for batch DML execution. This is
useful when data is already available in column-oriented Arrow format and needs
to be inserted into Oracle Database.

Pass the RecordBatch to
[Connection::execute_batch()](crate::Connection::execute_batch). Each Arrow
column maps to one bind position in the SQL statement, and each row in the
RecordBatch is one execution of the statement.

```rust
// The schema describes the Arrow columns. Each array below must match
// the corresponding field and have the same number of rows.
let batch = RecordBatch::try_new(
    Arc::new(Schema::new(vec![
        Field::new("ID", DataType::Int32, false),
        Field::new("NAME", DataType::Utf8, false),
    ])),
    vec![
        Arc::new(Int32Array::from(vec![1, 2])),
        Arc::new(StringArray::from(vec!["Anna", "John"])),
    ],
)?;

// Each Arrow column maps to a bind position, and each Arrow row is one
// execution of the INSERT statement.
let result = connection.execute_batch(
    "insert into arrow_table values (:1, :2)",
    batch,
)?;
```

The SQL statement should contain one bind variable for each column in the
RecordBatch. In the example above, column 0 is bound to :1 and column 1 is
bound to :2.

### <a name="arrowbindtypemapping"></a> 13.1.1 Arrow Bind Type Mapping

Rust-oracledb maps Arrow array types to Oracle Database bind types when
binding Arrow data.

When an Arrow RecordBatch is passed to
[Connection::execute_batch()](crate::Connection::execute_batch), each Arrow
column is bound using its Arrow data type.

The following table shows how Arrow Rust array data types are mapped to
oracledb database bind types when binding an Arrow RecordBatch. The first
column displays the Arrow Type and the second column displays the oracledb
Database bind type.

| Arrow Type        | oracledb Database Bind Type |
| ----------------- | --------------------------- |
| `Boolean` | `DB_TYPE_BOOLEAN` |
| `Int8`, `Int16`, `Int32`, `Int64` | `DB_TYPE_NUMBER` |
| `UInt8`, `UInt16`, `UInt32`, `UInt64` | `DB_TYPE_NUMBER` |
| `Decimal128` | `DB_TYPE_NUMBER` |
| `Null` | `DB_TYPE_NUMBER` |
| `Float16`, `Float32` | `DB_TYPE_BINARY_FLOAT` |
| `Float64` | `DB_TYPE_BINARY_DOUBLE` |
| `Date32`, `Date64`, `Timestamp` | `DB_TYPE_TIMESTAMP` |
| `Utf8`, `LargeUtf8` | `DB_TYPE_VARCHAR` |
| `Binary`, `LargeBinary`, `FixedSizeBinary` | `DB_TYPE_RAW` |

Null values in Arrow arrays are bound as database nulls. If an Arrow type is
not supported, [Connection.execute_batch()](crate::Connection::execute_batch)
returns an error.

## <a name="fetchingarrowdata"></a> 13.2 Fetching Arrow Data

Rust-oracledb can fetch SQL query results as Apache Arrow RecordBatch values.
This is useful for applications that process data in column-oriented form or
exchange query results with Arrow-based libraries.

To execute a query and return the rows as a single Arrow RecordBatch, use
[Connection::query_arrow()](crate::Connection::query_arrow):

```rust
use arrow_array::{Int32Array, StringArray};

let batch = connection.query_arrow(
    "select id, name from arrow_table order by id",
    oracledb::BindParameters::default(),
)?;

println!("Fetched {} rows as Arrow", batch.num_rows());

let ids = batch
    .column(0)
    .as_any()
    .downcast_ref::<Int32Array>()
    .unwrap();

let names = batch
    .column(1)
    .as_any()
    .downcast_ref::<StringArray>()
    .unwrap();

for row_index in 0..batch.num_rows() {
    println!(
        "id={}, name={}",
        ids.value(row_index),
        names.value(row_index)
    );
}
```

This prints the following output:

```text
Fetched 2 rows as Arrow
id=1, name=Anna
id=2, name=John
```

To control the output types or schema metadata, execute the query through a
`StatementBuilder` and call
[StatementBuilder::with_requested_schema()](crate::StatementBuilder::with_requested_schema).
See [Requesting an Arrow Schema](#arrowrequestedschema).

### <a name="arrowfetchtypemapping"></a> 13.2.1 Arrow Fetch Type Mapping

Rust-oracledb maps Oracle Database column types to Arrow array types when
fetching query results as Arrow data.

When query results are fetched with `Connection.query_arrow()`, rust-oracledb
derives the Arrow schema from the query result metadata.

The following table shows how oracledb database types are mapped to Apache
Arrow Rust array data types when fetching query results as an Arrow
RecordBatch. The first column displays the oracledb Database type and the
second column displays the Arrow type.

| oracledb Database Type | Arrow Type |
| ---------------------- | ---------- |
| `DB_TYPE_CHAR`, `DB_TYPE_VARCHAR`, `DB_TYPE_NCHAR`, and `DB_TYPE_NVARCHAR` | `Utf8` |
| `DB_TYPE_CLOB`, `DB_TYPE_NCLOB` and `DB_TYPE_LONG` | `LargeUtf8` |
| `DB_TYPE_RAW` | `Binary` |
| `DB_TYPE_BLOB` and `DB_TYPE_LONG_RAW` | `LargeBinary` |
| `DB_TYPE_DATE`, `DB_TYPE_TIMESTAMP`, `DB_TYPE_TIMESTAMP_TZ`, and `DB_TYPE_TIMESTAMP_LTZ` | `Timestamp(unit, None)` |
| `DB_TYPE_BINARY_FLOAT` | `Float32` |
| `DB_TYPE_BINARY_DOUBLE` | `Float64` |
| `DB_TYPE_BOOLEAN` | `Boolean` |
| `DB_TYPE_NUMBER`, precision 1 to 2, scale 0 | `Int8` |
| `DB_TYPE_NUMBER`, precision 3 to 4, scale 0 | `Int16` |
| `DB_TYPE_NUMBER`, precision 5 to 9, scale 0 | `Int32` |
| `DB_TYPE_NUMBER`, precision 10 to 18, scale 0 | `Int64` |
| `DB_TYPE_NUMBER` with nonzero scale, or precision greater than 18 | `Decimal128` |
| `DB_TYPE_NUMBER` with precision 0, or scale whose absolute value is greater than precision | `Float64` |

`DATE` values use seconds. `TIMESTAMP` values use seconds for scale 0,
milliseconds for scales 1 to 3, microseconds for scales 4 to 6, and nanoseconds
for scales 7 to 9.

For Arrow decimal values, Arrow requires a fixed precision and scale. If an
Oracle `NUMBER` column does not have fixed precision, or if its scale cannot be
represented as an Arrow decimal, rust-oracledb fetches the value as `Float64`.

If a query column has an Oracle Database type that cannot be mapped to an
Arrow type, `query_arrow()` returns an error.

### <a name="arrowrequestedschema"></a> 13.2.2 Requesting an Arrow Schema

By default, [Connection::query_arrow()](crate::Connection::query_arrow) derives
the Arrow schema and data types from Oracle Database metadata. To choose the
Arrow data type for a query result, use
[StatementBuilder::with_requested_schema()](crate::StatementBuilder::with_requested_schema).
For example:

```rust
use std::sync::Arc;
use arrow_schema::{DataType, Field, Schema};

// Fields correspond to SELECT columns by position
let schema = Arc::new(Schema::new(vec![
    Field::new("ID", DataType::Int64, true),
    Field::new("NAME", DataType::LargeUtf8, true),
]));

let batch = connection
    .statement("select id, name from arrow_table order by id")?
    .with_requested_schema(schema)
    .build()?
    .query_arrow(oracledb::BindParameters::default())?;
```

Fields map to query columns by position. The requested schema must contain
one field for each result column. The returned `RecordBatch` uses the requested
schema's field names, nullability, and metadata. Fields that can contain null
values should be marked as nullable.

The following requested conversions are supported:

| Oracle Database Type | Requested Arrow Type |
| -------------------- | -------------------- |
| `NUMBER` | `Int8`, `Int16`, `Int32`, `Int64` |
| `NUMBER` | `UInt8`, `UInt16`, `UInt32`, `UInt64` |
| `NUMBER` | `Float32`, `Float64` |
| `NUMBER` | `Decimal128(precision, scale)` |
| `NUMBER` | `Utf8`, `LargeUtf8` |
| Character types, including CLOB, NCLOB, LONG, and ROWID types | `Utf8`, `LargeUtf8` |
| RAW, LONG RAW, and BLOB | `Binary`, `LargeBinary` |
| DATE and TIMESTAMP types | `Timestamp` with seconds, milliseconds, microseconds, or nanoseconds |
| `BINARY_FLOAT` | `Float32` |
| `BINARY_DOUBLE` | `Float64` |
| `BOOLEAN` | `Boolean` |

Values must fit the requested type. Integer conversions require whole numbers
within the target integer range. Decimal128 conversions must preserve the
value exactly and fit the requested precision and scale. Timestamp units
coarser than the stored precision discard fractional digits. Timestamp
timezone metadata does not perform a timezone conversion.

An incompatible type, invalid decimal definition, or column count mismatch
returns an error. Without a requested schema, the default fetch mappings
continue to apply.

[Apache Arrow]: https://docs.rs/crate/arrow/latest
