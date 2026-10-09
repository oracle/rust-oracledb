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

pub(crate) struct ErrorInfo {
    db_error: Option<DbError>,
    cursor_id: u16,
    flags: u8,
    rowcount: u64,
}

impl ErrorInfo {
    /// Deserializes the error information from the database response.
    pub fn deserialize(
        resp: &mut Response,
        client: &Client,
    ) -> Result<ErrorInfo, Error> {
        resp.deserialize_status()?;
        let _current_row_num = resp.read_ub4()?;
        let _error_num_short = resp.read_ub2()?;
        let _array_elem_error_1 = resp.read_ub2()?;
        let _array_elem_error_2 = resp.read_ub2()?;
        let cursor_id = resp.read_ub2()?;
        let error_pos = resp.read_ub2()?;
        let _old_sql_type = resp.read_u8()?;
        let _fatal = resp.read_u8()?;
        let _flags_1 = resp.read_u8()?;
        let _user_cursor_options = resp.read_u8()?;
        let _upi_parameter = resp.read_u8()?;
        let flags = resp.read_u8()?;
        let _rowid = Rowid::deserialize(resp)?;
        let _os_error = resp.read_ub4()?;
        let _statement_num = resp.read_u8()?;
        let _call_num = resp.read_u8()?;
        let _padding = resp.read_ub2()?;
        let _success_iters = resp.read_ub4()?;
        if resp.read_ub4()? > 0 {
            let _logical_rowid = resp.read_bytes_with_length()?;
        }
        if resp.read_ub2()? > 0 {
            return Err(Error::not_implemented("batch errors".to_string()));
        }
        if resp.read_ub4()? > 0 {
            return Err(Error::not_implemented(
                "batch error offsets".to_string(),
            ));
        }
        if resp.read_ub2()? > 0 {
            return Err(Error::not_implemented(
                "batch error messages".to_string(),
            ));
        }
        let error_num = resp.read_ub4()?;
        let rowcount = resp.read_ub8()?;
        if client.supports_ttc_field_version(constants::TTC_FIELD_VERSION_20_1)
        {
            let _sql_type = resp.read_ub4()?;
            let _server_checksum = resp.read_ub4()?;
        }
        let db_error = if error_num == 0 {
            None
        } else {
            let message = resp.read_utf8_with_length()?;
            Some(DbError {
                code: error_num as usize,
                offset: error_pos as usize,
                message: message.trim_end().to_string(),
            })
        };

        Ok(ErrorInfo {
            cursor_id,
            flags,
            rowcount,
            db_error,
        })
    }

    /// Returns the cursor id.
    pub(crate) fn cursor_id(&self) -> u16 {
        self.cursor_id
    }

    /// Returns whether or not a compilation warning was returned.
    pub(crate) fn is_compilation_warning(&self) -> bool {
        self.flags & 0x20 != 0
    }

    /// Returns the row count.
    pub(crate) fn rowcount(&self) -> u64 {
        self.rowcount
    }

    /// Takes the error message from the response and returns it.
    pub(crate) fn take_db_error(&mut self) -> Option<DbError> {
        self.db_error.take()
    }

    /// Transfers information from another error info structure that was
    /// received earlier. This is intended for us when a batch of statements is
    /// being executed and a single response is being returned.
    pub(crate) fn transfer_into(&mut self, other_info: &mut ErrorInfo) {
        self.rowcount += other_info.rowcount;
    }
}

/// Represents errors returned by the database.
#[derive(Clone, Debug, PartialEq)]
pub struct DbError {
    code: usize,
    offset: usize,
    message: String,
}

impl std::fmt::Display for DbError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}", self.message)
    }
}

impl DbError {
    /// Returns the error code associated with the database error.
    pub fn code(&self) -> usize {
        self.code
    }

    /// Returns the error message associated with the database error.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the offset associated with the database error.
    pub fn offset(&self) -> usize {
        self.offset
    }
}

#[cfg(test)]
mod tests {
    use crate::client::Client;
    use crate::config::Config;
    use crate::error::Error;
    use crate::error::ErrorKind;
    use crate::read_buffer::ReadBuffer;
    use crate::response::Response;

    /// Appends a length-encoded unsigned integer to the buffer.
    fn push_ub(buf: &mut Vec<u8>, value: u64) {
        let bytes = value.to_be_bytes();
        let mut first = bytes.iter().position(|&b| b != 0).unwrap_or(7);
        if first > 0 && bytes[first] & 0x80 != 0 {
            first -= 1;
        }
        let len = 8 - first;
        buf.push(len as u8);
        buf.extend_from_slice(&bytes[first..]);
    }

    /// Builds a buffer containing an error info payload up to and including
    /// the three batch error counts.
    fn make_buf(
        batch_errors: u64,
        batch_error_offsets: u64,
        batch_error_messages: u64,
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        push_ub(&mut buf, 0); // call status
        push_ub(&mut buf, 0); // sequence number
        push_ub(&mut buf, 0); // current row num
        push_ub(&mut buf, 0); // error num (short)
        push_ub(&mut buf, 0); // array elem error 1
        push_ub(&mut buf, 0); // array elem error 2
        push_ub(&mut buf, 0); // cursor id
        push_ub(&mut buf, 0); // error position
        buf.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // remaining u8 fields
        push_ub(&mut buf, 0); // rowid rba
        push_ub(&mut buf, 0); // rowid partition id
        buf.push(0); // rowid unused byte
        push_ub(&mut buf, 0); // rowid block num
        push_ub(&mut buf, 0); // rowid slot num
        push_ub(&mut buf, 0); // os error
        buf.extend_from_slice(&[0, 0]); // statement num, call num
        push_ub(&mut buf, 0); // padding
        push_ub(&mut buf, 0); // success iterations
        push_ub(&mut buf, 0); // logical rowid count
        push_ub(&mut buf, batch_errors);
        push_ub(&mut buf, batch_error_offsets);
        push_ub(&mut buf, batch_error_messages);
        buf
    }

    /// Deserializes the error info from the given buffer and returns the
    /// result.
    fn deserialize(buf: &[u8]) -> Result<super::ErrorInfo, Error> {
        let mut resp = Response::new();
        resp.buf = ReadBuffer::from_bytes(buf);
        let client = Client::new(Config::default(), String::new());
        super::ErrorInfo::deserialize(&mut resp, &client)
    }

    #[test]
    fn batch_errors_return_an_error() {
        let error = match deserialize(&make_buf(1, 0, 0)) {
            Err(error) => error,
            Ok(_) => panic!("expected an error"),
        };
        assert!(matches!(
            error.kind(),
            ErrorKind::NotImplemented(feature)
                if feature.as_str() == "batch errors"
        ));
    }

    #[test]
    fn batch_error_offsets_return_an_error() {
        let error = match deserialize(&make_buf(0, 1, 0)) {
            Err(error) => error,
            Ok(_) => panic!("expected an error"),
        };
        assert!(matches!(
            error.kind(),
            ErrorKind::NotImplemented(feature)
                if feature.as_str() == "batch error offsets"
        ));
    }

    #[test]
    fn batch_error_messages_return_an_error() {
        let error = match deserialize(&make_buf(0, 0, 1)) {
            Err(error) => error,
            Ok(_) => panic!("expected an error"),
        };
        assert!(matches!(
            error.kind(),
            ErrorKind::NotImplemented(feature)
                if feature.as_str() == "batch error messages"
        ));
    }
}
