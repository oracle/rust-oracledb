//-----------------------------------------------------------------------------
// Copyright (c) 2026, Oracle and/or its affiliates.
//
// This software is dual-licensed to you under the Universal Permissive License
// (UPL) 1.0 as shown at https://oss.oracle.com/licenses/upl and Apache License
// 2.0 as shown at http://www.apache.org/licenses/LICENSE-2.0. You may choose
// either license.
//
// If you elect to accept the software under the Apache License, Version 2.0,
// the following applies:
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//-----------------------------------------------------------------------------

//-----------------------------------------------------------------------------
// error.rs
//
// Defines the errors used by the driver.
//-----------------------------------------------------------------------------

use std::error;
use std::fmt;
use std::io;

use crate::db_type::DbType;

/// Types of errors returned by the library.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum ErrorKind {
    ArrowOperation,
    CallTimeoutExceeded,
    ColumnTruncated(usize, usize),
    DbError(String),
    DeadConnection,
    DifferentTypes(&'static DbType, &'static DbType),
    EndUserSecurityContextRequiresTcps,
    IfileCycleDetected(String, String),
    IntegerTooLarge(usize, usize),
    InvalidBindName(String),
    InvalidColumnIndex(usize),
    InvalidConnectString(String, String),
    InvalidDescriptorNode(String, String),
    InvalidEncodedVector,
    InvalidEndUserSecurityContext(String),
    InvalidEndUserSecurityContextLength(usize),
    InvalidNetworkName(String),
    InvalidOracleNumber(String),
    InvalidOsonEncodedBytes,
    InvalidServiceName(String, String, String, u16),
    InvalidSid(String, String, String, u16),
    ListenerRefusedConnection(String, String, u16, usize),
    MissingBindValue(String),
    NameHasEmbeddedQuotes,
    NoConfigDir,
    NoConnectString,
    NoCredentials,
    NoDataFound,
    NotConnected,
    NotImplemented(String),
    OutOfData,
    OutOfRange(String),
    ParseError(String, usize),
    PemFileOperation,
    PoolHasBusyConnections,
    PoolIncrementZero,
    PoolMaxLessThanMin,
    PoolNotOpen,
    ServerVersionNotSupported,
    StreamOperation,
    TlsOperation,
    TnsAliasNotFound(String, String),
    UnableToRecover,
    UnexpectedError,
    UnexpectedNegativeInteger,
    UnexpectedRefuse(String),
    UnexpectedResult,
    UnknownServerSidePiggyback(u8),
    UnknownTtcMessageType(u8),
    UnsupportedArrowType(String),
    UnsupportedConversion(String, String),
    UnsupportedDbType(&'static DbType),
    UnsupportedDeepDataSecurityFeature,
    UnsupportedOsonNodeType(u8),
    UnsupportedOsonVersion(u8),
    UnsupportedVectorFormat(u8),
    UnsupportedVectorVersion(u8),
    ValueWasNull,
    WalletPasswordMissingOrInvalid(String, String),
    WalletPrivateKeyInvalid(String),
    WalletUnreadable(String),
    WrongNumPositionalBinds(usize, usize),
}

struct ErrorInner {
    kind: ErrorKind,
    cause: Option<Box<dyn error::Error + Sync + Send>>,
}

/// Represents errors returned by the library.
pub struct Error(Box<ErrorInner>);

impl fmt::Debug for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Error")
            .field("kind", &self.0.kind)
            .field("cause", &self.0.cause)
            .finish()
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        if e.kind() == std::io::ErrorKind::WouldBlock
            || e.kind() == std::io::ErrorKind::TimedOut
        {
            Error::call_timeout_exceeded()
        } else {
            Error::stream_operation(e)
        }
    }
}

impl From<rustls::Error> for Error {
    fn from(e: rustls::Error) -> Error {
        Error::tls_operation(e)
    }
}

impl From<rustls::pki_types::pem::Error> for Error {
    fn from(e: rustls::pki_types::pem::Error) -> Error {
        Error::pem_file_operation(e)
    }
}

