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
// protocol.rs
//
// Defines the structure used for sending and receiving the protocol message.
// This is the second message sent to the database while establishing a
// connection and can be combined with the data types and auth messages when
// fast authentication is available.
// -----------------------------------------------------------------------------

use crate::client::Client;
use crate::constants;
use crate::error::Error;
use crate::messages::Message;
use crate::read_buffer::ReadBuffer;
use crate::response::Response;
use crate::write_buffer::WriteBuffer;

pub struct ProtocolMessage {
    charset_id: u16,
    ncharset_id: u16,
    server_compile_caps: Vec<u8>,
    server_runtime_caps: Vec<u8>,
}

impl ProtocolMessage {
    pub fn new() -> Self {
        Self {
            charset_id: 0,
            ncharset_id: 0,
            server_compile_caps: Vec::<u8>::new(),
            server_runtime_caps: Vec::<u8>::new(),
        }
    }

    fn skip_array(&mut self, buf: &mut ReadBuffer) -> Result<(), Error> {
        loop {
            let byte = buf.read_u8()?;
            if byte == 0 {
                break;
            }
        }
        Ok(())
    }
}

impl Message for ProtocolMessage {
    fn deserialize_ttc_message(
        &mut self,
        _client: &Client,
        resp: &mut Response,
        buf: &mut ReadBuffer,
        message_type: u8,
    ) -> Result<(), Error> {
        if message_type != constants::TTC_MSG_TYPE_PROTOCOL {
            return Err(resp.unknown_ttc_message_type(buf, message_type));
        }
        self.skip_array(buf)?; // protocol message version
        self.skip_array(buf)?; // server banner
        self.charset_id = buf.read_u16le()?;
        let _server_flags = buf.read_u8()?;
        let num_elem = buf.read_u16le()?;
        if num_elem > 0 {
            let graph_len: usize = (num_elem * 5).into();
            let _charset_graph = buf.read_bytes(graph_len)?;
        }
        let fdo_len = buf.read_u16be()? as usize;
        buf.read_bytes(5)?; // skip first part of FDO
        let offset1 = buf.read_u8()? as usize;
        let offset2 = buf.read_u8()? as usize;
        buf.read_bytes(offset1 + offset2 + 2)?;
        self.ncharset_id = buf.read_u16be()?;
        buf.read_bytes(fdo_len - offset1 - offset2 - 11)?;
        self.server_compile_caps = buf.read_bytes_with_length()?.into();
        self.server_runtime_caps = buf.read_bytes_with_length()?.into();
        Ok(())
    }

    fn is_final_ttc_message_type(
        &self,
        _client: &Client,
        _message_type: u8,
    ) -> bool {
        true
    }

    /// Called after the message has been deserialized. The capabilities of the
    /// client are adjusted based on the capabilities of the server.
    fn post_deserialize(
        &mut self,
        client: &mut Client,
        _resp: &mut Response,
    ) -> Result<(), Error> {
        client.set_charset_ids(self.charset_id, self.ncharset_id);
        if !self.server_compile_caps.is_empty() {
            client.adjust_for_server_compile_caps(&self.server_compile_caps);
        }
        if !self.server_runtime_caps.is_empty() {
            client.adjust_for_server_runtime_caps(&self.server_runtime_caps);
        }
        Ok(())
    }

    fn serialize(&self, _client: &Client, buf: &mut WriteBuffer) {
        buf.write_u8(constants::TTC_MSG_TYPE_PROTOCOL);
        buf.write_u8(6); // protocol message version
        buf.write_u8(0); // "array" terminator
        buf.write_str(constants::DRIVER_NAME);
        buf.write_u8(0); // "NULL" terminator
    }
}
