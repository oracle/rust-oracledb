# <a name="highavailability"></a> 14. High Availability with rust-oracledb

Rust-oracledb applications can take advantage of Oracle Database high
availability features by using appropriate database, network, and connection
configuration. These features can help reduce downtime during planned and
unplanned outages, preserve performance, and simplify operational recovery.

## <a name="harecommend"></a> 14.1 General HA Recommendations

General recommendations for creating highly available rust-oracledb programs
are:

- Tune operating system and Oracle Network parameters to avoid long TCP
  timeouts, prevent firewalls killing connections, and avoid connection storms.
- Implement application error handling and recovery.
- Use the most recent version of Oracle Database.
- Use Oracle Database technologies such as [RAC] or standby databases.
- Use a [connection pool](#connpooling) because pools can handle database
  events and take proactive and corrective action for draining, run time load
  balancing, and fail over. Set the minimum and maximum pool sizes to the same
  values to avoid connection storms. Remove resource manager or user profiles
  that prematurely close sessions.
- Test all scenarios thoroughly.

## <a name="hanetwork"></a> 14.2 Network Configuration

The operating system TCP and Oracle Net configuration should be configured for
performance and availability.

Rust-oracledb is a Thin driver, so timeout and high availability options should
be configured using supported connect string options, connect descriptors, pool
settings, and application-level error handling.

Options such as connection retries, retry delays, failover, load balancing, and
connection keepalive settings can be explored.

[Oracle Net Services] options may also be useful for high availability and
performance tuning. These are configured in database `listener.ora`
configuration files or connect descriptors. For example, the database's
`listener.ora` file can have [RATE_LIMIT] and [QUEUESIZE] parameters that can
help handle connection storms.

These options are independent of the Rust application code, but they can affect
how quickly rust-oracledb applications can establish connections and recover
during planned or unplanned outages.

[Oracle Net Services]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=NETRF
[QUEUESIZE]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-FF87387C-1779-4CC3-932A-79BB01391C28
[RAC]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=RACAD
[RATE_LIMIT]: https://www.oracle.com/pls/topic/lookup?ctx=dblatest&id=GUID-F302BF91-64F2-4CE8-A3C7-9FDB5BA6DCF8