#[cfg(feature = "arrow")]
impl From<arrow_schema::ArrowError> for Error {
    fn from(e: arrow_schema::ArrowError) -> Self {
        Self::arrow_operation(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0.kind {
            ErrorKind::ArrowOperation => {
                fmt.write_str("Arrow operation failed")?
            }
            ErrorKind::CallTimeoutExceeded => {
                fmt.write_str("the configured call timeout was exceeded")?
            }
            ErrorKind::ColumnTruncated(col_value_size, actual_size) => write!(
                fmt,
                "column truncated to {col_value_size} bytes. \
                 Untruncated was {actual_size} bytes."
            )?,
            ErrorKind::DbError(message) => fmt.write_str(message)?,
            ErrorKind::DeadConnection => {
                fmt.write_str("the database or network closed the connection")?
            }
            ErrorKind::DifferentTypes(initial_db_type, subsequent_db_type) => {
                write!(
                    fmt,
                    "database type {} found when all earlier bind data was of \
                 database type {}",
                    subsequent_db_type, initial_db_type
                )?
            }
            ErrorKind::EndUserSecurityContextRequiresTcps => fmt.write_str(
                "end_user_security_context requires use of the tcps protocol",
            )?,
            ErrorKind::IfileCycleDetected(
                including_file_name,
                included_file_name,
            ) => write!(
                fmt,
                "file \"{}\" includes file \"{}\" which forms a cycle",
                including_file_name, included_file_name
            )?,
            ErrorKind::IntegerTooLarge(max_size, actual_size) => write!(
                fmt,
                "internal error: read integer of length {actual_size} when \
                 expecting an integer of no more than length {max_size}"
            )?,
            ErrorKind::InvalidBindName(name) => write!(
                fmt,
                "no bind placeholder named \":{name}\" was found \
                    in the SQL text"
            )?,
            ErrorKind::InvalidColumnIndex(index) => {
                write!(fmt, "invalid column index {} (zero-based)", index)?
            }
            ErrorKind::InvalidConnectString(connect_string, reason) => {
                write!(fmt, "invalid connect string: {connect_string}: {reason}")?
            }
            ErrorKind::InvalidDescriptorNode(key, expected_type) => write!(
                fmt,
                "full descriptor node {key} is not a valid {expected_type}"
            )?,
            ErrorKind::InvalidEncodedVector => {
                write!(fmt, "invalid encoded vector")?
            }
            ErrorKind::InvalidEndUserSecurityContext(message) => {
                fmt.write_str(message)?
            }
            ErrorKind::InvalidEndUserSecurityContextLength(actual_length) => {
                write!(
                    fmt,
                    "end user security context is too long: {actual_length} bytes"
                )?
            }
            ErrorKind::InvalidNetworkName(value) => write!(
                fmt,
                "\"{value}\" includes characters that are not allowed"
            )?,
            ErrorKind::InvalidOracleNumber(v) => {
                write!(fmt, "invalid number {}", v)?
            }
            ErrorKind::InvalidOsonEncodedBytes => {
                fmt.write_str("invalid OSON encoded bytes")?
            }
            ErrorKind::InvalidServiceName(
                connection_id,
                service_name,
                host,
                port,
            ) => write!(
                fmt,
                "service \"{service_name}\" is not registered with the \
                 listener at host \"{host}\" port {port}. \
                 (Similar to ORA-12514) (CONNECTION_ID={connection_id})"
            )?,
            ErrorKind::InvalidSid(connection_id, sid, host, port) => write!(
                fmt,
                "SID \"{sid}\" is not registered with the listener at \
                 host \"{host}\" port {port}. (Similar to ORA-12505) \
                 (CONNECTION_ID={connection_id})"
            )?,
            ErrorKind::ListenerRefusedConnection(
                connection_id,
                host,
                port,
                error_num,
            ) => write!(
                fmt,
                "listener at host \"{host}\" port {port} refused \
                     connection. (Similar to ORA-{error_num}) \
                     (CONNECTION_ID={connection_id})"
            )?,
            ErrorKind::OutOfData => fmt.write_str("out of data")?,
            ErrorKind::OutOfRange(m) => write!(fmt, "{}", m)?,
            ErrorKind::ParseError(s, p) => {
                write!(fmt, "parse error at position {p} in text {s}")?
            }
            ErrorKind::MissingBindValue(name) => write!(
                fmt,
                "a bind variable replacement value for \
                placeholder \":{name}\" was not provided"
            )?,
            ErrorKind::NameHasEmbeddedQuotes => {
                fmt.write_str("name has embedded quotes")?
            }
            ErrorKind::NoConfigDir => {
                fmt.write_str("no configuration directory specified")?
            }
            ErrorKind::NoConnectString => {
                fmt.write_str("no connect string specified")?
            }
            ErrorKind::NoCredentials => {
                fmt.write_str("no credentials specified")?
            }
            ErrorKind::NoDataFound => fmt.write_str("no data found")?,
            ErrorKind::NotConnected => {
                fmt.write_str("not connected to database")?
            }
            ErrorKind::NotImplemented(feature) => {
                write!(fmt, "not implemented: {}", feature)?
            }
            ErrorKind::PemFileOperation => {
                fmt.write_str("PEM file read operation failed")?
            }
            ErrorKind::PoolHasBusyConnections => fmt.write_str(
                "pool cannot be closed because connections are busy",
            )?,
            ErrorKind::PoolIncrementZero => fmt.write_str(
                "dynamically sized pools must have a non-zero increment",
            )?,
            ErrorKind::PoolMaxLessThanMin => fmt.write_str(
                "pool max connections must be greater or equal to pool min \
                 connections",
            )?,
            ErrorKind::PoolNotOpen => fmt.write_str("pool is not open")?,
            ErrorKind::ServerVersionNotSupported => fmt.write_str(
                "connections to this database server are not supported",
            )?,
            ErrorKind::StreamOperation => {
                fmt.write_str("stream operation failed")?
            }
            ErrorKind::TlsOperation => {
                fmt.write_str("TLS operation failed")?
            }
            ErrorKind::TnsAliasNotFound(file_name, alias) => {
                write!(fmt, "unable to find \"{}\" in {}", alias, file_name)?
            }
            ErrorKind::UnableToRecover => fmt.write_str(
                "unable to recover from error: connection has been closed",
            )?,
            ErrorKind::UnexpectedError => fmt.write_str("unexpected error")?,
            ErrorKind::UnexpectedNegativeInteger => fmt.write_str(
                "internal error: read a negative integer when expecting a \
                 positive integer",
            )?,
            ErrorKind::UnexpectedRefuse(connection_id) => write!(
                fmt,
                "the listener refused the connection but an \
                 unexpected error format was returned \
                 (CONNECTION_ID={connection_id})",
            )?,
            ErrorKind::UnexpectedResult => {
                fmt.write_str("unexpected result")?
            }
            ErrorKind::UnknownServerSidePiggyback(opcode) => write!(
                fmt,
                "internal error: unknown server-side piggyback opcode {opcode}"
            )?,
            ErrorKind::UnknownTtcMessageType(ttc_message_type) => write!(
                fmt,
                "internal error: unknown TTC message type {}",
                ttc_message_type
            )?,
            ErrorKind::UnsupportedArrowType(arrow_type) => {
                write!(fmt, "binding Arow type {}", arrow_type)?
            }
            ErrorKind::UnsupportedConversion(from_type, to_type) => write!(
                fmt,
                "unsupported conversion from {} to {}",
                from_type, to_type
            )?,
            ErrorKind::UnsupportedDbType(db_type) => {
                write!(fmt, "unsupported database type {}", db_type.name())?
            }
            ErrorKind::UnsupportedDeepDataSecurityFeature => fmt.write_str(
                "database does not support the Oracle Deep Data Security feature",
            )?,
            ErrorKind::UnsupportedOsonNodeType(node_type) => {
                write!(fmt, "unsupported OSON node type {node_type}")?
            }
            ErrorKind::UnsupportedOsonVersion(version) => {
                write!(fmt, "unsupported OSON version {version}")?
            }
            ErrorKind::UnsupportedVectorFormat(vector_format) => {
                write!(fmt, "unsupported vector format {}", vector_format)?
            }
            ErrorKind::UnsupportedVectorVersion(version) => {
                write!(fmt, "unsupported version: {}", version)?
            }
            ErrorKind::ValueWasNull => {
                fmt.write_str("database value was null")?
            }
            ErrorKind::WalletPasswordMissingOrInvalid(name, message) => {
                write!(
                    fmt,
                    "password for wallet file {name} is missing or invalid: \
                    {message}"
                )?
            }
            ErrorKind::WalletPrivateKeyInvalid(m) => {
                write!(fmt, "private key in wallet is invalid: {m}")?
            }
            ErrorKind::WalletUnreadable(file_name) => {
                write!(fmt, "wallet with name {file_name} unreadable")?
            }
            ErrorKind::WrongNumPositionalBinds(expected_num, actual_num) => {
                write!(
                    fmt,
                    "{expected_num} positional bind values are required but \
                     {actual_num} were provided",
                )?
            }
        };
        if let Some(ref cause) = self.0.cause {
            write!(fmt, ": {}", cause)?;
        }
        Ok(())
    }
}

impl Error {
    #[cfg(feature = "arrow")]
    fn arrow_operation(e: arrow_schema::ArrowError) -> Self {
        Self::new(ErrorKind::ArrowOperation, Some(Box::new(e)))
    }

