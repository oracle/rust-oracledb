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
// lob_op.rs
//
// Defines the message used for LOB operations (TTC RPC 96).
//-----------------------------------------------------------------------------

use crate::client::Client;
use crate::constants;
use crate::error::Error;
use crate::messages::Message;
use crate::response::Response;
use crate::write_buffer::WriteBuffer;
use std::borrow::Cow;

/// Represents a LOB operation and the values needed to serialize it.
pub(crate) enum LobOp<'a> {
    /// Create a temporary LOB.
    CreateTemp { ora_type_num: u16, csfrm: u8 },
    /// Close an opened LOB locator.
    Close,
    /// Retrieve the optimal database chunk size for the LOB.
    GetChunkSize,
    /// Retrieve the current LOB length.
    GetLength,
    /// Check whether the LOB locator is currently open.
    IsOpen,
    /// Open the LOB with the supplied TTC open mode.
    Open(u64),
    /// Read data at the supplied offset and amount.
    Read(u64, u64),
    /// Trim the LOB to the supplied size.
    Trim(u64),
    /// Write data at the supplied offset.
    Write(u64, Cow<'a, [u8]>),
}

impl LobOp<'_> {
    /// Returns the LOB amount value for operations that use one.
    fn amount(&self) -> u64 {
        match self {
            LobOp::Open(mode) => *mode,
            LobOp::Read(_, amount) => *amount,
            LobOp::Trim(new_size) => *new_size,
            _ => 0,
        }
    }

    /// Returns the character set ID required for the operation.
    fn charset_id(&self, client: &Client) -> u16 {
        match self {
            LobOp::CreateTemp { csfrm, .. }
                if *csfrm == constants::CS_FORM_NCHAR =>
            {
                client.get_ncharset_id()
            }
            LobOp::CreateTemp { .. } => constants::CHARSET_ID_UTF8,
            _ => 0,
        }
    }

    /// Returns the character set pointer value.
    fn charset_pointer(&self) -> u8 {
        match self {
            LobOp::CreateTemp { .. } => 1,
            _ => 0,
        }
    }

    /// Returns the data payload to send with the operation, if any.
    fn data_to_send(&self) -> Option<&[u8]> {
        match self {
            LobOp::Write(_, data) => Some(data.as_ref()),
            _ => None,
        }
    }

    /// Returns the destination length value.
    fn destination_length(&self) -> u32 {
        match self {
            LobOp::CreateTemp { .. } => constants::TTC_DURATION_SESSION,
            _ => 0,
        }
    }

    /// Returns the destination offset value for operations that use one.
    fn destination_offset(&self) -> u64 {
        match self {
            LobOp::CreateTemp { ora_type_num, .. } => (*ora_type_num).into(),
            _ => 0,
        }
    }

    /// Returns the NULL LOB pointer value.
    fn null_lob_pointer(&self) -> u8 {
        match self {
            LobOp::CreateTemp { .. } | LobOp::IsOpen => 1,
            _ => 0,
        }
    }

    /// Returns the TTC operation code sent for this LOB operation.
    fn operation(&self) -> u32 {
        match self {
            LobOp::CreateTemp { .. } => constants::TTC_LOB_OP_CREATE_TEMP,
            LobOp::Close => constants::TTC_LOB_OP_CLOSE,
            LobOp::GetChunkSize => constants::TTC_LOB_OP_GET_CHUNK_SIZE,
            LobOp::GetLength => constants::TTC_LOB_OP_GET_LENGTH,
            LobOp::IsOpen => constants::TTC_LOB_OP_IS_OPEN,
            LobOp::Open(_) => constants::TTC_LOB_OP_OPEN,
            LobOp::Read(_, _) => constants::TTC_LOB_OP_READ,
            LobOp::Trim(_) => constants::TTC_LOB_OP_TRIM,
            LobOp::Write(_, _) => constants::TTC_LOB_OP_WRITE,
        }
    }

    /// Returns whether the operation expects a boolean return value.
    fn returns_bool(&self) -> bool {
        matches!(self, LobOp::IsOpen)
    }

    /// Returns whether the operation returns character set metadata.
    fn returns_charset(&self) -> bool {
        matches!(self, LobOp::CreateTemp { .. })
    }

    /// Returns whether the operation sends an amount pointer.
    fn sends_amount(&self) -> bool {
        matches!(
            self,
            LobOp::GetChunkSize
                | LobOp::GetLength
                | LobOp::Open(_)
                | LobOp::Read(_, _)
                | LobOp::Trim(_)
        )
    }

    /// Returns the source offset value for operations that use one.
    fn source_offset(&self) -> u64 {
        match self {
            LobOp::CreateTemp { csfrm, .. } => (*csfrm).into(),
            LobOp::Read(offset, _) | LobOp::Write(offset, _) => *offset,
            _ => 0,
        }
    }
}

