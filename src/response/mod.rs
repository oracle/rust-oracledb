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
// response.rs
//
// Defines the structures used for handling the response to a request sent by
// the client to the server.
//-----------------------------------------------------------------------------

mod error_info;

use std::borrow::Cow;

use crate::client::Client;
use crate::client::ClientRef;
use crate::constants;
use crate::db_value::DbValue;
use crate::error::Error;
use crate::packet::Packet;
use crate::read_buffer::FromBuf;
use crate::read_buffer::FromBufFallible;
use crate::read_buffer::ReadBuffer;
use crate::row::RowData;
use crate::statement::CachedStatement;

use error_info::ErrorInfo;

pub(crate) struct Response {
    packet_type: u8,
    packet_flags: u8,
    buf: ReadBuffer,
    error_info: Option<ErrorInfo>,
    client_ref: Option<ClientRef>,
    edition: Option<String>,
    current_schema: Option<String>,
    warning: Option<String>,
    rows: Option<Vec<RowData>>,
    prev_fetch_last_row: Option<RowData>,
    bit_vector: Option<Vec<u8>>,
    num_columns: usize,
    end_of_fetch: bool,
}

impl Response {
    pub(crate) fn get_client_ref(&self) -> ClientRef {
        self.client_ref.clone().unwrap()
    }

    pub(crate) fn set_client_ref(&mut self, client_ref: ClientRef) {
        self.client_ref = Some(client_ref);
    }

    pub(crate) fn advance(&mut self, cnt: usize) -> Result<(), Error> {
        self.buf.read_bytes(cnt)?;
        Ok(())
    }

    pub(crate) fn check_for_end_of_fetch(
        &mut self,
        statement: &CachedStatement,
    ) -> Result<(), Error> {
        if self.get_error_num() == constants::DB_ERR_NUM_NO_DATA_FOUND
            && statement.is_query()
        {
            self.end_of_fetch = true;
            Ok(())
        } else {
            self.check_for_error()
        }
    }

    pub(crate) fn check_for_error(&mut self) -> Result<(), Error> {
        if let Some(error_info) = self.error_info.as_ref() {
            let message = error_info.error_message();
            if !message.is_empty() {
                return Err(Error::db_error(message.to_string()));
            }
        }
        Ok(())
    }

    pub(crate) fn deserialize_bit_vector(&mut self) -> Result<(), Error> {
        let num_columns = self.read_ub2()? as usize;
        if self.num_columns == 0 {
            self.num_columns = num_columns;
        }
        let mut num_bytes = self.num_columns / 8;
        if !self.num_columns.is_multiple_of(8) {
            num_bytes += 1;
        }
        self.bit_vector = Some(self.buf.read_bytes(num_bytes)?.into());
        Ok(())
    }

    pub(crate) fn deserialize_row_data(
        &mut self,
        client: &Client,
        statement: &CachedStatement,
        in_fetch: bool,
    ) -> Result<(), Error> {
        let mut column_values = RowData::new();
        for (i, metadata) in statement.out_metadata().iter().enumerate() {
            let value = DbValue::from_response(
                self, client, statement, metadata, in_fetch, i,
            )?;
            column_values.push(value);
        }
        if let Some(rows) = self.rows.as_mut() {
            rows.push(column_values);
        } else {
            self.rows = Some(vec![column_values]);
        }
        Ok(())
    }

    /// Deserializes a server side piggyback. These contain data from the
    /// server on which the client may need to operate.
    pub(crate) fn deserialize_server_side_piggyback(
        &mut self,
    ) -> Result<(), Error> {
        let opcode = self.read_u8()?;
        match opcode {
            constants::TTC_SERVER_PIGGYBACK_LTXID => {
                let _ltxid = self.read_bytes_with_length()?;
            }
            constants::TTC_SERVER_PIGGYBACK_QUERY_CACHE_INVALIDATION
            | constants::TTC_SERVER_PIGGYBACK_TRACE_EVENT => {}
            constants::TTC_SERVER_PIGGYBACK_OS_PID_MTS => {
                let _ = self.read_ub2()?;
                let _ = self.read_bytes_with_length()?;
            }
            constants::TTC_SERVER_PIGGYBACK_SYNC => {
                let _num_dtys = self.read_ub2()?;
                let _dty_length = self.read_u8()?;
                let num_elements = self.read_ub2()?;
                let _len = self.read_u8()?;
                self.process_keyword_value_pairs(num_elements)?;
                let _overall_flags = self.read_ub4()?;
            }
            constants::TTC_SERVER_PIGGYBACK_EXT_SYNC => {
                let _num_dtys = self.read_ub2()?;
                let _dty_length = self.read_u8()?;
            }
            constants::TTC_SERVER_PIGGYBACK_AC_REPLAY_CONTEXT => {
                let _num_dtys = self.read_ub2()?;
                let _dty_length = self.read_u8()?;
                let _flags = self.read_ub4()?;
                let _error_code = self.read_ub4()?;
                let _queue = self.read_u8()?;
                let _replay_context = self.read_bytes_with_length()?;
            }
            constants::TTC_SERVER_PIGGYBACK_SESS_RET => {
                self.read_ub2()?;
                self.read_u8()?;
                let num_elements = self.read_ub2()?;
                if num_elements > 0 {
                    self.read_u8()?;
                    for _ in 0..num_elements {
                        if self.read_ub2()? > 0 {
                            let _key = self.read_bytes_with_length()?;
                        }
                        if self.read_ub2()? > 0 {
                            let _value = self.read_bytes_with_length()?;
                        }
                        let _session_flags = self.read_ub2()?;
                    }
                }
                let _flags = self.read_ub4()?;
                let _session_id = self.read_ub4()?;
                let _serial_num = self.read_ub2()?;
            }
            constants::TTC_SERVER_PIGGYBACK_SESS_SIGNATURE => {
                let _num_dtys = self.read_ub2()?;
                let _dty_length = self.read_u8()?;
                let _signature_flags = self.read_ub8()?;
                let _client_signature = self.read_ub8()?;
                let _server_signature = self.read_ub8()?;
            }
            _ => {
                return Err(Error::unknown_server_side_piggyback(opcode));
            }
        }
        Ok(())
    }

