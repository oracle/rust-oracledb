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
// write_buffer.rs
//
// Defines the structure used for serializing data into a buffer.
//-----------------------------------------------------------------------------

use crate::client::Client;
use crate::constants;
use crate::db_type::DbType;

pub struct WriteBuffer {
    buf: Vec<u8>,
}

impl WriteBuffer {
    /// Clears the buffer of all content.
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    /// Returns a reference to the buffer contents.
    pub fn get_buf(&self) -> &[u8] {
        &self.buf[..]
    }

    /// Returns the length of the buffer.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Creates a new instance.
    pub fn new() -> Self {
        Self {
            buf: Vec::<u8>::new(),
        }
    }

    /// Reserves the specified number of bytes in the buffer for later use. The
    /// current length of the buffer (before reservation takes place) is
    /// returned as a convenience.
    pub fn reserve_bytes(&mut self, num_bytes: usize) -> usize {
        let orig_len = self.buf.len();
        self.buf.resize(orig_len + num_bytes, 0);
        orig_len
    }

    /// Writes the contents of the slice to the buffer.
    pub fn write_bytes(&mut self, value: &[u8]) {
        self.buf.extend(value);
    }

    /// Writes an optional byte slice using the TTC double-length format.
    pub fn write_bytes_with_double_length(&mut self, value: Option<&[u8]>) {
        if let Some(value) = value {
            self.write_ub4(value.len().try_into().unwrap());
            if !value.is_empty() {
                self.write_bytes_with_length(value);
            }
        } else {
            self.write_ub4(0);
        }
    }

    /// Writes the contents of the slice to the buffer but first encodes a
    /// length.
    pub fn write_bytes_with_length(&mut self, value: &[u8]) {
        let mut num_bytes = value.len();
        if num_bytes <= constants::TTC_MAX_SHORT_LENGTH.into() {
            self.write_u8(num_bytes.try_into().unwrap());
            self.write_bytes(value);
        } else {
            self.write_u8(constants::TTC_LONG_LENGTH_INDICATOR);
            let mut remaining_data = value;
            while num_bytes > 0 {
                let chunk_len =
                    std::cmp::min(num_bytes, constants::TTC_CHUNK_SIZE);
                self.write_ub4(chunk_len.try_into().unwrap());
                self.buf.extend(&remaining_data[..chunk_len]);
                remaining_data = &remaining_data[chunk_len..];
                num_bytes -= chunk_len;
            }
            self.write_ub4(0);
        }
    }

    /// Writes the function header to the buffer.
    pub fn write_function_header(&mut self, client: &Client, fn_type: u8) {
        self.write_rpc_header(
            client,
            constants::TTC_MSG_TYPE_FUNCTION,
            fn_type,
        );
    }

    /// Writes the piggyback header to the buffer.
    pub fn write_piggyback_header(
        &mut self,
        client: &Client,
        piggyback_type: u8,
    ) {
        self.write_rpc_header(
            client,
            constants::TTC_MSG_TYPE_PIGGYBACK,
            piggyback_type,
        );
    }

    /// Writes a QLocator (quasi-locator) which is used for writing values
    /// like JSON and vectors to the buffer in the format required by the
    /// database.
    pub fn write_qlocator(&mut self, encoded_data: &WriteBuffer) {
        self.write_ub4(40); // fixed length of QLocator
        self.write_u8(40); // repeated length
        self.write_u16be(38); // length less 2 bytes
        self.write_u16be(4); // QLocator version
        self.write_u8(
            constants::TTC_LOB_LOC_FLAGS_VALUE_BASED
                | constants::TTC_LOB_LOC_FLAGS_BLOB
                | constants::TTC_LOB_LOC_FLAGS_ABSTRACT,
        );
        self.write_u8(constants::TTC_LOB_LOC_FLAGS_INIT);
        self.write_u16be(0); // additional flags (unused)
        self.write_u16be(1); // byt1
        self.write_u64be(encoded_data.buf.len().try_into().unwrap());
        self.write_u16be(0); // unused
        self.write_u16be(0); // character set id (unused)
        self.write_u16be(0); // unused
        self.write_u64be(0); // unused
        self.write_u64be(0); // unused
        self.write_bytes_with_length(&encoded_data.buf);
    }

    /// Writes the RPC header to the buffer. The RPC could be a function or a
    /// piggyback. Only one function is sent in a request but many piggybacks
    /// could be sent at the same time.
    pub fn write_rpc_header(
        &mut self,
        client: &Client,
        message_type: u8,
        rpc_type: u8,
    ) {
        self.write_u8(message_type);
        self.write_u8(rpc_type);
        self.write_u8(0); // RPC sequence number (unused)
        if client.supports_ttc_field_version(
            constants::TTC_FIELD_VERSION_23_1_EXT_1,
        ) {
            self.write_ub8(0); // token number
        }
    }

    /// Writes a string to the buffer.
    pub fn write_str(&mut self, value: &str) {
        self.write_bytes(value.as_bytes());
    }

    /// Writes a u16 to the buffer as a length-encoded integer.
    pub fn write_ub2(&mut self, value: u16) {
        if value == 0 {
            self.write_u8(0);
        } else if value < u8::MAX.into() {
            self.write_u8(1);
            self.write_u8(value.try_into().unwrap());
        } else {
            self.write_u8(2);
            self.write_u16be(value);
        }
    }