    fn call_timeout_exceeded() -> Error {
        Error::new(ErrorKind::CallTimeoutExceeded, None)
    }

    fn new(
        kind: ErrorKind,
        cause: Option<Box<dyn error::Error + Sync + Send>>,
    ) -> Error {
        Error(Box::new(ErrorInner { kind, cause }))
    }

    fn pem_file_operation(e: rustls::pki_types::pem::Error) -> Error {
        Error::new(ErrorKind::PemFileOperation, Some(Box::new(e)))
    }

    fn stream_operation(e: io::Error) -> Error {
        Error::new(ErrorKind::StreamOperation, Some(Box::new(e)))
    }

    pub(crate) fn column_truncated(
        col_value_size: usize,
        actual_size: usize,
    ) -> Error {
        Error::new(
            ErrorKind::ColumnTruncated(col_value_size, actual_size),
            None,
        )
    }

    pub(crate) fn db_error(message: String) -> Error {
        Error::new(ErrorKind::DbError(message), None)
    }

    pub(crate) fn dead_connection() -> Error {
        Error::new(ErrorKind::DeadConnection, None)
    }

    pub(crate) fn different_types(
        initial_db_type: &'static DbType,
        subsequent_db_type: &'static DbType,
    ) -> Error {
        Error::new(
            ErrorKind::DifferentTypes(initial_db_type, subsequent_db_type),
            None,
        )
    }

