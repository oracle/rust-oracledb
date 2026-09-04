# <a name="releasenotes"></a> rust-oracledb Release Notes

## rust-oracledb 26.0.0-beta.3 (TBD)

1.  Added methods [Row::take()](crate::Row::take()) and
    [Row::take_array()](crate::Row::take_array()) which transfer ownership of
    the data in the row to the caller. The existing methods
    [Row::get()](crate::Row::get()) and
    [Row::get_array()](crate::Row::get_array()) return references to the row
    data where possible and clone the data where an owned type is desired. The
    method ``Row::get_cursor()`` has been removed in favor of the new method
    [Row::take()](crate::Row::take()).
1.  The method [Row::get()](crate::Row::get()) can now return `&str` and
    `&[u8]` references for string and raw data respectively. This allows
    returning a reference to the fetched data without copying it.
1.  Added support for using a name instead of a numeric position to identify
    columns in [Row::get()](crate::Row::get()) and
    [Row::take()](crate::Row::take())
    ([issue 12](https://github.com/oracle/rust-oracledb/issues/12)).
1.  Added support for binding long values in any order
    ([issue 10](https://github.com/oracle/rust-oracledb/issues/10)).
1.  Added support for the HA readiness requirements of Oracle Database 23.26.3.
1.  Errors that are returned now capture the backtrace and display it if
    configured with `RUST_BACKTRACE=1`, which aids in debugging.
1.  Improved errors that are a result of a failure to parse the server's
    response to a request.
1.  Fixed bug where returning a connection to the pool did not end the request
    correctly
    ([issue 15](https://github.com/oracle/rust-oracledb/issues/15)).
1.  Removed the ability to clone [Cursor](crate::Cursor) and [Lob](crate::Lob).
1.  Fixed bug which caused a named binding containing a single quote to panic.
1.  Fixed bug which caused a hang when executing a statement with PL/SQL out
    binds multiple times
    ([issue 17](https://github.com/oracle/rust-oracledb/issues/17)).
1.  Fixed bug which permitted a pool to be created with the maximum number of
    connections set to zero.  The error
    [ErrorKind::PoolMaxInvalid](crate::ErrorKind::PoolMaxInvalid) was renamed
    from `ErrorKind::PoolMaxLessThanMin` which now covers both scenarios.


## rust-oracledb 26.0.0-beta.2 (August 20, 2026)

1.  Added method [Statement::bind_names()](crate::Statement::bind_names()) in
    order to determine the list of bind variable names used by a statement.
1.  The struct [Row](crate::Row) has been exported publicly so that
    documentation on it is visible.
1.  Added method [Row::get_array()](crate::Row::get_array()) in order to get
    values returned in a DML RETURNING statement using the same types as are
    possible with scalar values.
1.  Fixed bugs and enhanced parsing of SQL statements
    ([issue 1](https://github.com/oracle/rust-oracledb/issues/1)).
1.  Fixed bugs and enhanced parsing of connect strings, including the handling
    of listener redirects
    ([issue 2](https://github.com/oracle/rust-oracledb/issues/2)).
1.  Fixed bug handling multiple packet responses with databases older than
    Oracle AI Database 26ai
    ([issue 5](https://github.com/oracle/rust-oracledb/issues/5)).
1.  Fixed bugs with reading and writing CLOB/NCLOB when the database character
    set is a fixed width character set.
1.  Fixed bug when a statement is executed twice and the second time a value
    that is bound to a placeholder is larger than the value bound to that
    placeholder the first time.
1.  Fixed bug when a statement querying LOBs is executed twice and the second
    time the `fetch_lobs` option is different from the first time.
1.  String decoding now returns an error instead of panicing when invalid
    encoded string data is detected.
1.  Added runnable examples.


## rust-oracledb 26.0.0-beta.1 (August 6, 2026)

Initial release of the rust-oracledb driver for Oracle Database.