    pub(crate) fn deserialize_warning(&mut self) -> Result<(), Error> {
        let error_num = self.read_ub2()?;
        let num_bytes = self.read_ub2()?;
        let _flags = self.read_ub2()?;
        if error_num != 0 && num_bytes > 0 {
            let message = self.read_utf8_with_length()?;
            self.warning = Some(message.trim_end().to_string());
        }
        Ok(())
    }

    pub(crate) fn get_cursor_id(&self) -> u16 {
        if let Some(error_info) = self.error_info.as_ref() {
            error_info.cursor_id()
        } else {
            0
        }
    }

    pub(crate) fn get_error_num(&self) -> usize {
        if let Some(error_info) = self.error_info.as_ref() {
            error_info.num
        } else {
            0
        }
    }

    pub(crate) fn get_last_row_fetched(&self) -> &RowData {
        if let Some(rows) = self.rows.as_ref() {
            rows.last().unwrap()
        } else {
            self.prev_fetch_last_row.as_ref().unwrap()
        }
    }

    pub(crate) fn get_packet_flags(&self) -> u8 {
        self.packet_flags
    }

    pub(crate) fn get_packet_type(&self) -> u8 {
        self.packet_type
    }

    /// Returns the rowcount returned by the database.
    pub(crate) fn get_rowcount(&self) -> u64 {
        if let Some(error_info) = self.error_info.as_ref() {
            error_info.rowcount()
        } else {
            0
        }
    }

    pub(crate) fn is_duplicate_data(&self, column_num: usize) -> bool {
        if let Some(bit_vector) = self.bit_vector.as_ref() {
            let byte_num = column_num / 8;
            let bit_num = column_num % 8;
            bit_vector[byte_num] & (1 << bit_num) == 0
        } else {
            false
        }
    }

    pub(crate) fn is_end_of_fetch(&self) -> bool {
        self.end_of_fetch
    }

    pub(crate) fn new(packets: &[Packet]) -> Response {
        let packet = packets.first().unwrap();
        Response {
            packet_type: packet.packet_type,
            packet_flags: packet.packet_flags,
            buf: ReadBuffer::from_packets(packets),
            error_info: None,
            client_ref: None,
            edition: None,
            current_schema: None,
            warning: None,
            rows: None,
            prev_fetch_last_row: None,
            bit_vector: None,
            num_columns: 0,
            end_of_fetch: false,
        }
    }

