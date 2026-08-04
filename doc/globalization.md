# <a name="globalization"></a> 15. Character Sets and Globalization

## <a name="dbcharset"></a> 15.1 Database Character Set

Data fetched from and sent to Oracle Database is mapped between the
[database character set] and the UTF-8 client character set used by
rust-oracledb.

To find the database character set, execute:

```sql
SELECT value AS db_charset
FROM nls_database_parameters
WHERE parameter = 'NLS_CHARACTERSET';
```

## <a name="dbnationalcharset"></a> 15.2 Database National Character Set

The [national character set] is used for NCHAR, NVARCHAR2, and NCLOB data
types.

To find the database national character set, execute:

```sql
SELECT value AS db_ncharset
FROM nls_database_parameters
WHERE parameter = 'NLS_NCHAR_CHARACTERSET';
```

## <a name="setclientcharset"></a> 15.3 Setting the Client Character Set

Rust-oracledb uses UTF-8 for character data. There are no connection parameters
for setting a different client encoding.

[database character set]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-EA913CC8-C5BA-4FB3-A1B8-882734AF4F43
[national character set]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-4E12D991-C286-4F1A-AFC6-F35040A5DE4F