    /// Creates an error for attempting Deep Data Security over a non-TCPS
    /// connection.
    pub(crate) fn end_user_security_context_requires_tcps() -> Error {
        Error::new(ErrorKind::EndUserSecurityContextRequiresTcps, None)
    }

    pub(crate) fn ifile_cycle_detected(
        including_file_name: String,
        included_file_name: String,
    ) -> Error {
        Error::new(
            ErrorKind::IfileCycleDetected(
                including_file_name,
                included_file_name,
            ),
            None,
        )
    }

    pub(crate) fn integer_too_large(
        max_size: usize,
        actual_size: usize,
    ) -> Error {
        Error::new(ErrorKind::IntegerTooLarge(max_size, actual_size), None)
    }

    pub(crate) fn invalid_bind_name(name: &str) -> Error {
        Error::new(ErrorKind::InvalidBindName(name.to_string()), None)
    }

    pub(crate) fn invalid_column_index(index: usize) -> Error {
        Error::new(ErrorKind::InvalidColumnIndex(index), None)
    }

    pub(crate) fn invalid_connect_string(
        connect_string: &str,
        reason: &str,
    ) -> Error {
        Error::new(
            ErrorKind::InvalidConnectString(
                connect_string.to_string(),
                reason.to_string(),
            ),
            None,
        )
    }

    pub(crate) fn invalid_descriptor_node(
        key: String,
        expected_type: String,
    ) -> Error {
        Error::new(ErrorKind::InvalidDescriptorNode(key, expected_type), None)
    }

    pub(crate) fn invalid_encoded_vector() -> Error {
        Error::new(ErrorKind::InvalidEncodedVector, None)
    }

    /// Creates an error for invalid Deep Data Security context input.
    pub(crate) fn invalid_end_user_security_context(message: &str) -> Error {
        Error::new(
            ErrorKind::InvalidEndUserSecurityContext(message.to_string()),
            None,
        )
    }

