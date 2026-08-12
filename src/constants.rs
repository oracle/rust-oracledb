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
// constants.rs
//
// Defines the constants used by the package.
//-----------------------------------------------------------------------------

// driver name
pub const DRIVER_NAME: &str = "rust-oracledb";

// authorization modes (multiple values can be selected by OR'ing)
pub const AUTH_MODE_DEFAULT: u8 = 0x00;
pub const AUTH_MODE_SYSASM: u8 = 0x01;
pub const AUTH_MODE_SYSBKP: u8 = 0x02;
pub const AUTH_MODE_SYSDBA: u8 = 0x04;
pub const AUTH_MODE_SYSDGD: u8 = 0x08;
pub const AUTH_MODE_SYSKMT: u8 = 0x10;
pub const AUTH_MODE_SYSOPER: u8 = 0x20;
pub const AUTH_MODE_SYSRAC: u8 = 0x40;

// pool purity values
pub const PURITY_NEW: u8 = 1;
pub const PURITY_SELF: u8 = 2;

// Oracle type numbers
pub const ORA_TYPE_NUM_BFILE: u16 = 114;
pub const ORA_TYPE_NUM_BINARY_DOUBLE: u16 = 101;
pub const ORA_TYPE_NUM_BINARY_FLOAT: u16 = 100;
pub const ORA_TYPE_NUM_BINARY_INTEGER: u16 = 3;
pub const ORA_TYPE_NUM_BLOB: u16 = 113;
pub const ORA_TYPE_NUM_BOOLEAN: u16 = 252;
pub const ORA_TYPE_NUM_CHAR: u16 = 96;
pub const ORA_TYPE_NUM_CLOB: u16 = 112;
pub const ORA_TYPE_NUM_CURSOR: u16 = 102;
pub const ORA_TYPE_NUM_DATE: u16 = 12;
pub const ORA_TYPE_NUM_INTERVAL_DS: u16 = 183;
pub const ORA_TYPE_NUM_INTERVAL_YM: u16 = 182;
pub const ORA_TYPE_NUM_JSON: u16 = 119;
pub const ORA_TYPE_NUM_LONG: u16 = 8;
pub const ORA_TYPE_NUM_LONG_RAW: u16 = 24;
pub const ORA_TYPE_NUM_NUMBER: u16 = 2;
pub const ORA_TYPE_NUM_OBJECT: u16 = 109;
pub const ORA_TYPE_NUM_RAW: u16 = 23;
pub const ORA_TYPE_NUM_ROWID: u16 = 11;
pub const ORA_TYPE_NUM_TIMESTAMP: u16 = 180;
pub const ORA_TYPE_NUM_TIMESTAMP_LTZ: u16 = 231;
pub const ORA_TYPE_NUM_TIMESTAMP_TZ: u16 = 181;
pub const ORA_TYPE_NUM_UROWID: u16 = 208;
pub const ORA_TYPE_NUM_VARCHAR: u16 = 1;
pub const ORA_TYPE_NUM_VECTOR: u16 = 127;

// Oracle type buffer sizes
pub const ORA_TYPE_SIZE_BINARY_DOUBLE: usize = 8;
pub const ORA_TYPE_SIZE_BINARY_FLOAT: usize = 4;
pub const ORA_TYPE_SIZE_BOOLEAN: usize = 4;
pub const ORA_TYPE_SIZE_DATE: usize = 7;
pub const ORA_TYPE_SIZE_INTERVAL_DS: usize = 11;
pub const ORA_TYPE_SIZE_INTERVAL_YM: usize = 5;
pub const ORA_TYPE_SIZE_LOB: usize = 112;
pub const ORA_TYPE_SIZE_NUMBER: usize = 22;
pub const ORA_TYPE_SIZE_ROWID: usize = 18;
pub const ORA_TYPE_SIZE_TIMESTAMP: usize = 11;
pub const ORA_TYPE_SIZE_TIMESTAMP_TZ: usize = 13;