    pub(crate) fn process_keyword_value_pairs(
        &mut self,
        num_pairs: u16,
    ) -> Result<(), Error> {
        for _ in 0..num_pairs {
            let text_value = self.read_utf8_with_double_length()?.to_string();
            let _binary_value = self.read_bytes_with_double_length()?;
            let keyword_num = self.read_ub2()?;
            match keyword_num {
                constants::TTC_KEYWORD_NUM_CURRENT_SCHEMA => {
                    self.current_schema = Some(text_value);
                }
                constants::TTC_KEYWORD_NUM_EDITION => {
                    self.edition = Some(text_value);
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(crate) fn read_bit_vector(&mut self) -> Result<(), Error> {
        let num_bytes = self.read_ub4()?;
        if num_bytes == 0 {
            self.bit_vector = None;
        } else {
            let bit_vector = self.read_bytes_with_length()?;
            self.bit_vector = Some(bit_vector.into());
        }
        Ok(())
    }

    pub(crate) fn read_bytes(
        &mut self,
        num_bytes: usize,
    ) -> Result<&[u8], Error> {
        self.buf.read_bytes(num_bytes)
    }

    pub(crate) fn read_bytes_with_length(
        &mut self,
    ) -> Result<Cow<'_, [u8]>, Error> {
        self.buf.read_bytes_with_length()
    }

    pub(crate) fn read_bytes_with_double_length(
        &mut self,
    ) -> Result<Cow<'_, [u8]>, Error> {
        self.buf.read_bytes_with_double_length()
    }

    pub(crate) fn read_error_info(
        &mut self,
        client: &Client,
    ) -> Result<(), Error> {
        let error_info = ErrorInfo::deserialize(self, client)?;
        if error_info.is_compilation_warning() {
            self.warning =
                Some("creation succeeded with compilation errors".to_string());
        }
        self.error_info = Some(error_info);
        Ok(())
    }

    pub(crate) fn read_i8(&mut self) -> Result<i8, Error> {
        self.buf.read_i8()
    }

    pub(crate) fn read_short_length(&mut self) -> Result<u8, Error> {
        self.buf.read_short_length()
    }

    /// Called when a value is being read from the buffer. The value is assumed
    /// to contain a simple set of encoded bytes which can be transformed into
    /// the target type without error.
    pub(crate) fn read_value<T>(&mut self) -> Result<Option<T>, Error>
    where
        T: FromBuf,
    {
        self.buf.read_value::<T>()
    }

    /// Called when a value is being read from a value-based LOB. The value is
    /// assumed to contain a complex set of encoded bytes which will need to be
    /// decoded in a fallible fashion.
    pub(crate) fn read_value_lob<T>(&mut self) -> Result<Option<T>, Error>
    where
        T: FromBufFallible,
    {
        self.buf.read_value_lob::<T>()
    }

    pub(crate) fn read_sb4(&mut self) -> Result<i32, Error> {
        self.buf.read_sb4()
    }

    pub(crate) fn read_sb8(&mut self) -> Result<i64, Error> {
        self.buf.read_sb8()
    }

    pub(crate) fn read_ub2(&mut self) -> Result<u16, Error> {
        self.buf.read_ub2()
    }

    pub(crate) fn read_ub4(&mut self) -> Result<u32, Error> {
        self.buf.read_ub4()
    }

    pub(crate) fn read_ub8(&mut self) -> Result<u64, Error> {
        self.buf.read_ub8()
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8, Error> {
        self.buf.read_u8()
    }

    pub(crate) fn read_u16be(&mut self) -> Result<u16, Error> {
        self.buf.read_u16be()
    }

    pub(crate) fn read_u16le(&mut self) -> Result<u16, Error> {
        self.buf.read_u16le()
    }

    pub(crate) fn read_u32be(&mut self) -> Result<u32, Error> {
        self.buf.read_u32be()
    }

    /// Reads the specified number of bytes from the buffer which are assumed
    /// to be valid UTF-8 encoded bytes, and returns a string reference.
    pub(crate) fn read_utf8(
        &mut self,
        num_bytes: usize,
    ) -> Result<&str, Error> {
        self.buf.read_utf8(num_bytes)
    }

    /// Reads an encoded unsigned integer from the buffer followed by
    /// length-encoded bytes which are assumed to be valid UTF-8 encoded bytes.
    /// An error is returned if either the integer or the bytes cannot be read
    /// from the buffer.
    pub(crate) fn read_utf8_with_double_length(
        &mut self,
    ) -> Result<Cow<'_, str>, Error> {
        self.buf.read_utf8_with_double_length()
    }

    /// Reads length encoded bytes which are assumed to be valid UTF-8 encoded
    /// bytes. An error is returned if such a string cannot be read from the
    /// buffer.
    pub(crate) fn read_utf8_with_length(
        &mut self,
    ) -> Result<Cow<'_, str>, Error> {
        self.buf.read_utf8_with_length()
    }

    pub(crate) fn set_prev_fetch_last_row(
        &mut self,
        last_row: Option<RowData>,
    ) {
        self.prev_fetch_last_row = last_row;
    }

    pub(crate) fn set_num_columns(&mut self, num_columns: usize) {
        self.num_columns = num_columns;
    }

    /// Takes the rows from the response and returns them.
    pub(crate) fn take_rows(&mut self) -> Option<Vec<RowData>> {
        self.rows.take()
    }

    /// Takes the warning from the response and returns them.
    pub(crate) fn take_warning(&mut self) -> Option<String> {
        self.warning.take()
    }

    /// Transfers information from another response that was received earlier.
    /// This is intended for use when a batch of statements is being executed
    /// and a single response is being returned.
    pub(crate) fn transfer_info(&mut self, other_resp: &mut Response) {
        if self.rows.is_none() {
            self.rows = other_resp.take_rows();
        } else if let Some(mut other_rows) = other_resp.take_rows() {
            let mut final_rows = self.rows.take().unwrap();
            other_rows.append(&mut final_rows);
            self.rows = Some(other_rows);
        }
        if let Some(error_info) = self.error_info.as_mut() {
            error_info.rowcount += other_resp.get_rowcount();
        }
    }
}