    /// Creates an error for a Deep Data Security context payload that exceeds
    /// protocol limits.
    pub(crate) fn invalid_end_user_security_context_length(
        length: usize,
    ) -> Error {
        Error::new(
            ErrorKind::InvalidEndUserSecurityContextLength(length),
            None,
        )
    }

    pub(crate) fn invalid_network_name(value: String) -> Error {
        Error::new(ErrorKind::InvalidNetworkName(value), None)
    }

    pub(crate) fn invalid_oracle_number(v: String) -> Error {
        Error::new(ErrorKind::InvalidOracleNumber(v), None)
    }

    pub(crate) fn invalid_oson_encoded_bytes() -> Error {
        Error::new(ErrorKind::InvalidOsonEncodedBytes, None)
    }

    pub(crate) fn invalid_service_name(
        connection_id: String,
        service_name: String,
        host: String,
        port: u16,
    ) -> Error {
        Error::new(
            ErrorKind::InvalidServiceName(
                connection_id,
                service_name,
                host,
                port,
            ),
            None,
        )
    }

    pub(crate) fn invalid_sid(
        connection_id: String,
        sid: String,
        host: String,
        port: u16,
    ) -> Error {
        Error::new(ErrorKind::InvalidSid(connection_id, sid, host, port), None)
    }

    /// Returns a boolean indicating if the error is a call timeout exceeded
    /// error.
    pub(crate) fn is_call_timeout_exceeded(&self) -> bool {
        self.kind() == &ErrorKind::CallTimeoutExceeded
    }

    /// Returns a boolean indicating if the error is an out of data error.
    pub(crate) fn is_out_of_data(&self) -> bool {
        self.kind() == &ErrorKind::OutOfData
    }

    pub(crate) fn listener_refused_connection(
        connection_id: String,
        host: String,
        port: u16,
        error_num: usize,
    ) -> Error {
        Error::new(
            ErrorKind::ListenerRefusedConnection(
                connection_id,
                host,
                port,
                error_num,
            ),
            None,
        )
    }

    pub(crate) fn missing_bind_value(name: &str) -> Error {
        Error::new(ErrorKind::MissingBindValue(name.to_string()), None)
    }

    pub(crate) fn name_has_embedded_quotes() -> Error {
        Error::new(ErrorKind::NameHasEmbeddedQuotes, None)
    }

    pub(crate) fn no_config_dir() -> Error {
        Error::new(ErrorKind::NoConfigDir, None)
    }

    pub(crate) fn no_connect_string() -> Error {
        Error::new(ErrorKind::NoConnectString, None)
    }

    pub(crate) fn no_credentials() -> Error {
        Error::new(ErrorKind::NoCredentials, None)
    }

    pub(crate) fn no_data_found() -> Error {
        Error::new(ErrorKind::NoDataFound, None)
    }

    pub(crate) fn not_connected() -> Error {
        Error::new(ErrorKind::NotConnected, None)
    }

    pub(crate) fn not_implemented(feature: String) -> Error {
        Error::new(ErrorKind::NotImplemented(feature), None)
    }

    pub(crate) fn out_of_data() -> Error {
        Error::new(ErrorKind::OutOfData, None)
    }

    pub(crate) fn out_of_range(m: impl Into<String>) -> Error {
        Error::new(ErrorKind::OutOfRange(m.into()), None)
    }

    pub(crate) fn parse_error(s: String, p: usize) -> Error {
        Error::new(ErrorKind::ParseError(s, p), None)
    }

    pub(crate) fn pool_has_busy_connections() -> Error {
        Error::new(ErrorKind::PoolHasBusyConnections, None)
    }

    pub(crate) fn pool_increment_zero() -> Error {
        Error::new(ErrorKind::PoolIncrementZero, None)
    }

    pub(crate) fn pool_max_less_than_min() -> Error {
        Error::new(ErrorKind::PoolMaxLessThanMin, None)
    }

    pub(crate) fn pool_not_open() -> Error {
        Error::new(ErrorKind::PoolNotOpen, None)
    }

    pub(crate) fn server_version_not_supported() -> Error {
        Error::new(ErrorKind::ServerVersionNotSupported, None)
    }