// database type numbers
pub const DB_TYPE_NUM_BFILE: u16 = 2020;
pub const DB_TYPE_NUM_BINARY_DOUBLE: u16 = 2008;
pub const DB_TYPE_NUM_BINARY_FLOAT: u16 = 2007;
pub const DB_TYPE_NUM_BINARY_INTEGER: u16 = 2009;
pub const DB_TYPE_NUM_BLOB: u16 = 2019;
pub const DB_TYPE_NUM_BOOLEAN: u16 = 2022;
pub const DB_TYPE_NUM_CHAR: u16 = 2003;
pub const DB_TYPE_NUM_CLOB: u16 = 2017;
pub const DB_TYPE_NUM_CURSOR: u16 = 2021;
pub const DB_TYPE_NUM_DATE: u16 = 2011;
pub const DB_TYPE_NUM_INTERVAL_DS: u16 = 2015;
pub const DB_TYPE_NUM_INTERVAL_YM: u16 = 2016;
pub const DB_TYPE_NUM_JSON: u16 = 2027;
pub const DB_TYPE_NUM_LONG_NVARCHAR: u16 = 2031;
pub const DB_TYPE_NUM_LONG_RAW: u16 = 2025;
pub const DB_TYPE_NUM_LONG_VARCHAR: u16 = 2024;
pub const DB_TYPE_NUM_NCHAR: u16 = 2004;
pub const DB_TYPE_NUM_NCLOB: u16 = 2018;
pub const DB_TYPE_NUM_NUMBER: u16 = 2010;
pub const DB_TYPE_NUM_NVARCHAR: u16 = 2002;
pub const DB_TYPE_NUM_OBJECT: u16 = 2023;
pub const DB_TYPE_NUM_RAW: u16 = 2006;
pub const DB_TYPE_NUM_ROWID: u16 = 2005;
pub const DB_TYPE_NUM_TIMESTAMP: u16 = 2012;
pub const DB_TYPE_NUM_TIMESTAMP_LTZ: u16 = 2014;
pub const DB_TYPE_NUM_TIMESTAMP_TZ: u16 = 2013;
pub const DB_TYPE_NUM_UNKNOWN: u16 = 0;
pub const DB_TYPE_NUM_UROWID: u16 = 2030;
pub const DB_TYPE_NUM_VARCHAR: u16 = 2001;
pub const DB_TYPE_NUM_VECTOR: u16 = 2033;
pub const DB_TYPE_NUM_XMLTYPE: u16 = 2032;

// character set forms
pub const CS_FORM_NONE: u8 = 0;
pub const CS_FORM_IMPLICIT: u8 = 1;
pub const CS_FORM_NCHAR: u8 = 2;

// packet types
pub const PACKET_TYPE_ACCEPT: u8 = 2;
pub const PACKET_TYPE_CONNECT: u8 = 1;
pub const PACKET_TYPE_CONTROL: u8 = 14;
pub const PACKET_TYPE_DATA: u8 = 6;
pub const PACKET_TYPE_MARKER: u8 = 12;
pub const PACKET_TYPE_REFUSE: u8 = 4;
pub const PACKET_TYPE_REDIRECT: u8 = 5;
pub const PACKET_TYPE_RESEND: u8 = 11;

// packet flags
pub const PACKET_FLAGS_REDIRECT: u8 = 0x04;
pub const PACKET_FLAGS_TLS_RENEG: u8 = 0x08;

// marker types
pub const _MARKER_TYPE_BREAK: u8 = 1;
pub const MARKER_TYPE_RESET: u8 = 2;
pub const MARKER_TYPE_INTERRUPT: u8 = 3;

// protocol version constants
pub const PROTOCOL_VERSION_MIN: u16 = 300;
pub const PROTOCOL_VERSION_12: u16 = 315;
pub const PROTOCOL_VERSION_18: u16 = 318;
pub const PROTOCOL_VERSION_23: u16 = 319;

// database error constants
pub const DB_ERR_NUM_NO_DATA_FOUND: usize = 1403;
pub const DB_ERR_NUM_INVALID_SERVICE_NAME: usize = 12514;
pub const DB_ERR_NUM_INVALID_SID: usize = 12505;
pub const DB_ERR_NUM_SESSION_SHUTDOWN: usize = 12572;

