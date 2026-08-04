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
// capabilities.rs
//
// Defines the structure containing the capabilities supported by the client
// and the server.
//-----------------------------------------------------------------------------

use crate::constants;

// capability array sizes
const COMPILE_CAPS_SIZE: usize = 53;
const RUNTIME_CAPS_SIZE: usize = 11;

// compile capability indexes
const CCAP_IX_SQL_VERSION: usize = 0;
const CCAP_IX_LOGON_TYPES: usize = 4;
const CCAP_IX_FEATURE_BACKPORT: usize = 5;
const CCAP_IX_FIELD_VERSION: usize = 7;
const CCAP_IX_SERVER_DEFINE_CONV: usize = 8;
const CCAP_IX_DEQUEUE_WITH_SELECTOR: usize = 9;
const CCAP_IX_TTC1: usize = 15;
const CCAP_IX_OCI1: usize = 16;
const CCAP_IX_TDS_VERSION: usize = 17;
const CCAP_IX_RPC_VERSION: usize = 18;
const CCAP_IX_RPC_SIG: usize = 19;
const CCAP_IX_DBF_VERSION: usize = 21;
const CCAP_IX_LOB: usize = 23;
const CCAP_IX_TTC2: usize = 26;
const CCAP_IX_UB2_DTY: usize = 27;
const CCAP_IX_OCI2: usize = 31;
const CCAP_IX_CLIENT_FN: usize = 34;
const CCAP_IX_TTC3: usize = 37;
const CCAP_IX_SESS_SIGNATURE_VERSION: usize = 39;
const CCAP_IX_TTC4: usize = 40;
const CCAP_IX_LOB2: usize = 42;
const CCAP_IX_TTC5: usize = 44;
const CCAP_IX_FEATURE_BACKPORT2: usize = 45;
const CCAP_IX_VECTOR_FEATURES: usize = 52;

// compile capability values
const CCAP_VAL_O5LOGON: u8 = 8;
const CCAP_VAL_O5LOGON_NP: u8 = 2;
const CCAP_VAL_O7LOGON: u8 = 32;
const CCAP_VAL_O8LOGON_LONG_IDENTIFIER: u8 = 64;
const CCAP_VAL_O9LOGON_LONG_PASSWORD: u8 = 0x80;
const CCAP_VAL_CTB_IMPLICIT_POOL: u8 = 0x08;
const CCAP_VAL_END_OF_CALL_STATUS: u8 = 0x01;
const CCAP_VAL_IND_RCD: u8 = 0x08;
const CCAP_VAL_FAST_BVEC: u8 = 0x20;
const CCAP_VAL_FAST_SESSION_PROPAGATE: u8 = 0x10;
const CCAP_VAL_APP_CTX_PIGGYBACK: u8 = 0x80;
const CCAP_VAL_TDS_VERSION_MAX: u8 = 3;
const CCAP_VAL_RPC_VERSION_MAX: u8 = 7;
const CCAP_VAL_RPC_SIG_VALUE: u8 = 3;
const CCAP_VAL_DBF_VERSION_MAX: u8 = 1;
const CCAP_VAL_LTXID: u8 = 0x08;
const CCAP_VAL_IMPLICIT_RESULTS: u8 = 0x10;
const CCAP_VAL_BIG_CHUNK_CLR: u8 = 0x20;
const CCAP_VAL_KEEP_OUT_ORDER: u8 = 0x80;
const CCAP_VAL_LOB_UB8_SIZE: u8 = 0x01;
const CCAP_VAL_LOB_ENCS: u8 = 0x02;
const CCAP_VAL_LOB_PREFETCH_DATA: u8 = 0x04;
const CCAP_VAL_LOB_TEMP_SIZE: u8 = 0x08;
const CCAP_VAL_LOB_PREFETCH_LENGTH: u8 = 0x40;
const CCAP_VAL_LOB_12C: u8 = 0x80;
const CCAP_VAL_LOB2_QUASI: u8 = 0x01;
const CCAP_VAL_LOB2_2GB_PREFETCH: u8 = 0x04;
const CCAP_VAL_DRCP: u8 = 0x10;
const CCAP_VAL_ZLNP: u8 = 0x04;
const CCAP_VAL_INBAND_NOTIFICATION: u8 = 0x04;
const CCAP_VAL_EXPLICIT_BOUNDARY: u8 = 0x40;
const CCAP_VAL_END_OF_RESPONSE: u8 = 0x20;
const CCAP_VAL_CLIENT_FN_MAX: u8 = 12;
const CCAP_VAL_VECTOR_SUPPORT: u8 = 0x08;
const CCAP_VAL_TOKEN_SUPPORTED: u8 = 0x02;
const CCAP_VAL_PIPELINING_SUPPORT: u8 = 0x04;
const CCAP_VAL_PIPELINING_BREAK: u8 = 0x10;
const CCAP_VAL_END_USER_SEC_CTX_PIGGYBACK: u8 = 0x02;
const CCAP_VAL_VECTOR_FEATURE_BINARY: u8 = 0x01;
const CCAP_VAL_VECTOR_FEATURE_SPARSE: u8 = 0x02;

