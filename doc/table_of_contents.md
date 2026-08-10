Rust-oracledb Driver Documentation.

# Table of Contents

The rust-oracledb driver is an open source Rust module that enables quick and
easy access to Oracle Database for Rust applications without the use of Oracle
Client libraries. It is lightweight and high-performance. The module is
maintained by Oracle.

This is a pre-release of rust-oracledb, intended to provide early access to
the driver and gather user feedback. The APIs and functionalities are subject
to change as development continues.

You can use assistive technology products, such as screen readers, while you
work with the rust-oracledb documentation. You can also use the keyboard
instead of the mouse.

**User Guide**

1. [Rust-oracledb Driver for Oracle Database](#introduction)

   1.1 [Architecture](#architecture)

   1.2 [Installing rust-oracledb](#installing)
   - 1.2.1 [Installation Requirements](#instreq)
   - 1.2.2 [Installation](#installation)

   1.3 [Feature Highlights of rust-oracledb](#featurehighlights)

2. [Connecting to Oracle Database](#connhandling)

   2.1 [Create a Basic Configuration](#basicconfig)

   2.2 [Oracle Net Services Connection Strings](#connstr)
   - 2.2.1 [Easy Connect Syntax for Connection Strings](#easyconnect)
   - 2.2.2 [Connect Descriptors](#conndescriptor)
   - 2.2.3 [TNS Aliases for Connection Strings](#netservice)
   - 2.2.4 [Optional Oracle Net Configuration File](#optnetfile)
   - 2.2.5 [JDBC and Oracle SQL Developer Connection Strings](#jdbcconnstring)

   2.3 [Standalone Connections](#standaloneconnection)
   - 2.3.1 [Creating a Standalone Connection](#createstandaloneconnection)
   - 2.3.2 [Closing Connections](#closeconnection)

   2.4 [Authenticating to Oracle Database](#authentication)
   - 2.4.1 [Database Authentication](#dbauthentication)
   - 2.4.2 [Proxy Authentication](#proxyauth)

   2.5 [Connection Pooling](#connpooling)
   - 2.5.1 [Create a Basic Pool Configuration](#basicpoolconfig)
   - 2.5.2 [Creating a Connection Pool](#createconnpool)
   - 2.5.3 [Getting Connections from a Pool](#getconnpool)
   - 2.5.4 [Returning Connections to a Pool](#returnconnpool)
   - 2.5.5 [Closing a Connection Pool](#closeconnpool)
   - 2.5.6 [Connection Pool Sizing](#connpoolsize)
     - 2.5.6.1 [Connection Pool Growth](#connpoolgrowth)
   - 2.5.7 [Pool Connection Health](#poolhealth)

   2.6 [Deep Data Security](#deepdatasecurity)
   - 2.6.1 [Creating an End-User Security Context Payload](#createendusersecctx)
   - 2.6.2 [Setting an End-User Security Context Payload](#setendusersecctx)
   - 2.6.3 [Clearing an End-User Security Context Payload](#clearendusersecctx)

   2.7 [Privileged Connections](#privilegedconn)

   2.8 [Securely Encrypting Network Traffic to Oracle Database](#netencrypt)

   2.9 [Resetting Passwords](#resetpassword)

   2.10 [Connecting to Oracle Cloud Autonomous Databases](#autonomousdb)
   - 2.10.1 [One-way TLS Connection to Oracle Autonomous Database](#onewaytls)
     - 2.10.1.1 [Allowing One-way TLS Access to Oracle Autonomous Database](#allowonewaytls)
     - 2.10.1.2 [Connecting with One-way TLS](#connectonewaytls)
   - 2.10.2 [Mutual TLS (mTLS) Connection to Oracle Autonomous Database](#twowaytls)
     - 2.10.2.1 [Allowing mTLS Access to Oracle Autonomous Database](#allowmtls)
     - 2.10.2.2 [Downloading the Database Wallet](#getwallet)
     - 2.10.2.3 [Connecting with mTLS](#connectmtls)
     - 2.10.2.4 [Using the Easy Connect Syntax with Oracle Autonomous Database](#easyconnectadb)
     - 2.10.2.5 [Creating a PEM File](#createpem)

3. [Executing SQL](#sqlexecution)

   3.1 [SELECT Statements](#sqlqueries)
   - 3.1.1 [Fetching a Single Row](#fetchsinglerow)
   - 3.1.2 [Fetching Multiple Rows](#fetchmultiplerows)
   - 3.1.3 [Fetch Data Types](#defaultfetchtypes)
   - 3.1.4 [Limiting Rows](#rowlimit)

   3.2 [INSERT and UPDATE Statements](#dml)

   3.3 [Dynamic SQL Construction and Validation](#validatingsql)
   - 3.3.1 [Quoting SQL Identifiers](#quotenames)
   - 3.3.2 [Quoting Literals](#quoteliterals)
   - 3.3.3 [Validating Simple SQL Names](#validatesimplesqlnames)
   - 3.3.4 [Validating Qualified SQL Names](#validatequalifiedsqlnames)

4. [Executing PL/SQL](#plsqlexecution)

   4.1 [PL/SQL Stored Procedures](#plsqlproc)

   4.2 [PL/SQL Stored Functions](#plsqlfunc)

   4.3 [Anonymous PL/SQL Blocks](#anonplsql)

   4.4 [Passing NULL values to PL/SQL](#plsqlnull)

   4.5 [Creating Stored Procedures and Packages](#storedprocpkg)
   - 4.5.1 [PL/SQL Compilation Warnings](#plsqlwarning)

   4.6 [Using DBMS_OUTPUT](#dbmsoutput)

   4.7 [Edition-Based Redefinition (EBR)](#ebr)

5. [Using Bind Variables](#bind)

   5.1 [Binding by Name or Position](#binding)
   - 5.1.1 [Bind by Name](#bindbyname)
   - 5.1.2 [Bind by Position](#bindbyposition)

   5.2 [Duplicate Bind Variable Placeholders](#dupbindplaceholders)

   5.3 [Bind Direction](#binddir)

   5.4 [Binding Null Values](#bindnull)

   5.5 [Binding ROWID Values](#bindrowid)

   5.6 [Binding UROWID Values](#bindurowid)

   5.7 [DML RETURNING Bind Variables](#dml-returning-bind)

   5.8 [Binding Multiple Values to a SQL WHERE IN Clause](#multiplevalueswherein)
   - 5.8.1 [Binding a Large Number of Items in an IN List](#bindinlist)

   5.9 [Binding Column and Table Names](#bindcoltblnames)

6. [Executing Batch Statements and Bulk Loading](#batchstmnt)

   6.1 [Batch Statement Execution](#batchstmntexec)
   - 6.1.1 [Batch Execution of SQL](#batchstmntexecsql)
   - 6.1.2 [Batch Execution of PL/SQL](#batchplsql)

   6.2 [Identifying Affected Rows](#identifyaffectedrows)

   6.3 [DML RETURNING](#dmlreturning)

7. [Managing Transactions](#txnmgmnt)

8. [Tuning rust-oracledb](#tuning)

   8.1 [Database Round-trips](#roundtrips)
   - 8.1.1 [Finding the Number of Round-Trips](#numroundtrips)

   8.2 [Statement Caching](#stmtcache)
   - 8.2.1 [Setting the Statement Cache](#setstmtcache)
   - 8.2.2 [Tuning the Statement Cache](#tunestmtcache)
   - 8.2.3 [Disabling the Statement Cache](#disablestmtcache)

9. [Using CLOB, NCLOB, and BLOB](#lob)

   9.1 [Simple Inserting and Querying of LOBs](#simplelobs)

   9.2 [Inserting and Querying Using LOB Locator](#loblocator)

10. [Using JSON Data](#jsondatatype)

    10.1 [Inserting Oracle Database JSON Type](#insertjsondatatype)

    10.2 [Fetching Oracle Database JSON Type](#fetchjsondatatype)

    10.3 [IN Bind Type Mapping](#inbindtypemapping)

    10.4 [Query and OUT Bind Type Mapping](#outbindtypemapping)

    10.5 [SQL/JSON Path Expressions](#pathexpr)

    10.6 [Accessing Relational Data as JSON](#accessrelationaldata)

    10.7 [JSON-Relational Duality Views](#jsondualityviews)

11. [Using INTERVAL Data](#intervaldatatype)

    11.1 [Using INTERVAL YEAR TO MONTH Data](#intervalym)
    - 11.1.1 [Inserting INTERVAL YEAR TO MONTH Data](#insertintervalym)
    - 11.1.2 [Fetching INTERVAL YEAR TO MONTH Data](#fetchintervalym)

    11.2 [Using INTERVAL DAY TO SECOND Data](#intervalds)
    - 11.2.1 [Inserting INTERVAL DAY TO SECOND Data](#insertintervalds)
    - 11.2.2 [Fetching INTERVAL DAY TO SECOND Data](#fetchintervalds)

12. [Using VECTOR Data](#vectors)

    12.1 [Using FLOAT32, FLOAT64, and INT8 Vectors](#intfloatformat)
    - 12.1.1 [Inserting FLOAT32, FLOAT64, and INT8 Vectors](#insertintfloatformat)
    - 12.1.2 [Fetching FLOAT32, FLOAT64, and INT8 Vectors](#fetchintfloatformat)

    12.2 [Using BINARY Vectors](#binaryformat)
    - 12.2.1 [Inserting BINARY Vectors](#insertbinaryvector)
    - 12.2.2 [Fetching BINARY Vectors](#fetchbinaryvector)

    12.3 [Using SPARSE Vectors](#sparsevectors)
    - 12.3.1 [Inserting SPARSE Vectors](#insertsparsevectors)
    - 12.3.2 [Fetching Sparse Vectors](#fetchsparsevectors)

13. [Using Apache Arrow Data](#arrowdata)

    13.1 [Inserting Arrow Data](#insertingarrowdata)
    - 13.1.1 [Arrow Bind Type Mapping](#arrowbindtypemapping)

    13.2 [Fetching Arrow Data](#fetchingarrowdata)
    - 13.2.1 [Arrow Fetch Type Mapping](#arrowfetchtypemapping)

14. [High Availability with rust-oracledb](#highavailability)

    14.1 [General HA Recommendations](#harecommend)

    14.2 [Network Configuration](#hanetwork)

15. [Character Sets and Globalization](#globalization)

    15.1 [Database Character Set](#dbcharset)

    15.2 [Database National Character Set](#dbnationalcharset)

    15.3 [Setting the Client Character Set](#setclientcharset)

16. [Tracing rust-oracledb](#tracingsql)

    16.1 [Application Tracing](#applntracing)
    - 16.1.1 [Oracle Database End-to-End Tracing](#endtoendtracing)
    - 16.1.2 [Using Connection Identifiers](#connectionid)
    - 16.1.3 [Tracing Bind Values](#tracingbind)
    - 16.1.4 [Database Views for Tracing rust-oracledb](#dbviews)
      - 16.1.4.1 [V$SESSION](#vsession)
      - 16.1.4.2 [V$SESSION_CONNECT_INFO](#vsessionconninfo)

    16.2 [Low Level Rust-oracledb Driver Tracing](#lowleveltracing)

17. [Appendix A: Oracle Database Features Supported by rust-oracledb](#appendixa)

**Release Notes**

[rust-oracledb Release Notes](#releasenotes)

**API Manual**

The API reference is generated by rustdoc from the public Rust items exported by
the crate and includes the following sections:

- [Structs](../index.html#structs)
- [Enums](../index.html#enums)
- [Traits](../index.html#traits)
- [Functions](../index.html#functions)
- [Constants](../index.html#constants)