// TTC field version constants
pub const _TTC_FIELD_VERSION_11_2: u8 = 6;
pub const _TTC_FIELD_VERSION_12_1: u8 = 7;
pub const TTC_FIELD_VERSION_12_2: u8 = 8;
pub const TTC_FIELD_VERSION_12_2_EXT1: u8 = 9;
pub const _TTC_FIELD_VERSION_18_1: u8 = 10;
pub const TTC_FIELD_VERSION_18_1_EXT_1: u8 = 11;
pub const _TTC_FIELD_VERSION_19_1: u8 = 12;
pub const TTC_FIELD_VERSION_19_1_EXT_1: u8 = 13;
pub const TTC_FIELD_VERSION_20_1: u8 = 14;
pub const _TTC_FIELD_VERSION_20_1_EXT_1: u8 = 15;
pub const _TTC_FIELD_VERSION_21_1: u8 = 16;
pub const TTC_FIELD_VERSION_23_1: u8 = 17;
pub const TTC_FIELD_VERSION_23_1_EXT_1: u8 = 18;
pub const _TTC_FIELD_VERSION_23_1_EXT_2: u8 = 19;
pub const TTC_FIELD_VERSION_23_1_EXT_3: u8 = 20;
pub const _TTC_FIELD_VERSION_23_1_EXT_4: u8 = 21;
pub const _TTC_FIELD_VERSION_23_1_EXT_5: u8 = 22;
pub const _TTC_FIELD_VERSION_23_3_EXT_6: u8 = 23;
pub const TTC_FIELD_VERSION_23_4: u8 = 24;
pub const FAST_AUTH_TTC_FIELD_VERSION: u8 = TTC_FIELD_VERSION_19_1_EXT_1;

// TTC message types
pub const TTC_MSG_TYPE_PROTOCOL: u8 = 1;
pub const TTC_MSG_TYPE_DATA_TYPES: u8 = 2;
pub const TTC_MSG_TYPE_FUNCTION: u8 = 3;
pub const TTC_MSG_TYPE_ERROR: u8 = 4;
pub const TTC_MSG_TYPE_ROW_HEADER: u8 = 6;
pub const TTC_MSG_TYPE_ROW_DATA: u8 = 7;
pub const TTC_MSG_TYPE_PARAMETER: u8 = 8;
pub const TTC_MSG_TYPE_STATUS: u8 = 9;
pub const TTC_MSG_TYPE_IO_VECTOR: u8 = 11;
pub const TTC_MSG_TYPE_LOB_DATA: u8 = 14;
pub const TTC_MSG_TYPE_WARNING: u8 = 15;
pub const TTC_MSG_TYPE_DESCRIBE_INFO: u8 = 16;
pub const TTC_MSG_TYPE_PIGGYBACK: u8 = 17;
pub const _TTC_MSG_TYPE_FLUSH_OUT_BINDS: u8 = 19;
pub const TTC_MSG_TYPE_BIT_VECTOR: u8 = 21;
pub const TTC_MSG_TYPE_SERVER_SIDE_PIGGYBACK: u8 = 23;
pub const _TTC_MSG_TYPE_ONEWAY_FN: u8 = 26;
pub const _TTC_MSG_TYPE_IMPLICIT_RESULTSET: u8 = 27;
pub const _TTC_MSG_TYPE_RENEGOTIATE: u8 = 28;
pub const TTC_MSG_TYPE_END_OF_RESPONSE: u8 = 29;
pub const _TTC_MSG_TYPE_TOKEN: u8 = 33;
pub const TTC_MSG_TYPE_FAST_AUTH: u8 = 34;

// server-side piggyback opcodes
pub const TTC_SERVER_PIGGYBACK_QUERY_CACHE_INVALIDATION: u8 = 1;
pub const TTC_SERVER_PIGGYBACK_OS_PID_MTS: u8 = 2;
pub const TTC_SERVER_PIGGYBACK_TRACE_EVENT: u8 = 3;
pub const TTC_SERVER_PIGGYBACK_SESS_RET: u8 = 4;
pub const TTC_SERVER_PIGGYBACK_SYNC: u8 = 5;
pub const TTC_SERVER_PIGGYBACK_LTXID: u8 = 7;
pub const TTC_SERVER_PIGGYBACK_AC_REPLAY_CONTEXT: u8 = 8;
pub const TTC_SERVER_PIGGYBACK_EXT_SYNC: u8 = 9;
pub const TTC_SERVER_PIGGYBACK_SESS_SIGNATURE: u8 = 10;

