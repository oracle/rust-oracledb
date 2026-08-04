# <a name="appendixa"></a> 17. Appendix A: Oracle Database Features Supported by rust-oracledb

Rust-oracledb supports Oracle Database versions 26ai, 21c, and 19c. The
following table summarizes the Oracle Database features supported by
rust-oracledb. The first column displays the Oracle Database feature. The
second column displays whether it is supported in rust-oracledb.

| Oracle feature                                                  | rust-oracledb        |
| --------------------------------------------------------------- | -------------        |
| Standalone connections                                          | Yes                  |
| Connection Pooling                                              | Homogeneous only     |
| Named Connection Pools                                          | No                   |
| Connection Pool Connection Load Balancing (CLB)                 | Yes                  |
| Connection Pool Runtime Load Balancing (RLB)                    | No                   |
| Connection Pool draining                                        | Yes                  |
| Connection Pool session state callback                          | No                   |
| Connection pool session tagging                                 | No                   |
| Password authentication                                         | Yes                  |
| External authentication                                         | No                   |
| Token-based authentication                                      | No                   |
| Kerberos and Radius authentication                              | No                   |
| LDAP connections                                                | No                   |
| Oracle Deep Data Security                                       | Yes                  |
| Proxy connections                                               | Yes                  |
| SOCKS Proxy connections                                         | No                   |
| Connection mode privileges                                      | Yes                  |
| Preliminary connections                                         | No                   |
| Set current schema using an attribute                           | Yes                  |
| Oracle Cloud Database connectivity                              | Yes                  |
| Real Application Clusters (RAC)                                 | Yes                  |
| Oracle Globally Distributed Database                            | No                   |
| Native Network Encryption (NNE)                                 | No - use TLS instead |
| `tnsnames.ora` file                                             | Yes                  |
| `sqlnet.ora` file                                               | Not applicable       |
| `oraaccess.xml`                                                 | Not applicable       |
| Easy Connect connection strings                                 | Yes                  |
| Centralized Configuration Providers                             | No                   |
| One-way TLS connections                                         | Yes                  |
| Mutual TLS (mTLS) connections                                   | Yes                  |
| Secure External Password Store (SEPS) wallet                    | No                   |
| Dedicated Servers, Shared Servers, and DRCP                     | Yes                  |
| 26ai Implicit Connection Pooling with DRCP and PRCP             | No                   |
| Multitenant Databases                                           | Yes                  |
| CMAN and CMAN-TDM connectivity                                  | Yes                  |
| Password changing                                               | Yes                  |
| Edition Based Redefinition (EBR)                                | Yes                  |
| SQL execution                                                   | Yes                  |
| PL/SQL execution                                                | Yes                  |
| SODA                                                            | No                   |
| Bind variables for data binding                                 | Yes                  |
| Array DML binding for bulk DML and PL/SQL                       | Yes                  |
| SQL and PL/SQL types and collections                            | No                   |
| Query column metadata                                           | Yes                  |
| Client character set support                                    | UTF-8                |
| Row prefetching on first query execute                          | Yes                  |
| Array fetching for queries                                      | Yes                  |
| Statement caching                                               | Yes                  |
| Client Result Caching (CRC)                                     | No                   |
| Direct Path Loads                                               | No                   |
| 26ai JSON-Relational Duality Views                              | Yes                  |
| Continuous Query Notification (CQN)                             | No                   |
| Transactional Event Queues and Advanced Queuing (AQ)            | No                   |
| Call timeouts                                                   | Yes                  |
| Scrollable cursors                                              | No                   |
| Database startup and shutdown                                   | No                   |
| Transaction management                                          | Yes                  |
| Fast Application Notification (FAN)                             | No                   |
| In-band notifications                                           | Yes                  |
| Transparent Application Failover (TAF)                          | No                   |
| Transaction Guard (TG)                                          | No                   |
| Data Guard and Active Data Guard                                | No                   |
| Application Continuity and Transparent Application Continuity   | No                   |
| 26ai Pipelining                                                 | No                   |
| End-to-end monitoring and tracing                               | Yes                  |
| Java Debug Wire Protocol for debugging PL/SQL                   | No                   |
| 26ai Sessionless Transactions                                   | No                   |
| Two-phase Commit (TPC)                                          | No                   |
| Pipelined tables                                                | Yes                  |
| Implicit Result Sets                                            | No                   |
| Persistent and Temporary LOBs                                   | Yes                  |
| LOB length prefetching                                          | Yes                  |
| LOB locator operations such as trim                             | Yes                  |
