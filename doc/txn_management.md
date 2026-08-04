# <a name="txnmgmnt"></a> 7. Managing Transactions

A database transaction is a grouping of SQL statements that make a logical data
change to the database. When statements like
[Connection::execute()](crate::Connection::execute) or
[Connection::execute_batch()](crate::Connection::execute_batch) execute SQL
statements like INSERT or UPDATE, a transaction is started or continued. By
default, rust-oracledb does not commit this transaction to the database. You
can explicitly commit or roll it back using the methods
[Connection::commit()](crate::Connection::commit) and
[Connection::rollback()](crate::Connection::rollback). For example, to commit a
new row:

```rust
connection.execute(
    "INSERT INTO mytab (name) VALUES ('John')", &[])?;
connection.commit()?;
```

When a database connection is closed, such as with
[Connection::close()](crate::Connection::close), or when variables referencing
the connection go out of scope, any uncommitted transaction will be rolled
back.

When [Data Definition Language (DDL)] statements such as CREATE are executed,
Oracle Database will always perform a commit.

[Data Definition Language (DDL)]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-FD9A8CB4-6B9A-44E5-B114-EFB8DA76FC88
