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
// error_info.rs
//
// Defines the structure representing error information returned by the server.
//-----------------------------------------------------------------------------

use crate::client::Client;
use crate::constants;
use crate::error::Error;
use crate::response::Response;
use crate::rowid::Rowid;

pub struct ErrorInfo {
    pub(super) num: usize,
    cursor_id: u16,
    flags: u8,
    pub(super) rowcount: u64,
    message: String,
}

impl ErrorInfo {
    /// Deserializes the error information from the database response.
    pub fn deserialize(
        resp: &mut Response,
        client: &Client,
    ) -> Result<ErrorInfo, Error> {
        resp.read_ub4()?; // end of call status
        resp.read_ub2()?; // end to end seq#
        resp.read_ub4()?; // current row number
        resp.read_ub2()?; // error number (short)
        resp.read_ub2()?; // array elem error
        resp.read_ub2()?; // array elem error
        let cursor_id = resp.read_ub2()?;
        resp.read_ub2()?; // error position
        resp.read_u8()?; // sql type (19c and earlier)
        resp.read_u8()?; // fatal?
        resp.read_u8()?; // flags
        resp.read_u8()?; // user cursor options
        resp.read_u8()?; // UPI parameter
        let flags = resp.read_u8()?;
        let _rowid = Rowid::deserialize(resp)?;
        resp.read_ub4()?; // OS error
        resp.read_u8()?; // statement number
        resp.read_u8()?; // call number
        resp.read_ub2()?; // padding
        resp.read_ub4()?; // success iters
        let num_bytes = resp.read_ub4()?; // logical rowid
        if num_bytes > 0 {
            resp.read_bytes_with_length()?;
        }
        let num_errors = resp.read_ub2()?;
        if num_errors > 0 {
            todo!();
        }
        let num_offsets = resp.read_ub4()?;
        if num_offsets > 0 {
            todo!();
        }
        let num_messages = resp.read_ub2()?;
        if num_messages > 0 {
            todo!();
        }
        let error_num = resp.read_ub4()?;
        let rowcount = resp.read_ub8()?;
        if client.supports_ttc_field_version(constants::TTC_FIELD_VERSION_20_1)
        {
            resp.read_ub4()?; // sql type
            resp.read_ub4()?; // server checksum
        }
        let message: &str = if error_num == 0 {
            ""
        } else {
            &resp.read_utf8_with_length()?
        };
        Ok(ErrorInfo {
            num: error_num as usize,
            cursor_id,
            flags,
            rowcount,
            message: message.trim_end().to_string(),
        })
    }

    /// Returns the cursor id.
    pub(crate) fn cursor_id(&self) -> u16 {
        self.cursor_id
    }

    /// Returns the error message.
    pub(crate) fn error_message(&self) -> &str {
        &self.message
    }

    /// Returns whether or not a compilation warning was returned.
    pub(crate) fn is_compilation_warning(&self) -> bool {
        self.flags & 0x20 != 0
    }

    /// Returns the row count.
    pub(crate) fn rowcount(&self) -> u64 {
        self.rowcount
    }
}