// TTC RPCs
pub const TTC_RPC_AUTH_PHASE_ONE: u8 = 118;
pub const TTC_RPC_AUTH_PHASE_TWO: u8 = 115;
pub const TTC_RPC_CLOSE_CURSORS: u8 = 105;
pub const TTC_RPC_COMMIT: u8 = 14;
pub const TTC_RPC_EXECUTE: u8 = 94;
pub const TTC_RPC_FETCH: u8 = 5;
pub const TTC_RPC_LOB_OP: u8 = 96;
pub const _TTC_RPC_AQ_ENQ: u8 = 121;
pub const _TTC_RPC_AQ_DEQ: u8 = 122;
pub const _TTC_RPC_ARRAY_AQ: u8 = 145;
pub const TTC_RPC_LOGOFF: u8 = 9;
pub const TTC_RPC_PING: u8 = 147;
pub const _TTC_RPC_PIPELINE_BEGIN: u8 = 199;
pub const _TTC_RPC_PIPELINE_END: u8 = 200;
pub const TTC_RPC_ROLLBACK: u8 = 15;
pub const TTC_RPC_SET_END_TO_END_ATTR: u8 = 135;
pub const TTC_RPC_REEXECUTE: u8 = 4;
pub const TTC_RPC_REEXECUTE_AND_FETCH: u8 = 78;
pub const _TTC_RPC_SESSION_GET: u8 = 162;
pub const _TTC_RPC_SESSION_RELEASE: u8 = 163;
pub const _TTC_RPC_SESSION_STATE: u8 = 176;
pub const _TTC_RPC_SET_SCHEMA: u8 = 152;
pub const _TTC_RPC_TPC_TXN_SWITCH: u8 = 103;
pub const _TTC_RPC_TPC_TXN_CHANGE_STATE: u8 = 104;
pub const TTC_RPC_END_USER_SECURITY_CONTEXT: u8 = 205;

// TTC authentication modes
pub const TTC_AUTH_MODE_LOGON: u32 = 0x00000001;
pub const TTC_AUTH_MODE_CHANGE_PASSWORD: u32 = 0x00000002;
pub const TTC_AUTH_MODE_SYSDBA: u32 = 0x00000020;
pub const TTC_AUTH_MODE_SYSOPER: u32 = 0x00000040;
pub const TTC_AUTH_MODE_WITH_PASSWORD: u32 = 0x00000100;
pub const TTC_AUTH_MODE_SYSASM: u32 = 0x00400000;
pub const TTC_AUTH_MODE_SYSBKP: u32 = 0x01000000;
pub const TTC_AUTH_MODE_SYSDGD: u32 = 0x02000000;
pub const TTC_AUTH_MODE_SYSKMT: u32 = 0x04000000;
pub const TTC_AUTH_MODE_SYSRAC: u32 = 0x08000000;
pub const _TTC_AUTH_MODE_IAM_TOKEN: u32 = 0x20000000;

// TTC execute options
pub const TTC_EXEC_OPTION_PARSE: u32 = 0x01;
pub const TTC_EXEC_OPTION_BIND: u32 = 0x08;
pub const TTC_EXEC_OPTION_DEFINE: u32 = 0x10;
pub const TTC_EXEC_OPTION_EXECUTE: u32 = 0x20;
pub const TTC_EXEC_OPTION_FETCH: u32 = 0x40;
pub const TTC_EXEC_OPTION_COMMIT: u32 = 0x100;
pub const TTC_EXEC_OPTION_PLSQL_BIND: u32 = 0x400;
pub const TTC_EXEC_OPTION_NOT_PLSQL: u32 = 0x8000;
pub const TTC_EXEC_OPTION_DESCRIBE: u32 = 0x20000;
pub const _TTC_EXEC_OPTION_NO_COMPRESSED_FETCH: u32 = 0x40000;
pub const TTC_EXEC_OPTION_BATCH_ERRORS: u32 = 0x80000;

// TTC execute flags
pub const TTC_EXEC_FLAGS_DML_ROWCOUNTS: u32 = 0x4000;
pub const TTC_EXEC_FLAGS_IMPLICIT_RESULTSET: u32 = 0x8000;
pub const _TTC_EXEC_FLAGS_SCROLLABLE: u32 = 0x02;

// TTC set end-to-end attributes flags
pub const TTC_END_TO_END_FLAGS_ACTION: u32 = 0x0010;
pub const TTC_END_TO_END_FLAGS_CLIENT_IDENTIFIER: u32 = 0x0001;
pub const TTC_END_TO_END_FLAGS_CLIENT_INFO: u32 = 0x0100;
pub const TTC_END_TO_END_FLAGS_DB_OP: u32 = 0x0200;
pub const TTC_END_TO_END_FLAGS_MODULE: u32 = 0x0008;

