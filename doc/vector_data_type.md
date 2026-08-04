# <a name="vectors"></a> 12. Using VECTOR Data

Oracle AI Database 26ai introduced a new data type [VECTOR] for artificial
intelligence and machine learning search operations. For more information about
using vectors in Oracle Database, see the
[Oracle AI Vector Search User's Guide].

In rust-oracledb, VECTOR values are represented with [Vector](crate::Vector).
With the VECTOR data type, you define the number of dimensions and the storage
format for each dimension value in the vector.

Vectors can be dense or sparse. A dense vector stores a value for every
dimension and is created with `Vector::Dense`. A sparse vector stores only the
non-zero values and their dimension indexes. It is created with
`Vector::Sparse` and `SparseVector`.

With the VECTOR data type, you define the number of dimensions and the storage
format for each dimension value in the vector. In rust-oracledb, VECTOR values
are represented with [Vector](crate::Vector) and
[VectorData](crate::VectorData).

The supported Rust variants are:

- `VectorData::Int8(Vec<i8>)` for `int8`
- `VectorData::Binary(Vec<u8>)` for `binary`
- `VectorData::Float32(Vec<f32>)` for `float32`
- `VectorData::Float64(Vec<f64>)` for `float64`

Vectors can also be defined with an arbitrary number of dimensions and
formats. This allows you to specify vectors of different dimensions with
the various storage formats mentioned above. For example:

```sql
CREATE TABLE vector_table (
    vec_data vector
)
```

## <a name="intfloatformat"></a> 12.1 Using FLOAT32, FLOAT64, and INT8 Vectors

To create a table with three columns for vector data:

``` sql
CREATE TABLE vector_table (
    v32 vector(3, float32),
    v64 vector(3, float64),
    v8  vector(3, int8)
)
```

In this example, each column can store vector data of three dimensions
where each dimension value is of the specified format.

### <a name="insertintfloatformat"></a> 12.1.1 Inserting FLOAT32, FLOAT64, and INT8 Vectors

With rust-oracledb, vector data can be inserted using
[`Vector`](crate::Vector) values. Dense vector data is created with
[`Vector::Dense`](crate::Vector::Dense) and one of the
[`VectorData`](crate::VectorData) variants such as `VectorData::Float32` for
`float32`, `VectorData::Float64` for `float64`, and `VectorData::Int8` for
`int8` vector columns. For example:

```rust
use oracledb::{Vector, VectorData};

let vec_data_32 = Vector::Dense(VectorData::Float32(vec![1.625, 1.5, 1.0]));
let vec_data_64 = Vector::Dense(VectorData::Float64(vec![11.25, 11.75, 11.5]));
let vec_data_8 = Vector::Dense(VectorData::Int8(vec![1, 2, 3]));

connection.execute(
    "insert into vector_table (v32, v64, v8) values (:1, :2, :3)",
    &[&vec_data_32, &vec_data_64, &vec_data_8],
)?;
```

### <a name="fetchintfloatformat"></a> 12.1.2 Fetching FLOAT32, FLOAT64, and INT8 Vectors

With rust-oracledb, vector columns of int8, float32, and float64
format are fetched as `oracledb::Vector` with their `VectorData` variant. For
example:

```rust
let row = connection.query_row(
    "select * from vector_table",
    &[],
)?;

let fetched_v32: Vector = row.get(0)?;
let fetched_v64: Vector = row.get(1)?;
let fetched_v8: Vector = row.get(2)?;

println!("v32 = {fetched_v32:?}");
println!("v64 = {fetched_v64:?}");
println!("v8 = {fetched_v8:?}");
```

This prints the following output:

```text
v32 = Dense(Float32([1.625, 1.5, 1.0]))
v64 = Dense(Float64([11.25, 11.75, 11.5]))
v8 = Dense(Int8([1, 2, 3]))
```

## <a name="binaryformat"></a> 12.2 Using BINARY Vectors

A Binary vector format represents each dimension value as a binary value (0 or
1). Binary vectors require less memory storage. For example, a 16 dimensional
vector with binary format requires only 2 bytes of storage while a 16
dimensional vector with int8 format requires 16 bytes of storage.

Binary vectors are represented as 8-bit unsigned integers. For the binary
format, you must define the number of dimensions as a multiple of 8.

To create a table with one column for vector data:

``` sql
CREATE TABLE vector_binary_table (
    vb vector(24, binary)
)
```

In this example, the VB column can store vector data of 24 dimensions where
each dimension value is represented as a single bit. Note that the number of
dimensions 24 is a multiple of 8.

If you specify a vector dimension that is not a multiple of 8, then you
will get `ORA-51813`.

### <a name="insertbinaryvector"></a> 12.2.1 Inserting BINARY Vectors

For binary vectors, rust-oracledb uses `VectorData::Binary(Vec<u8>)`. The
`Vec<u8>` contains packed binary vector data. Its length must be equal to the
number of vector dimensions divided by 8. For example, if the vector column has
24 dimensions, the `Vec<u8>` must contain 3 bytes. Each byte can range from 0
to 255. For example:

```rust
use oracledb::{Vector, VectorData};

let vector_data_vb = Vector::Dense(VectorData::Binary(vec![180, 150, 100,]));

connection.execute(
    "insert into vector_binary_table values (:1)",
    &[&vector_data_vb],
)?;
```

### <a name="fetchbinaryvector"></a> 12.2.2 Fetching BINARY Vectors

With rust-oracledb, vector columns of binary format are fetched as
`oracledb::Vector`. For example:

```rust
let cursor = connection.query(
    "select * from vector_binary_table",
    &[],
)?;

for row_result in cursor {
    let row = row_result?;
    let vector: oracledb::Vector = row.get(0)?;

    println!("{vector:?}");
}
```

This prints an output such as:

```text
Dense(Binary([180, 150, 100]))
```

## <a name="sparsevectors"></a> 12.3 Using SPARSE Vectors

A sparse vector is a vector in which most of its dimensions have a value of
zero. This vector only physically stores the non-zero values. For more
information on sparse vectors, see the
[Oracle AI Vector search User'sGuide][sparse vector].

Sparse vectors are represented by the total number of vector dimensions, an
array of indices, and an array of values where each value's location in the
vector is indicated by the corresponding indices array position. All other
vector values are treated as zero. The storage formats that can be used with
sparse vectors are float32, float64, and int8. Note that the binary storage
format cannot be used with sparse vectors.

For example, a string representation could be:

```text
[25, [5, 8, 11], [25.25, 6.125, 8.25]]
```

In this example, the sparse vector has 25 dimensions. Only indices 5, 8, and
11 have values which are 25.25, 6.125, and 8.25 respectively. All of the other
values are zero.

In Oracle AI Database, you can define a column for a sparse vector using the
following format:

```text
VECTOR(number_of_dimensions, dimension_storage_format, sparse)
```

For example, to create a table with three columns for sparse vectors:

``` sql
CREATE TABLE vector_sparse_table (
    float32sparsecol vector(25, float32, sparse),
    float64sparsecol vector(30, float64, sparse),
    int8sparsecol vector(35, int8, sparse)
)
```

In this example:

- The float32sparsecol column can store sparse vector data of 25 dimensions
  where each dimension value is a 32-bit floating-point number.
- The float64sparsecol column can store sparse vector data of 30 dimensions
  where each dimension value is a 64-bit floating-point number.
- The int8sparsecol column can store sparse vector data of 35 dimensions where
  each dimension value is a 8-bit signed integer.

### <a name="insertsparsevectors"></a> 12.3.1 Inserting SPARSE Vectors

With rust-oracledb, sparse vector data can be inserted using
[`SparseVector`](crate::SparseVector) values wrapped in
[`Vector::Sparse`](crate::Vector::Sparse). The same `Vector::Sparse` values are
used when fetching sparse VECTOR columns and as bind values when inserting into
sparse VECTOR columns.

A sparse vector is created with:

```rust
SparseVector::new(total_dimensions, indices, values)
```

where, total_dimensions is the full size of the vector, indices contains the
positions of the non-zero values, and values contains the non-zero values using
a VectorData variant.

```rust
use oracledb::{SparseVector, Vector, VectorData};

let float32_val = Vector::Sparse(SparseVector::new(
    25,
    vec![6, 10, 18],
    VectorData::Float32(vec![26.25, 129.625, 579.875]),
));

let float64_val = Vector::Sparse(SparseVector::new(
    30,
    vec![9, 16, 24],
    VectorData::Float64(vec![19.125, 78.5, 977.375]),
));

let int8_val = Vector::Sparse(SparseVector::new(
    35,
    vec![10, 20, 30],
    VectorData::Int8(vec![26, 125, -37]),
));

connection.execute(
    "insert into vector_sparse_table values (:1, :2, :3)",
    &[&float32_val, &float64_val, &int8_val],
)?;
```

### <a name="fetchsparsevectors"></a> 12.3.2 Fetching Sparse Vectors

With rust-oracledb, sparse VECTOR columns are fetched as
[`Vector`](crate::Vector) values. For sparse VECTOR columns, the fetched value
is `Vector::Sparse`, which contains a [`SparseVector`](crate::SparseVector).

```rust
let cursor = connection.query(
    "select * from vector_sparse_table",
    &[],
)?;

for row_result in cursor {
    let row = row_result?;

    let v32: oracledb::Vector = row.get(0)?;
    let v64: oracledb::Vector = row.get(1)?;
    let v8: oracledb::Vector = row.get(2)?;

    println!("v32 = {v32:?}");
    println!("v64 = {v64:?}");
    println!("v8 = {v8:?}");
}
```

This prints:

```text
v32 = Sparse(SparseVector { num_dimensions: 25, indices: [6, 10, 18], values: Float32([26.25, 129.625, 579.875]) })
v64 = Sparse(SparseVector { num_dimensions: 30, indices: [9, 16, 24], values: Float64([19.125, 78.5, 977.375]) })
v8 = Sparse(SparseVector { num_dimensions: 35, indices: [10, 20, 30], values: Int8([26, 125, -37]) })
```

[Oracle AI Vector Search User's Guide]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=VECSE
[sparse vector]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-6015566C-3277-4A3C-8DD0-08B346A05478
[VECTOR]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-746EAA47-9ADA-4A77-82BB-64E8EF5309BE
