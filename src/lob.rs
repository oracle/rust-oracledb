//-----------------------------------------------------------------------------
// Copyright (c) 2026, Oracle and/or its affiliates.
//
// This software is dual-licensed to you under the Universal Permissive License
// (UPL) 1.0 as shown at https://oss.oracle.com/licenses/upl and Apache License
// 2.0 as shown at http://www.apache.org/licenses/LICENSE-2.0. You may choose
// either license.
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
// lob.rs
//
// Defines a minimal LOB type. At this stage it is simply an opaque wrapper
// around the locator returned by the database when fetching LOB locators.
//-----------------------------------------------------------------------------

use crate::DB_TYPE_BFILE;
use crate::client::ClientRef;
use crate::constants;
use crate::db_type::DbType;
use crate::error::Error;
use crate::messages::{LobOp, LobOpMessage};
use crate::response::Response;
use crate::utils;
use std::borrow::Cow;
use std::fmt;
use std::io;

/// Represents a database LOB locator.
///
/// This object is returned when BLOB, CLOB, NCLOB, and BFILE data is fetched
/// as a LOB locator. Use the methods on this type to read, write, open, close,
/// trim, and inspect the LOB. This type implements [`std::io::Read`] and
/// [`std::io::Write`] for streaming access.
///
/// Character LOB offsets and amounts use Oracle UCS-2 units, not UTF-8 bytes
/// or Rust `char` counts. Supplemental characters use two Oracle units.
/// Decoding to UTF-8 can expand the data, so reads need a large enough caller
/// buffer and writes need complete UTF-8 in each call.
#[derive(Clone)]
pub struct Lob {
    client_ref: ClientRef,
    locator: Vec<u8>,
    db_type: &'static DbType,
    size: Option<usize>,
    chunk_size: Option<usize>,
    offset: usize,
}

impl fmt::Debug for Lob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Lob")
            .field("locator_len", &self.locator.len())
            .field("db_type", &self.db_type.name())
            .field("size", &self.size)
            .field("chunk_size", &self.chunk_size)
            .field("offset", &self.offset)
            .finish()
    }
}

impl Lob {
    /// Decodes bytes read from a character LOB into a Rust string.
    fn decode_string(
        bytes: &[u8],
        encoding: LobStringEncoding,
    ) -> io::Result<String> {
        match encoding {
            LobStringEncoding::Utf8 => String::from_utf8(bytes.to_vec())
                .map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        error.to_string(),
                    )
                }),
            LobStringEncoding::Utf16Be => {
                Ok(utils::utf16be_bytes_to_string(bytes))
            }
            LobStringEncoding::Utf16Le => {
                Ok(utils::utf16le_bytes_to_string(bytes))
            }
        }
    }

    /// Converts a driver error into an I/O error for trait implementations.
    fn io_error(error: Error) -> io::Error {
        io::Error::other(error.to_string())
    }

    /// Returns whether the LOB contains character data.
    fn is_character_lob(&self) -> bool {
        self.db_type.ora_type_num == constants::ORA_TYPE_NUM_CLOB
    }

    /// Converts length (in bytes) to the amount (in bytes or UCS-2 code
    /// points) required by the Oracle Database.
    fn length_to_amount(&self, length: usize) -> usize {
        if self.is_character_lob() {
            length / 2
        } else {
            length
        }
    }

    /// Returns the TTC open mode to use for this LOB type.
    fn open_mode(&self) -> u64 {
        if self.db_type == &DB_TYPE_BFILE {
            constants::TTC_LOB_OPEN_READ_ONLY
        } else {
            constants::TTC_LOB_OPEN_READ_WRITE
        }
    }

    /// Processes a LOB operation and updates the locator returned by the
    /// server.
    fn process_lob_op<'a>(
        &mut self,
        op: LobOp<'a>,
    ) -> Result<LobOpMessage<'a>, Error> {
        let mut message = LobOpMessage::new(&self.locator, op);
        self.client_ref
            .lock()
            .unwrap()
            .process_message(&mut message)?;
        if let Some(locator) = message.take_returned_locator() {
            self.locator = locator;
        }
        Ok(message)
    }

    /// Reads data from the LOB at the specified Oracle offset.
    fn read_at(
        &mut self,
        offset: usize,
        amount: usize,
    ) -> Result<Vec<u8>, Error> {
        let mut message = self.process_lob_op(LobOp::Read(
            offset.try_into().unwrap(),
            amount.try_into().unwrap(),
        ))?;
        Ok(message.take_data())
    }

    /// Determines the string encoding used by the current LOB locator.
    fn string_encoding(&self) -> LobStringEncoding {
        if self.db_type.csfrm == constants::CS_FORM_NCHAR {
            return LobStringEncoding::Utf16Be;
        }
        let flags3 = self.locator[constants::TTC_LOB_LOC_OFFSET_FLAG_3];
        if flags3 & constants::TTC_LOB_LOC_FLAGS_VAR_LENGTH_CHARSET != 0 {
            let flags4 = self.locator[constants::TTC_LOB_LOC_OFFSET_FLAG_4];
            if flags4 & constants::TTC_LOB_LOC_FLAGS_LITTLE_ENDIAN != 0 {
                return LobStringEncoding::Utf16Le;
            }
            return LobStringEncoding::Utf16Be;
        }
        LobStringEncoding::Utf8
    }

    /// Writes already-encoded data to the LOB at the specified Oracle offset.
    fn write_at(
        &mut self,
        data: Cow<'_, [u8]>,
        offset: usize,
    ) -> Result<(), Error> {
        let amount = self.length_to_amount(data.len());
        self.process_lob_op(LobOp::Write(offset.try_into().unwrap(), data))?;
        self.size = None;
        self.offset += amount;
        Ok(())
    }

    /// Deserializes a LOB locator from a database response.
    pub(crate) fn from_resp(
        resp: &mut Response,
        db_type: &'static DbType,
    ) -> Result<Option<Lob>, Error> {
        let non_null_indicator = resp.read_ub4()?;
        if non_null_indicator == 0 {
            Ok(None)
        } else {
            let size: usize = resp.read_ub8()?.try_into().unwrap();
            let chunk_size: usize = resp.read_ub4()?.try_into().unwrap();
            // For BLOB/CLOB locators the protocol returns the locator as a
            // length-encoded byte sequence (no prefetched data payload)
            let locator = resp.read_bytes_with_length()?.into_owned();
            Ok(Some(Lob {
                client_ref: resp.get_client_ref(),
                locator,
                db_type,
                size: Some(size),
                chunk_size: Some(chunk_size),
                offset: 1,
            }))
        }
    }

    /// Closes the LOB.
    pub fn close(&mut self) -> Result<(), Error> {
        self.process_lob_op(LobOp::Close)?;
        Ok(())
    }

    /// Returns the database type of the LOB.
    pub fn db_type(&self) -> &'static DbType {
        self.db_type
    }

    /// Returns the chunk size for the LOB.
    pub fn get_chunk_size(&mut self) -> Result<usize, Error> {
        if self.chunk_size.is_none() {
            let message = self.process_lob_op(LobOp::GetChunkSize)?;
            let chunk_size: usize =
                message.returned_amount().try_into().unwrap();
            self.chunk_size = Some(chunk_size);
        }
        Ok(self.chunk_size.unwrap())
    }

    /// Returns the size of the data in the LOB.
    pub fn get_size(&mut self) -> Result<usize, Error> {
        if self.size.is_none() {
            let message = self.process_lob_op(LobOp::GetLength)?;
            let size: usize = message.returned_amount().try_into().unwrap();
            self.size = Some(size);
        }
        Ok(self.size.unwrap())
    }

    /// Returns whether the LOB has been opened using [`open`](Self::open).
    pub fn is_open(&mut self) -> Result<bool, Error> {
        let message = self.process_lob_op(LobOp::IsOpen)?;
        Ok(message.returned_bool())
    }

    /// Opens the LOB.
    pub fn open(&mut self) -> Result<(), Error> {
        self.process_lob_op(LobOp::Open(self.open_mode()))?;
        Ok(())
    }

    /// Trims the LOB to the new size.
    pub fn trim(&mut self, new_size: usize) -> Result<(), Error> {
        self.process_lob_op(LobOp::Trim(new_size.try_into().unwrap()))?;
        self.size = Some(new_size);
        if self.offset > new_size + 1 {
            self.offset = new_size + 1;
        }
        Ok(())
    }
}