// runtime capability indexes
const RCAP_IX_COMPAT: usize = 0;
const RCAP_IX_TTC: usize = 6;

// runtime capbility values
const RCAP_VAL_COMPAT_81: u8 = 2;
const RCAP_VAL_TTC_ZERO_COPY: u8 = 0x01;
const RCAP_VAL_TTC_32K: u8 = 0x04;
const RCAP_VAL_TTC_SESSION_STATE_OPS: u8 = 0x10;

// accept flags
const ACCEPT_FLAG_FAST_AUTH: u32 = 0x10000000;
const ACCEPT_FLAG_HAS_END_OF_RESPONSE: u32 = 0x02000000;

pub struct Capabilities {
    protocol_version: u16,
    ttc_field_version: u8,
    compile_caps: [u8; COMPILE_CAPS_SIZE],
    runtime_caps: [u8; RUNTIME_CAPS_SIZE],
    max_string_size: u32,
    supports_fast_auth: bool,
    supports_end_of_response: bool,
    supports_pipelining: bool,
    supports_request_boundaries: bool,
    supports_end_user_security_context: bool,
}

impl Capabilities {
    pub fn adjust_for_protocol(&mut self, protocol_version: u16, flags: u32) {
        self.protocol_version = protocol_version;
        if flags & ACCEPT_FLAG_FAST_AUTH != 0 {
            self.supports_fast_auth = true;
        }
        if protocol_version >= constants::PROTOCOL_VERSION_23
            && flags & ACCEPT_FLAG_HAS_END_OF_RESPONSE != 0
        {
            self.compile_caps[CCAP_IX_TTC4] |= CCAP_VAL_END_OF_RESPONSE;
            self.supports_end_of_response = true;
            self.supports_pipelining = true;
        }
    }

    pub fn adjust_for_server_compile_caps(&mut self, caps: &[u8]) {
        if caps[CCAP_IX_FIELD_VERSION] < self.ttc_field_version {
            self.ttc_field_version = caps[CCAP_IX_FIELD_VERSION];
            self.compile_caps[CCAP_IX_FIELD_VERSION] = self.ttc_field_version;
        }
        if caps[CCAP_IX_TTC4] & CCAP_VAL_EXPLICIT_BOUNDARY != 0 {
            self.supports_request_boundaries = true;
        }
        if caps.len() > CCAP_IX_FEATURE_BACKPORT2
            && caps[CCAP_IX_FEATURE_BACKPORT2]
                & CCAP_VAL_END_USER_SEC_CTX_PIGGYBACK
                != 0
        {
            self.supports_end_user_security_context = true;
        }
    }

    pub fn adjust_for_server_runtime_caps(&mut self, caps: &[u8]) {
        if caps[RCAP_IX_TTC] & RCAP_VAL_TTC_32K != 0 {
            self.max_string_size = 32767;
        } else {
            self.max_string_size = 4000;
        }
        if caps[RCAP_IX_TTC] & RCAP_VAL_TTC_SESSION_STATE_OPS == 0 {
            self.supports_request_boundaries = false;
        }
    }

    pub fn compile_caps(&self) -> &[u8] {
        &self.compile_caps
    }