// TTC control packet types
pub const TTC_CONTROL_TYPE_INBAND_NOTIF: u16 = 8;

// TTC LOB locator flags (byte 1)
pub const TTC_LOB_LOC_FLAGS_BLOB: u8 = 0x01;
pub const TTC_LOB_LOC_FLAGS_VALUE_BASED: u8 = 0x20;
pub const TTC_LOB_LOC_FLAGS_ABSTRACT: u8 = 0x40;

// TTC LOB locator flags (byte 2)
pub const TTC_LOB_LOC_FLAGS_INIT: u8 = 0x08;

// TTC LOB operations
pub const TTC_LOB_OP_READ: u32 = 0x0002;
pub const TTC_LOB_OP_GET_LENGTH: u32 = 0x0001;
pub const TTC_LOB_OP_TRIM: u32 = 0x0020;
pub const TTC_LOB_OP_WRITE: u32 = 0x0040;
pub const TTC_LOB_OP_GET_CHUNK_SIZE: u32 = 0x4000;
pub const TTC_LOB_OP_OPEN: u32 = 0x8000;
pub const TTC_LOB_OP_CLOSE: u32 = 0x10000;
pub const TTC_LOB_OP_IS_OPEN: u32 = 0x11000;
pub const _TTC_LOB_OP_FREE_TEMP: u32 = 0x0111;
pub const _TTC_LOB_OP_FILE_EXISTS: u32 = 0x0800;
pub const _TTC_LOB_OP_FILE_OPEN: u32 = 0x0100;
pub const _TTC_LOB_OP_FILE_CLOSE: u32 = 0x0200;
pub const _TTC_LOB_OP_FILE_ISOPEN: u32 = 0x0400;

// LOB open modes
pub const TTC_LOB_OPEN_READ_WRITE: u64 = 2;
pub const TTC_LOB_OPEN_READ_ONLY: u64 = 11;

// LOB locator constants (offsets/flags)
pub const TTC_LOB_LOC_OFFSET_FLAG_3: usize = 6;
pub const TTC_LOB_LOC_OFFSET_FLAG_4: usize = 7;
pub const TTC_LOB_LOC_FLAGS_VAR_LENGTH_CHARSET: u8 = 0x80;
pub const TTC_LOB_LOC_FLAGS_LITTLE_ENDIAN: u8 = 0x40;

// TTC re-execute options
pub const _TTC_REEXEC_OPTION_COMMIT: u32 = 0x1;

// TTC keyword numbers
pub const TTC_KEYWORD_NUM_CURRENT_SCHEMA: u16 = 168;
pub const TTC_KEYWORD_NUM_EDITION: u16 = 172;

// TTC bind flags
pub const TTC_BIND_FLAG_USE_INDICATORS: u8 = 0x01;

// TTC bind directions
pub const TTC_BIND_DIR_INPUT: u8 = 0x20;
pub const TTC_BIND_DIR_OUTPUT: u8 = 0x10;

// character sets
pub const CHARSET_ID_UTF8: u16 = 873;

// verifier types
pub const _VERIFIER_TYPE_11G_1: u16 = 0xb152;
pub const _VERIFIER_TYPE_11G_2: u16 = 0x1b25;
pub const _VERIFIER_TYPE_12C: u16 = 0x4815;

// data flags
pub const TTC_DATA_FLAGS_EOF: u16 = 0x0040;
pub const TTC_DATA_FLAGS_END_OF_RESPONSE: u16 = 0x2000;

// timezone offset constants
pub const TZ_HOUR_OFFSET: u8 = 20;
pub const TZ_MINUTE_OFFSET: u8 = 60;

// duration offset constants
pub const DURATION_MID: u32 = 0x80000000;
pub const DURATION_OFFSET: u8 = 60;

// OracleNumber constants
pub const ORA_NUM_MAX_DIGITS: usize = 40;

// other constants
pub const TTC_CHUNK_SIZE: usize = 32767;
pub const TTC_MAX_SHORT_LENGTH: u8 = 252;
pub const TTC_LONG_LENGTH_INDICATOR: u8 = 254;
pub const TTC_NULL_LENGTH_INDICATOR: u8 = 255;