/// Reads LOB data as a forward-only byte stream.
///
/// For CLOB/NCLOB data, Oracle offsets and amounts use UCS-2 units. If UTF-8
/// expansion makes the decoded data larger than `buf`, `read()` returns
/// `InvalidInput` without advancing the LOB offset.
impl io::Read for Lob {
    /// Reads bytes from the current LOB stream position into `buf`.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let size = self.get_size().map_err(Self::io_error)?;
        if self.offset > size {
            return Ok(0);
        }

        let is_character_lob = self.is_character_lob();
        let remaining = size - self.offset + 1;
        let amount = if is_character_lob {
            // Character LOB amounts are Oracle UCS-2 units. Allow up to 3
            // UTF-8 bytes per unit after decoding.
            remaining.min((buf.len() / 3).max(1))
        } else {
            remaining.min(buf.len())
        };
        let bytes =
            self.read_at(self.offset, amount).map_err(Self::io_error)?;
        let offset_increment = self.length_to_amount(bytes.len());

        let data = if is_character_lob {
            // Character LOBs are returned to callers as UTF-8, regardless of
            // the stored representation used by the locator.
            let encoding = self.string_encoding();
            let value = Self::decode_string(&bytes, encoding)?;
            value.into_bytes()
        } else {
            bytes
        };

        if data.len() > buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "buffer too small for LOB read",
            ));
        }
        buf[..data.len()].copy_from_slice(&data);
        self.offset += offset_increment;
        Ok(data.len())
    }
}

/// Writes LOB data as a forward-only byte stream.
impl io::Write for Lob {
    /// Flushes this output stream.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    /// Writes bytes at the current LOB stream position.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.db_type == &DB_TYPE_BFILE {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "writing to BFILE is not supported",
            ));
        }

        let data = if self.is_character_lob() {
            let value = std::str::from_utf8(buf).map_err(|error| {
                io::Error::new(io::ErrorKind::InvalidData, error.to_string())
            })?;
            match self.string_encoding() {
                LobStringEncoding::Utf16Be => {
                    Cow::Owned(utils::string_to_utf16be_bytes(value))
                }
                LobStringEncoding::Utf16Le => {
                    Cow::Owned(utils::string_to_utf16le_bytes(value))
                }
                _ => todo!(),
            }
        } else {
            Cow::Borrowed(buf)
        };

        self.write_at(data, self.offset).map_err(Self::io_error)?;
        Ok(buf.len())
    }
}

/// Character encoding used by a character LOB locator.
#[derive(Clone, Copy, Debug)]
enum LobStringEncoding {
    /// UTF-8 character data.
    Utf8,
    /// Big-endian UTF-16 character data.
    Utf16Be,
    /// Little-endian UTF-16 character data.
    Utf16Le,
}