    /// Writes a u32 to the buffer as a length-encoded integer.
    pub fn write_ub4(&mut self, value: u32) {
        if value == 0 {
            self.write_u8(0);
        } else if value < u8::MAX.into() {
            self.write_u8(1);
            self.write_u8(value.try_into().unwrap());
        } else if value < u16::MAX.into() {
            self.write_u8(2);
            self.write_u16be(value.try_into().unwrap());
        } else {
            self.write_u8(4);
            self.write_u32be(value);
        }
    }

    /// Writes a u64 to the buffer as a length-encoded integer.
    pub fn write_ub8(&mut self, value: u64) {
        if value == 0 {
            self.write_u8(0);
        } else if value < u8::MAX.into() {
            self.write_u8(1);
            self.write_u8(value.try_into().unwrap());
        } else if value < u16::MAX.into() {
            self.write_u8(2);
            self.write_u16be(value.try_into().unwrap());
        } else if value < u32::MAX.into() {
            self.write_u8(4);
            self.write_u32be(value.try_into().unwrap());
        } else {
            self.write_u8(8);
            self.write_u64be(value);
        }
    }

    /// Writes an unsigned 8-bit integer to the buffer.
    pub fn write_u8(&mut self, value: u8) {
        self.buf.push(value);
    }

    /// Writes a unsigned 16-bit integer to the buffer in big endian format.
    pub fn write_u16be(&mut self, value: u16) {
        self.buf.extend(value.to_be_bytes());
    }

    /// Writes an unsigned 16-bit integer to the buffer in little endian
    /// format.
    pub fn write_u16le(&mut self, value: u16) {
        self.buf.extend(value.to_le_bytes());
    }

    /// Writes an unsigned 32-bit integer to the buffer in big endian format.
    pub fn write_u32be(&mut self, value: u32) {
        self.buf.extend(value.to_be_bytes());
    }

    /// Writes an unsigned 32-bit integer to the buffer in big endian format at
    /// the specified offset.
    pub fn write_u32be_at(&mut self, value: u32, offset: usize) {
        let bytes = value.to_be_bytes();
        self.buf[offset..offset + 4].copy_from_slice(&bytes);
    }

    /// Writes an unsigned 64-bit integer to the buffer in big endian format.
    pub fn write_u64be(&mut self, value: u64) {
        self.buf.extend(value.to_be_bytes());
    }
}

pub trait ToBuf {
    /// Writes the value to the buffer.
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        db_type: &'static DbType,
        write_length: bool,
    );

    /// Writes a null value to the buffer.
    fn to_buf_null(&self, buf: &mut WriteBuffer) {
        buf.write_u8(0);
    }
}

impl ToBuf for bool {
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        _db_type: &'static DbType,
        write_length: bool,
    ) {
        if *self {
            if write_length {
                buf.write_u8(2);
            }
            buf.write_u8(1);
            buf.write_u8(1);
        } else {
            if write_length {
                buf.write_u8(1);
            }
            buf.write_u8(0);
        }
    }
}

impl ToBuf for f32 {
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        _db_type: &'static DbType,
        write_length: bool,
    ) {
        let mut conv_buf = self.to_be_bytes();
        if conv_buf[0] & 0x80 == 0 {
            conv_buf[0] |= 0x80;
        } else {
            for value in &mut conv_buf {
                *value = !*value;
            }
        }
        if write_length {
            buf.write_bytes_with_length(&conv_buf);
        } else {
            buf.write_bytes(&conv_buf);
        }
    }
}

impl ToBuf for f64 {
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        _db_type: &'static DbType,
        write_length: bool,
    ) {
        let mut conv_buf = self.to_be_bytes();
        if conv_buf[0] & 0x80 == 0 {
            conv_buf[0] |= 0x80;
        } else {
            for value in &mut conv_buf {
                *value = !*value;
            }
        }
        if write_length {
            buf.write_bytes_with_length(&conv_buf);
        } else {
            buf.write_bytes(&conv_buf);
        }
    }
}

impl ToBuf for String {
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        _db_type: &'static DbType,
        write_length: bool,
    ) {
        if write_length {
            buf.write_bytes_with_length(self.as_bytes());
        } else {
            buf.write_bytes(self.as_bytes());
        }
    }
}

impl ToBuf for &str {
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        _db_type: &'static DbType,
        write_length: bool,
    ) {
        if write_length {
            buf.write_bytes_with_length(self.as_bytes());
        } else {
            buf.write_bytes(self.as_bytes());
        }
    }
}

impl ToBuf for Vec<u8> {
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        _db_type: &'static DbType,
        write_length: bool,
    ) {
        if write_length {
            buf.write_bytes_with_length(self);
        } else {
            buf.write_bytes(self);
        }
    }
}

impl ToBuf for &[u8] {
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        _db_type: &'static DbType,
        write_length: bool,
    ) {
        if write_length {
            buf.write_bytes_with_length(self);
        } else {
            buf.write_bytes(self);
        }
    }
}

impl<T> ToBuf for Option<T>
where
    T: ToBuf + Default,
{
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        db_type: &'static DbType,
        write_length: bool,
    ) {
        if let Some(value) = self {
            <T>::to_buf(value, buf, db_type, write_length);
        } else {
            <T>::default().to_buf_null(buf);
        }
    }
}
