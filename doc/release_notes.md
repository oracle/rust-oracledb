# <a name="releasenotes"></a> rust-oracledb Release Notes

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
    Oracle Database 26ai
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
