# <a name="releasenotes"></a> rust-oracledb Release Notes

## rust-oracledb 26.0.0-beta.2 (TBD)

1.  Added method [Statement::bind_names()](crate::Statement::bind_names()) in
    order to determine the list of bind variable names used by a statement.
1.  Fixed bugs and enhanced parsing of SQL statements
    ([issue 1](https://github.com/oracle/rust-oracledb/issues/1)).
1.  Fixed bugs and enhanced parsing of connect strings, including the handling
    of listener redirects
    ([issue 2](https://github.com/oracle/rust-oracledb/issues/2)).
1.  String decoding now returns an error instead of panicing when invalid
    encoded string data is detected.
1.  Added runnable examples.


## rust-oracledb 26.0.0-beta.1 (August 6, 2026)

Initial release of the rust-oracledb driver for Oracle Database.
