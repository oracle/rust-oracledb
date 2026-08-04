# <a name="jsondatatype"></a> 10. Using JSON Data

JSON data can be used with relational database features, including
transactions, indexing, declarative querying, and views. You can project JSON
data relationally, making it available for relational processes and
tools. [JSON-Relational Duality Views](#jsondualityviews) provide the
benefits of the relational model and SQL access, while also allowing read and
write access to data as JSON documents.

Support for JSON was introduced in Oracle Database 12c. For more information
about using JSON in Oracle Database see the [Database JSON Developer's Guide].

Oracle Database 21c introduced a dedicated JSON data type with a new
[binary storage format OSON] that improves performance and functionality
compared with earlier releases.

To create a table with a column called JSON_DATA for JSON data, for example:

```sql
create table json_tab (
    id number primary key,
    json_data json
);
```

## <a name="insertjsondatatype"></a> 10.1 Inserting Oracle Database JSON Type


With Oracle Database 21c (or later), when using rust-oracledb, you can use
[oracledb::JsonValue](crate::JsonValue) to bind JSON data:

```rust
// Import HashMap to build a JSON object
use std::collections::HashMap;

// Create an empty JSON object map
let mut obj = HashMap::new();

// Add a new key-value pair to the JSON object
obj.insert(
    "department_name".to_string(), // Define the JSON field name
    oracledb::JsonValue::String("Sales".to_string()), // Define the JSON string value
);

// Add another key-value pair
obj.insert(
    "location".to_string(), // Define the JSON field name
    oracledb::JsonValue::String("London".to_string()), // Define the JSON boolean value
);

// Convert the map into a JsonValue object
let json = oracledb::JsonValue::JsonObject(obj);
// Define the primary key value for the row
let id = 1;

// Execute an INSERT statement
connection.execute(
    "insert into json_tab (id, json_data) values (:1, :2)",
    &[&id, &json],
)?;

// Commit the transaction
connection.commit()?;
```

## <a name="fetchjsondatatype"></a> 10.2 Fetching Oracle Database JSON Type

Fetching a JSON column returns an [oracledb::JsonValue](crate::JsonValue).
This enum represents the JSON document as Rust values. JSON objects are returned
as `JsonValue::JsonObject`, JSON arrays as `JsonValue::JsonArray`, and scalar
values as variants such as `JsonValue::String`, `JsonValue::Boolean`,
`JsonValue::Number`, and `JsonValue::Null`.

```rust
let row = connection.query_row(
    "select json_data from json_tab where id = :1",
    &[&id],
)?;

let json_data: oracledb::JsonValue = row.get(0)?;

println!("{json_data:?}");
```

This gives:

```text
JsonObject(
    {
        "department_name": String(
            "Sales",
        ),
        "location": String(
            "London",
        ),
    },
)
```

## <a name="inbindtypemapping"></a> 10.3 IN Bind Type Mapping

When binding JSON data, use [oracledb::JsonValue](crate::JsonValue). The
`JsonValue` variant determines the JSON value or Oracle extended JSON scalar
type that is sent to the database.

| Rust `JsonValue` variant | JSON Attribute Type or Value | SQL Equivalent Example |
| --- | --- | --- |
| `JsonValue::Null` | `null` | `NULL`  |
| `JsonValue::Boolean(true)` | `true`  | `json_scalar(true)` |
| `JsonValue::Boolean(false)`| `false` | `json_scalar(false)` |
| `JsonValue::Number(OracleNumber)` | `NUMBER` | `json_scalar(1)` |
| `JsonValue::String(String)` | `VARCHAR2` | `json_scalar('String')` |
| `JsonValue::Timestamp(OracleTimestamp)` | `TIMESTAMP` | `json_scalar(to_timestamp('2020-03-10', 'YYYY-MM-DD'))` |
| `JsonValue::Raw(Vec<u8>)` | `RAW` | `json_scalar(utl_raw.cast_to_raw('A raw value'))` |
| `JsonValue::JsonArray(Vec<JsonValue>)` | Array | `json_array(1, 2, 3 returning json)` |
| `JsonValue::JsonObject(HashMap<String, JsonValue>)` | Object | `json_object(key 'Fred' value json_scalar(5) returning json)` |
| `JsonValue::IntervalYM(OracleIntervalYM)` | `INTERVAL YEAR TO MONTH` | `json_scalar(to_yminterval('+5-9'))` |
| `JsonValue::IntervalDS(OracleIntervalDS)` | `INTERVAL DAY TO SECOND` | `json_scalar(to_dsinterval('P25DT8H25M'))` |
| `JsonValue::BinaryDouble(f64)` | `BINARY_DOUBLE` | `json_scalar(to_binary_double(25))` |
| `JsonValue::BinaryFloat(f32)` | `BINARY_FLOAT` | `json_scalar(to_binary_float(15.5))` |
| `JsonValue::Vector(Vector)` | `VECTOR` | `json_object('vector' value vector('[1,2]', 2, float32) returning json)` |

## <a name="outbindtypemapping"></a> 10.4 Query and OUT Bind Type Mapping

When Oracle Database JSON values are fetched, rust-oracledb returns them as
[JsonValue](crate::JsonValue). The JSON attribute type determines which
`JsonValue` variant is used. JSON objects and arrays are returned as
`JsonValue::JsonObject` and `JsonValue::JsonArray`; scalar JSON values are
returned as the corresponding `JsonValue` variants.

| Database JSON Attribute Type or Value | Rust `JsonValue` variant |
| --- | --- |
| `null` | `JsonValue::Null` |
| `false` | `JsonValue::Boolean(false)` |
| `true` | `JsonValue::Boolean(true)` |
| `NUMBER` | `JsonValue::Number(OracleNumber)` |
| `VARCHAR2` | `JsonValue::String(String)` |
| `RAW` | `JsonValue::Raw(Vec<u8>)` |
| `DATE` | `JsonValue::Timestamp(OracleTimestamp)` |
| `TIMESTAMP` | `JsonValue::Timestamp(OracleTimestamp)` |
| `INTERVAL YEAR TO MONTH` | `JsonValue::IntervalYM(OracleIntervalYM)` |
| `INTERVAL DAY TO SECOND` | `JsonValue::IntervalDS(OracleIntervalDS)` |
| `BINARY_DOUBLE` | `JsonValue::BinaryDouble(f64)` |
| `BINARY_FLOAT` | `JsonValue::BinaryFloat(f32)` |
| Arrays | `JsonValue::JsonArray(Vec<JsonValue>)` |
| Objects | `JsonValue::JsonObject(HashMap<String, JsonValue>)` |
| `VECTOR` | `JsonValue::Vector(Vector)` |

## <a name="pathexpr"></a> 10.5 SQL/JSON Path Expressions

Oracle Database provides SQL access to JSON data using SQL/JSON path
expressions. A path expression selects zero or more JSON values that match, or
satisfy, it. Path expressions can use wildcards and array ranges. A simple path
expression is `$.friends` which is the value of the JSON field `friends`.

For example, the previously created table with JSON column JSON_DATA can be
queried using:

``` sql
select j.json_data.department_name from json_tab j where j.id=1;
```

The queried value would be `Sales`.

The JSON_EXISTS functions tests for the existence of a particular value within
some JSON data. To look for JSON entries that have a `department_name` field:

```rust
let row = connection.query_row(
    "select count(*)
     from json_tab
     where json_exists(json_data, '$.department_name')",
    &[],
)?;

let count: i32 = row.get(0)?;

println!("rows with department_name: {count}");
```

This query displays:

```text
rows with department_name: 1
```

The SQL/JSON functions `JSON_VALUE` and `JSON_QUERY` can also be used.

Note that the default error-handling behavior for these functions is
`NULL ON ERROR`, which means that no value is returned if an error occurs. To
ensure that an error is raised, use `ERROR ON ERROR`.

For more information, see [SQL/JSON Path Expressions] in the Oracle JSON
Developer's Guide.

## <a name="accessrelationaldata"></a> 10.6 Accessing Relational Data as JSON

Oracle Database 12.2, or later, can convert relational data to JSON by using
SQL JSON generation functions such as [JSON_OBJECT] and [JSON_ARRAYAGG].
rust-oracledb executes these SQL statements normally, and the returned JSON can
be fetched as [JsonValue](crate::JsonValue) when the SQL expression returns
Oracle JSON data.

An example of using [JSON_OBJECT] is shown below:

```rust
let cursor = connection.query(
    r#"
    select json_object(
        'deptId' value d.department_id,
        'name' value d.department_name
        returning json
    ) department
    from departments d
    where department_id in (:1, :2, :3, :4)
    order by d.department_id
    "#,
    &[&10, &20, &30, &40],
)?;

for row_result in cursor {
    let row = row_result?;
    let department: oracledb::JsonValue = row.get(0)?;
    println!("{department:?}");
}
```

This displays the following output:

```text
JsonObject({"deptId": Number(10), "name": String("Administration")})
JsonObject({"name": String("Human Resources"), "deptId": Number(40)})
JsonObject({"deptId": Number(20), "name": String("Marketing")})
JsonObject({"name": String("Purchasing"), "deptId": Number(30)})
```

To select a result set from a relational query as a single object you can use
[JSON_ARRAYAGG], for example:

```rust
let row = connection.query_row_named(
    r#"
    select json_arrayagg(
               json_object(
                   'deptid' value d.department_id,
                   'name' value d.department_name
               )
               returning clob
           )
    from departments d
    where department_id in (:did1, :did2, :did3, :did4)
    "#,
    &[("did1", &10), ("did2", &20), ("did3", &30), ("did4", &40)],
)?;

let j: String = row.get(0)?;

println!("{j}");
```

This displays the following output:

```text
[{"deptid":10,"name":"Administration"},{"deptid":20,"name":"Marketing"},{"deptid":30,"name":"Purchasing"},{"deptid":40,"name":"Human Resources"}]
```

## <a name="jsondualityviews"></a> 10.7 JSON-Relational Duality Views

Oracle AI Database 26ai JSON-Relational Duality Views allow data to be stored
as rows in tables to provide the benefits of the relational model and SQL
access, while also allowing read and write access to data as JSON documents for
application simplicity. See the [JSON-Relational Duality Developer's Guide] for
more information.

For example, if the tables `AuthorTab` and `BookTab` exist:


```sql
create table AuthorTab (
    AuthorId number generated by default on null as identity primary key,
    AuthorName varchar2(100)
);

create table BookTab (
    BookId number generated by default on null as identity primary key,
    BookTitle varchar2(100),
    AuthorId number references AuthorTab (AuthorId)
);
```

Then a JSON Duality View over the tables could be created in SQL*Plus:

```sql
create or replace json relational duality view BookDV as
BookTab @insert @update @delete
{
    _id: BookId,
    book_title: BookTitle,
    author: AuthorTab @insert @update
    {
        author_id: AuthorId,
        author_name: AuthorName
    }
};
```

Applications can choose whether to use relational access to the underlying
tables, or use the duality view.

You can use SQL/JSON to query the view and return JSON. The query uses the
special column DATA:

```rust
use std::collections::HashMap;

use oracledb::{JsonValue, OracleNumber}

// Query the JSON duality view
let cursor = connection.query(
    r#"
    select b.data.book_title, b.data.author.author_name
    from BookDV b
    where b.data.author.author_id = :1
    "#,
    &[&1],
)?;

for row_result in cursor {
    let row = row_result?;

    let book_title: String = row.get(0)?;
    let author_name: String = row.get(1)?;

    println!("{book_title}, {author_name}");
}
```

Inserting JSON into the view will update the base relational tables:

```rust
let mut author = HashMap::new();

author.insert(
    "author_id".to_string(),
    JsonValue::Number(OracleNumber::from(2000)),
);

author.insert(
    "author_name".to_string(),
    JsonValue::String("John Doe".to_string()),
);

let mut data = HashMap::new();

data.insert(
    "_id".to_string(),
    JsonValue::Number(OracleNumber::from(1000)),
);

data.insert(
    "book_title".to_string(),
    JsonValue::String("My New Book".to_string()),
);

data.insert(
    "author".to_string(),
    JsonValue::JsonObject(author),
);

let json_doc = JsonValue::JsonObject(data);

connection.execute(
    "insert into BookDV values (:1)",
    &[&json_doc],
)?;

connection.commit()?;
```

[Database JSON Developer's Guide]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=ADJSN
[JSON-Relational Duality Developer's Guide]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=JSNVU
[binary storage format OSON]: https://blogs.oracle.com/jsondb/osonformat
[JSON_ARRAYAGG]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-6D56077D-78DE-4CC0-9498-225DDC42E054
[JSON_OBJECT]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-1EF347AE-7FDA-4B41-AFE0-DD5A49E8B370
[SQL/JSON Path Expressions]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-2DC05D71-3D62-4A14-855F-76E054032494
