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
// read_buffer.rs
//
// Defines the structures used for deserializing data found in buffers.
//-----------------------------------------------------------------------------

use std::borrow::Cow;

use crate::constants;
use crate::error::Error;
use crate::packet::Packet;

pub(crate) struct ReadBuffer {
    buf: Vec<u8>,
    pos: usize,
}

impl ReadBuffer {
    /// Internal method returning a tuple providing an integer buffer and sign.
    fn get_integer_buf_and_sign(
        &mut self,
        max_size: usize,
    ) -> Result<(&[u8], bool), Error> {
        let length_byte = self.read_u8()?;
        let is_negative = length_byte & 0x80 != 0;
        let length = if is_negative {
            length_byte & 0x7f
        } else {
            length_byte
        };
        let buf = self.read_bytes(length as usize)?;
        if buf.len() > max_size {
            Err(Error::integer_too_large(max_size, buf.len()))
        } else {
            Ok((buf, is_negative))
        }
    }

    /// Internal method returning a buffer containing an unsigned integer. If a
    /// signed integer is in the buffer an error is returned instead.
    fn get_unsigned_integer_buf(
        &mut self,
        max_size: usize,
    ) -> Result<&[u8], Error> {
        let (buf, is_negative) = self.get_integer_buf_and_sign(max_size)?;
        if is_negative {
            Err(Error::unexpected_negative_integer())
        } else {
            Ok(buf)
        }
    }

    /// Returns whether the buffer is empty or not.
    fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Creates and returns a new ReadBuffer struct given a slice. The slice is
    /// copied so that the original source need not be retained.
    pub(crate) fn from_bytes(bytes: &[u8]) -> ReadBuffer {
        ReadBuffer {
            buf: bytes.to_vec(),
            pos: 0,
        }
    }

    /// Creates and returns a new ReadBuffer struct given a series of packets.
    pub(crate) fn from_packets(packets: &[Packet]) -> ReadBuffer {
        let mut buf = Vec::<u8>::new();
        for packet in packets {
            buf.extend_from_slice(&packet.buf);
        }
        ReadBuffer { buf, pos: 0 }
    }

    /// Gets the current position in the buffer.
    pub(crate) fn get_pos(&self) -> usize {
        self.pos
    }

    /// Reads the specified number of bytes from the buffer. An error is
    /// returned if that number of bytes isn't available in the buffer.
    pub(crate) fn read_bytes(
        &mut self,
        num_bytes: usize,
    ) -> Result<&[u8], Error> {
        if self.pos + num_bytes > self.buf.len() {
            Err(Error::out_of_data())
        } else {
            let buf = &self.buf[self.pos..self.pos + num_bytes];
            self.pos += num_bytes;
            Ok(buf)
        }
    }

