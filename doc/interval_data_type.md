# <a name="intervaldatatype"></a> 11. Using INTERVAL Data

Oracle Database supports two INTERVAL data types that store time durations -
INTERVAL YEAR TO MONTH and INTERVAL DAY TO SECOND. For more information on
these data types, see [Oracle Interval Types].

## <a name="intervalym"></a> 11.1 Using INTERVAL YEAR TO MONTH Data

The INTERVAL YEAR TO MONTH data type stores a period of time using years and
months.

To create a table with a column for INTERVAL DAY TO SECOND data, for example:

```sql
CREATE TABLE Table_IntervalYM (IntervalYM_Col INTERVAL YEAR TO MONTH);
```

### <a name="insertintervalym"></a> 11.1.1 Inserting INTERVAL YEAR TO MONTH Data

To insert into an INTERVAL YEAR TO MONTH Data column, create an
[OracleIntervalYM](crate::OracleIntervalYM) value. You can define the years and
months values in the OracleIntervalYM value. For example:

```rust
let interval_value = OracleIntervalYM::new(2, 3);
```

This creates an OracleIntervalYM value with 2 years and 3 months.

To insert an OracleIntervalYM value into an INTERVAL YEAR TO MONTH column, for
example:

```rust
connection.execute(
    "insert into Table_IntervalYM (IntervalYM_Col) values (:1)",
    &[&interval_value],
)?;
```

### <a name="fetchintervalym"></a> 11.1.2 Fetching INTERVAL YEAR TO MONTH Data

To query an INTERVAL YEAR TO MONTH column, you can use:

```rust
let row = connection.query_row(
    "select IntervalYM_Col from Table_IntervalYM",
    &[],
)?;

let value: OracleIntervalYM = row.get(0)?;
println!(
    "IntervalYM {{ years: {}, months: {} }}",
    value.years(),
    value.months()
);
```

This query prints the following output:

```text
IntervalYM { years: 2, months: 3 }
```

## <a name="intervalds"></a> 11.2 Using INTERVAL DAY TO SECOND Data

The INTERVAL DAY TO SECOND data type stores a period of time using days, hours,
minutes, seconds, and fractional seconds.

To create a table with a column for INTERVAL DAY TO SECOND data, for example:

```sql
create table Table_IntervalDS (IntervalDS_Col INTERVAL DAY TO SECOND);
```

### <a name="insertintervalds"></a> 11.2.1 Inserting INTERVAL DAY TO SECOND Data

To insert into an INTERVAL DAY TO SECOND column, create an
[OracleIntervalDS](crate::OracleIntervalDS) value. You can define the days,
hours, minutes, seconds, and nanoseconds values in the OracleIntervalDS value.
For example:

```rust
let interval_value = OracleIntervalDS::new(5, 3, 4, 6, 0);
```

This creates an OracleIntervalDS value with 5 days, 3 hours, 4 minutes,
6 seconds, and 0 nanoseconds.

To insert an OracleIntervalDS value into an INTERVAL DAY TO SECOND column, for
example:

```rust
connection.execute(
    "insert into Table_IntervalDS (IntervalDS_Col) values (:1)",
    &[&interval_value],
)?;
```

### <a name="fetchintervalds"></a> 11.2.2 Fetching INTERVAL DAY TO SECOND Data

To query an INTERVAL DAY TO SECOND column, you can use:

```rust
let cursor = connection.query(
    "select Interval_DSCol from Table_IntervalDS",
    &[],
)?;
let value: OracleIntervalDS = row.get(0)?;
println!(
    "IntervalDS {{ days: {}, hours: {}, minutes: {}, seconds: {}, fseconds: {} }}",
    value.days(),
    value.hours(),
    value.minutes(),
    value.seconds(),
    value.nanoseconds()
);
```

This query prints the following output:

```text
IntervalDS { days: 5, hours: 3, minutes: 4, seconds: 6, fseconds: 0 }
```

[Oracle Interval Types]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-7690645A-0EE3-46CA-90DE-C96DF5A01F8F