    pub(crate) fn tls_operation(e: rustls::Error) -> Error {
        Error::new(ErrorKind::TlsOperation, Some(Box::new(e)))
    }

    pub(crate) fn tns_alias_not_found(
        file_name: String,
        alias: String,
    ) -> Error {
        Error::new(ErrorKind::TnsAliasNotFound(file_name, alias), None)
    }

    pub(crate) fn unable_to_recover() -> Error {
        Error::new(ErrorKind::UnableToRecover, None)
    }

    pub(crate) fn unexpected_error(
        e: Box<dyn error::Error + Sync + Send>,
    ) -> Error {
        Error::new(ErrorKind::UnexpectedError, Some(e))
    }

    pub(crate) fn unexpected_negative_integer() -> Error {
        Error::new(ErrorKind::UnexpectedNegativeInteger, None)
    }

    pub(crate) fn unexpected_refuse(connection_id: String) -> Error {
        Error::new(ErrorKind::UnexpectedRefuse(connection_id), None)
    }

    pub(crate) fn unexpected_result() -> Error {
        Error::new(ErrorKind::UnexpectedResult, None)
    }

    pub(crate) fn unknown_server_side_piggyback(opcode: u8) -> Error {
        Error::new(ErrorKind::UnknownServerSidePiggyback(opcode), None)
    }

    pub(crate) fn unknown_ttc_message_type(ttc_message_type: u8) -> Error {
        Error::new(ErrorKind::UnknownTtcMessageType(ttc_message_type), None)
    }

    #[cfg(feature = "arrow")]
    pub(crate) fn unsupported_arrow_type(arrow_type: String) -> Error {
        Error::new(ErrorKind::UnsupportedArrowType(arrow_type), None)
    }

    pub(crate) fn unsupported_conversion(
        from_type: &str,
        to_type: &str,
    ) -> Error {
        Error::new(
            ErrorKind::UnsupportedConversion(
                from_type.to_string(),
                to_type.to_string(),
            ),
            None,
        )
    }

    pub(crate) fn unsupported_db_type(db_type: &'static DbType) -> Error {
        Error::new(ErrorKind::UnsupportedDbType(db_type), None)
    }

    /// Creates an error for a server that does not support Deep Data Security.
    pub(crate) fn unsupported_deep_data_security_feature() -> Error {
        Error::new(ErrorKind::UnsupportedDeepDataSecurityFeature, None)
    }

    pub(crate) fn unsupported_oson_node_type(node_type: u8) -> Error {
        Error::new(ErrorKind::UnsupportedOsonNodeType(node_type), None)
    }

    pub(crate) fn unsupported_oson_version(version: u8) -> Error {
        Error::new(ErrorKind::UnsupportedOsonVersion(version), None)
    }

    pub(crate) fn unsupported_vector_format(vector_format: u8) -> Error {
        Error::new(ErrorKind::UnsupportedVectorFormat(vector_format), None)
    }

    pub(crate) fn unsupported_vector_version(version: u8) -> Error {
        Error::new(ErrorKind::UnsupportedVectorVersion(version), None)
    }

    pub(crate) fn value_was_null() -> Error {
        Error::new(ErrorKind::ValueWasNull, None)
    }

    pub(crate) fn wallet_password_missing_or_invalid(
        name: String,
        message: String,
    ) -> Error {
        Error::new(
            ErrorKind::WalletPasswordMissingOrInvalid(name, message),
            None,
        )
    }

    pub(crate) fn wallet_missing(e: io::Error, file_name: String) -> Error {
        Error::new(ErrorKind::WalletUnreadable(file_name), Some(Box::new(e)))
    }

    pub(crate) fn wallet_private_key_invalid(reason: String) -> Error {
        Error::new(ErrorKind::WalletPrivateKeyInvalid(reason), None)
    }

    pub(crate) fn wrong_num_positional_binds(
        expected_num: usize,
        actual_num: usize,
    ) -> Error {
        Error::new(
            ErrorKind::WrongNumPositionalBinds(expected_num, actual_num),
            None,
        )
    }

    /// Returns the kind of error.
    pub fn kind(&self) -> &ErrorKind {
        &self.0.kind
    }
}