    /// Reads an unsigned integer length followed by length-encoded bytes.
    pub(crate) fn read_bytes_with_double_length(
        &mut self,
    ) -> Result<Cow<'_, [u8]>, Error> {
        let num_bytes = self.read_ub4()?;
        if num_bytes > 0 {
            self.read_bytes_with_length()
        } else {
            Ok(Cow::from(&[]))
        }
    }

    /// Reads length-encoded bytes from the buffer. An error is returned if
    /// those bytes are not available in the buffer.
    pub(crate) fn read_bytes_with_length(
        &mut self,
    ) -> Result<Cow<'_, [u8]>, Error> {
        let len = self.read_short_length()?;
        if len != constants::TTC_LONG_LENGTH_INDICATOR {
            Ok(Cow::from(self.read_bytes(len as usize)?))
        } else {
            let mut output = Vec::<u8>::new();
            loop {
                let num_bytes = self.read_ub4()?;
                if num_bytes == 0 {
                    break;
                }
                let bytes = self.read_bytes(num_bytes as usize)?;
                output.extend_from_slice(bytes);
            }
            Ok(Cow::from(output))
        }
    }

    /// Reads a signed 8-bit integer from the buffer. An error is returned if
    /// there is not at least one byte in the buffer.
    pub(crate) fn read_i8(&mut self) -> Result<i8, Error> {
        let buf = self.read_bytes(1)?;
        Ok(buf[0] as i8)
    }

    /// Reads an encoded signed 32-bit integer from the buffer. An error is
    /// returned if such an integer cannot be read from the buffer.
    pub fn read_sb4(&mut self) -> Result<i32, Error> {
        let (buf, is_negative) = self.get_integer_buf_and_sign(4)?;
        let value = u32::from_buf(buf);
        if is_negative {
            Ok((0_i32) - (value as i32))
        } else {
            Ok(value as i32)
        }
    }

    /// Reads an encoded signed 64-bit integer from the buffer. An error is
    /// returned if such an integer cannot be read from the buffer.
    pub fn read_sb8(&mut self) -> Result<i64, Error> {
        let (buf, is_negative) = self.get_integer_buf_and_sign(8)?;
        let value = u64::from_buf(buf);
        if is_negative {
            Ok((0_i64) - (value as i64))
        } else {
            Ok(value as i64)
        }
    }

    /// Reads a short length from the buffer, but if the value is the null
    /// length indicator, 0 is returned instead. An error is returned if there
    /// isn't at least one byte available in the buffer.
    pub(crate) fn read_short_length(&mut self) -> Result<u8, Error> {
        let mut short_len = self.read_u8()?;
        if short_len == constants::TTC_NULL_LENGTH_INDICATOR {
            short_len = 0;
        }
        Ok(short_len)
    }

    /// Reads length encoded bytes which are assumed to be valid UTF-8 encoded
    /// bytes. An error is returned if such a string cannot be read from the
    /// buffer.
    pub(crate) fn read_str_with_length(&mut self) -> Result<String, Error> {
        let bytes = self.read_bytes_with_length()?.into_owned();
        Ok(String::from_utf8(bytes).unwrap())
    }

    /// Reads an encoded unsigned integer from the buffer followed by
    /// length-encoded bytes which are assumed to be valid UTF-8 encoded bytes.
    /// An error is returned if either the integer or the bytes cannot be read
    /// from the buffer.
    pub(crate) fn read_str_with_double_length(
        &mut self,
    ) -> Result<String, Error> {
        let bytes = self.read_bytes_with_double_length()?.into_owned();
        Ok(String::from_utf8(bytes).unwrap())
    }

    /// Reads an encoded unsigned 16-bit integer from the buffer.
    pub(crate) fn read_ub2(&mut self) -> Result<u16, Error> {
        let buf = self.get_unsigned_integer_buf(2)?;
        Ok(u16::from_buf(buf))
    }

    /// Reads an encoded unsigned 32-bit integer from the buffer.
    pub(crate) fn read_ub4(&mut self) -> Result<u32, Error> {
        let buf = self.get_unsigned_integer_buf(4)?;
        Ok(u32::from_buf(buf))
    }

    /// Reads an encoded unsigned 64-bit integer from the buffer.
    pub(crate) fn read_ub8(&mut self) -> Result<u64, Error> {
        let buf = self.get_unsigned_integer_buf(8)?;
        Ok(u64::from_buf(buf))
    }

    /// Reads a unsigned 8-bit integer from the buffer.
    pub(crate) fn read_u8(&mut self) -> Result<u8, Error> {
        let buf = self.read_bytes(1)?;
        Ok(buf[0])
    }

    /// Reads an unsigned 16-bit integer in big endian format from the buffer.
    pub(crate) fn read_u16be(&mut self) -> Result<u16, Error> {
        let buf = self.read_bytes(2)?;
        Ok(u16::from_be_bytes(buf.try_into().unwrap()))
    }

    /// Reads an unsigned 16-bit integer in little endian format from the
    /// buffer.
    pub(crate) fn read_u16le(&mut self) -> Result<u16, Error> {
        let buf = self.read_bytes(2)?;
        Ok(u16::from_le_bytes(buf.try_into().unwrap()))
    }

    /// Reads an unsigned 32-bit integer in big endian format from the buffer.
    pub(crate) fn read_u32be(&mut self) -> Result<u32, Error> {
        let buf = self.read_bytes(4)?;
        Ok(u32::from_be_bytes(buf.try_into().unwrap()))
    }

    /// Called when a value is being read from the buffer. The value is assumed
    /// to contain a simple set of encoded bytes which can be transformed into
    /// the target type without error.
    pub(crate) fn read_value<T>(&mut self) -> Result<Option<T>, Error>
    where
        T: FromBuf,
    {
        let bytes = self.read_bytes_with_length()?;
        if bytes.is_empty() {
            Ok(None)
        } else {
            Ok(Some(<T>::from_buf(&bytes)))
        }
    }

    /// Called when a value is being read from a value-based LOB. The value is
    /// assumed to contain a complex set of encoded bytes which will need to be
    /// decoded in a fallible fashion.
    pub(crate) fn read_value_lob<T>(&mut self) -> Result<Option<T>, Error>
    where
        T: FromBufFallible,
    {
        let num_bytes = self.read_ub4()?;
        if num_bytes == 0 {
            Ok(None)
        } else {
            let _size = self.read_ub8()?;
            let _chunk_size = self.read_ub4()?;
            let mut buf =
                ReadBuffer::from_bytes(&self.read_bytes_with_length()?);
            let _locator = self.read_bytes_with_length()?;
            if buf.is_empty() {
                Ok(None)
            } else {
                Ok(Some(<T>::from_buf_fallible(&mut buf)?))
            }
        }
    }

    /// Sets the specified position in the buffer.
    pub(crate) fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }
}