/// Represents a TTC LOB operation message and its decoded response fields.
pub(crate) struct LobOpMessage<'a> {
    op: LobOp<'a>,
    source_locator: Vec<u8>,
    returned_locator: Option<Vec<u8>>,
    returned_amount: i64,
    returned_bool: bool,
    data: Vec<u8>,
}

impl<'a> LobOpMessage<'a> {
    /// Creates a LOB operation message for the supplied locator and operation.
    pub(crate) fn new(locator: &[u8], op: LobOp<'a>) -> LobOpMessage<'a> {
        LobOpMessage {
            op,
            source_locator: locator.to_vec(),
            returned_locator: None,
            returned_amount: 0,
            returned_bool: false,
            data: Vec::new(),
        }
    }

    /// Returns the scalar amount received from the server.
    pub(crate) fn returned_amount(&self) -> i64 {
        self.returned_amount
    }

    /// Returns the boolean value received from the server.
    pub(crate) fn returned_bool(&self) -> bool {
        self.returned_bool
    }

    /// Returns any data payload received for the operation.
    pub(crate) fn take_data(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.data)
    }

    /// Returns the locator received from the server, if any.
    pub(crate) fn take_returned_locator(&mut self) -> Option<Vec<u8>> {
        self.returned_locator.take()
    }
}

impl Message for LobOpMessage<'_> {
    /// Deserializes any LOB data payload returned for the operation.
    fn deserialize_lob_data(
        &mut self,
        _client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        let bytes = resp.read_bytes_with_length()?;
        self.data.extend_from_slice(&bytes);
        Ok(())
    }

    /// Deserializes the return parameters associated with the LOB operation.
    fn deserialize_return_parameters(
        &mut self,
        _client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        let new_locator = resp.read_bytes(self.source_locator.len())?.to_vec();
        self.returned_locator = Some(new_locator);
        if self.op.returns_charset() {
            let _charsetid = resp.read_ub2()?;
            let _flags = resp.read_u8()?;
        } else if self.op.sends_amount() {
            self.returned_amount = resp.read_sb8()?;
        }
        if self.op.returns_bool() {
            self.returned_bool = resp.read_u8()? > 0;
        }
        Ok(())
    }

    /// Serializes the LOB operation request into the wire buffer.
    fn serialize(&self, client: &Client, buf: &mut WriteBuffer) {
        buf.write_function_header(client, constants::TTC_RPC_LOB_OP);
        buf.write_u8(1); // source pointer
        buf.write_ub4(self.source_locator.len().try_into().unwrap());
        buf.write_u8(0); // dest pointer
        buf.write_ub4(self.op.destination_length());
        buf.write_ub4(0); // short source offset
        buf.write_ub4(0); // short dest offset
        buf.write_u8(self.op.charset_pointer());
        buf.write_u8(0); // pointer (short amount)
        buf.write_u8(self.op.null_lob_pointer());
        buf.write_ub4(self.op.operation());
        buf.write_u8(0); // pointer (SCN array)
        buf.write_u8(0); // SCN array length
        buf.write_ub8(self.op.source_offset());
        buf.write_ub8(self.op.destination_offset());
        if self.op.sends_amount() {
            buf.write_u8(1); // pointer (amount)
        } else {
            buf.write_u8(0); // pointer (amount)
        }
        for _ in 0..3 {
            buf.write_u16be(0); // array LOB (not used)
        }
        buf.write_bytes(&self.source_locator);
        if self.op.returns_charset() {
            buf.write_ub4(self.op.charset_id(client).into());
        }
        if let Some(data) = self.op.data_to_send() {
            buf.write_u8(constants::TTC_MSG_TYPE_LOB_DATA);
            buf.write_bytes_with_length(data);
        }
        if self.op.sends_amount() {
            buf.write_ub8(self.op.amount());
        }
    }
}