    fn init_compile_caps(&mut self) {
        self.ttc_field_version = constants::TTC_FIELD_VERSION_23_4;
        self.compile_caps[CCAP_IX_SQL_VERSION] = 6;
        self.compile_caps[CCAP_IX_LOGON_TYPES] = CCAP_VAL_O5LOGON
            | CCAP_VAL_O5LOGON_NP
            | CCAP_VAL_O7LOGON
            | CCAP_VAL_O8LOGON_LONG_IDENTIFIER
            | CCAP_VAL_O9LOGON_LONG_PASSWORD;
        self.compile_caps[CCAP_IX_FEATURE_BACKPORT] =
            CCAP_VAL_CTB_IMPLICIT_POOL;
        self.compile_caps[CCAP_IX_FIELD_VERSION] = self.ttc_field_version;
        self.compile_caps[CCAP_IX_SERVER_DEFINE_CONV] = 1;
        self.compile_caps[CCAP_IX_DEQUEUE_WITH_SELECTOR] = 1;
        self.compile_caps[CCAP_IX_TTC1] = CCAP_VAL_FAST_BVEC
            | CCAP_VAL_END_OF_CALL_STATUS
            | CCAP_VAL_IND_RCD;
        self.compile_caps[CCAP_IX_OCI1] =
            CCAP_VAL_FAST_SESSION_PROPAGATE | CCAP_VAL_APP_CTX_PIGGYBACK;
        self.compile_caps[CCAP_IX_TDS_VERSION] = CCAP_VAL_TDS_VERSION_MAX;
        self.compile_caps[CCAP_IX_RPC_VERSION] = CCAP_VAL_RPC_VERSION_MAX;
        self.compile_caps[CCAP_IX_RPC_SIG] = CCAP_VAL_RPC_SIG_VALUE;
        self.compile_caps[CCAP_IX_DBF_VERSION] = CCAP_VAL_DBF_VERSION_MAX;
        self.compile_caps[CCAP_IX_LOB] = CCAP_VAL_LOB_UB8_SIZE
            | CCAP_VAL_LOB_ENCS
            | CCAP_VAL_LOB_PREFETCH_LENGTH
            | CCAP_VAL_LOB_TEMP_SIZE
            | CCAP_VAL_LOB_12C
            | CCAP_VAL_LOB_PREFETCH_DATA;
        self.compile_caps[CCAP_IX_UB2_DTY] = 1;
        self.compile_caps[CCAP_IX_LOB2] =
            CCAP_VAL_LOB2_QUASI | CCAP_VAL_LOB2_2GB_PREFETCH;
        self.compile_caps[CCAP_IX_TTC3] = CCAP_VAL_IMPLICIT_RESULTS
            | CCAP_VAL_BIG_CHUNK_CLR
            | CCAP_VAL_KEEP_OUT_ORDER
            | CCAP_VAL_LTXID;
        self.compile_caps[CCAP_IX_TTC2] = CCAP_VAL_ZLNP;
        self.compile_caps[CCAP_IX_OCI2] = CCAP_VAL_DRCP;
        self.compile_caps[CCAP_IX_CLIENT_FN] = CCAP_VAL_CLIENT_FN_MAX;
        self.compile_caps[CCAP_IX_SESS_SIGNATURE_VERSION] =
            constants::TTC_FIELD_VERSION_12_2;
        self.compile_caps[CCAP_IX_TTC4] =
            CCAP_VAL_INBAND_NOTIFICATION | CCAP_VAL_EXPLICIT_BOUNDARY;
        self.compile_caps[CCAP_IX_TTC5] = CCAP_VAL_VECTOR_SUPPORT
            | CCAP_VAL_TOKEN_SUPPORTED
            | CCAP_VAL_PIPELINING_SUPPORT
            | CCAP_VAL_PIPELINING_BREAK;
        self.compile_caps[CCAP_IX_VECTOR_FEATURES] =
            CCAP_VAL_VECTOR_FEATURE_BINARY | CCAP_VAL_VECTOR_FEATURE_SPARSE;
        self.compile_caps[CCAP_IX_FEATURE_BACKPORT2] =
            CCAP_VAL_END_USER_SEC_CTX_PIGGYBACK;
    }

    fn init_runtime_caps(&mut self) {
        self.runtime_caps[RCAP_IX_COMPAT] = RCAP_VAL_COMPAT_81;
        self.runtime_caps[RCAP_IX_TTC] =
            RCAP_VAL_TTC_ZERO_COPY | RCAP_VAL_TTC_32K;
    }

    pub fn new() -> Capabilities {
        let mut caps = Capabilities {
            protocol_version: 0,
            ttc_field_version: 0,
            compile_caps: [0; COMPILE_CAPS_SIZE],
            runtime_caps: [0; RUNTIME_CAPS_SIZE],
            max_string_size: 0,
            supports_fast_auth: false,
            supports_end_of_response: false,
            supports_pipelining: false,
            supports_request_boundaries: false,
            supports_end_user_security_context: false,
        };
        caps.init_compile_caps();
        caps.init_runtime_caps();
        caps
    }

    pub fn runtime_caps(&self) -> &[u8] {
        &self.runtime_caps
    }

    pub fn set_supports_end_of_response(&mut self, value: bool) -> bool {
        let orig_value = self.supports_end_of_response;
        self.supports_end_of_response = value;
        orig_value
    }

    pub fn supports_end_of_response(&self) -> bool {
        self.supports_end_of_response
    }

    /// Returns whether the negotiated capabilities allow Deep Data Security
    /// piggybacking.
    pub fn supports_end_user_security_context(&self) -> bool {
        self.supports_end_user_security_context
    }

    pub fn supports_fast_auth(&self) -> bool {
        self.supports_fast_auth
    }

    pub fn supports_ttc_field_version(&self, version: u8) -> bool {
        self.ttc_field_version >= version
    }
}