pub(crate) trait FromBuf {
    fn from_buf(buf: &[u8]) -> Self;
}

impl FromBuf for bool {
    fn from_buf(buf: &[u8]) -> bool {
        buf[buf.len() - 1] == 1
    }
}

impl FromBuf for f32 {
    fn from_buf(buf: &[u8]) -> f32 {
        let mut raw_bytes: [u8; 4] = [0; 4];
        raw_bytes.copy_from_slice(buf);
        if raw_bytes[0] & 0x80 != 0 {
            raw_bytes[0] &= 0x7f;
        } else {
            raw_bytes[0] = !raw_bytes[0];
            raw_bytes[1] = !raw_bytes[1];
            raw_bytes[2] = !raw_bytes[2];
            raw_bytes[3] = !raw_bytes[3];
        }
        f32::from_be_bytes(raw_bytes)
    }
}

impl FromBuf for f64 {
    fn from_buf(buf: &[u8]) -> f64 {
        let mut raw_bytes: [u8; 8] = [0; 8];
        raw_bytes.copy_from_slice(buf);
        if raw_bytes[0] & 0x80 != 0 {
            raw_bytes[0] &= 0x7f;
        } else {
            raw_bytes[0] = !raw_bytes[0];
            raw_bytes[1] = !raw_bytes[1];
            raw_bytes[2] = !raw_bytes[2];
            raw_bytes[3] = !raw_bytes[3];
            raw_bytes[4] = !raw_bytes[4];
            raw_bytes[5] = !raw_bytes[5];
            raw_bytes[6] = !raw_bytes[6];
            raw_bytes[7] = !raw_bytes[7];
        }
        f64::from_be_bytes(raw_bytes)
    }
}

impl FromBuf for u16 {
    fn from_buf(buf: &[u8]) -> u16 {
        match buf.len() {
            0 => 0,
            1 => buf[0] as u16,
            _ => u16::from_be_bytes(buf.try_into().unwrap()),
        }
    }
}

impl FromBuf for u32 {
    fn from_buf(buf: &[u8]) -> u32 {
        match buf.len() {
            3 => {
                let first = u16::from_buf(&buf[..2]);
                (first as u32) << 8 | (buf[2] as u32)
            }
            4 => u32::from_be_bytes(buf.try_into().unwrap()),
            _ => u16::from_buf(buf) as u32,
        }
    }
}

impl FromBuf for u64 {
    fn from_buf(buf: &[u8]) -> u64 {
        match buf.len() {
            5 => {
                let first = u32::from_buf(&buf[..4]);
                (first as u64) << 8 | (buf[4] as u64)
            }
            6 => {
                let first = u32::from_buf(&buf[..4]);
                let second = u16::from_buf(&buf[4..]);
                (first as u64) << 16 | (second as u64)
            }
            7 => {
                let first = u32::from_buf(&buf[..4]);
                let second = u32::from_buf(&buf[4..]);
                (first as u64) << 24 | (second as u64)
            }
            8 => u64::from_be_bytes(buf.try_into().unwrap()),
            _ => u32::from_buf(buf) as u64,
        }
    }
}

pub(crate) trait FromBufFallible {
    fn from_buf_fallible(buf: &mut ReadBuffer) -> Result<Self, Error>
    where
        Self: Sized;
}
