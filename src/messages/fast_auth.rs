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
// fast_auth.rs
//
// Defines the structure used for sending and receiving the fast authentication
// message. This consists of an amalgamation of three other messages, thereby
// reducing round trips and simplifying the negotiation with the server.
// -----------------------------------------------------------------------------

use crate::client::Client;
use crate::constants;
use crate::db_info::DbInfo;
use crate::error::Error;
use crate::messages::Message;
use crate::messages::auth::AuthMessage;
use crate::messages::data_types::DataTypesMessage;
use crate::messages::protocol::ProtocolMessage;
use crate::response::Response;
use crate::write_buffer::WriteBuffer;

// constants
const TTC_FAST_AUTH_VERSION: u8 = 1;
const TTC_SERVER_CONVERTS_CHARS: u8 = 0x01;

pub struct FastAuthMessage {
    protocol_message: ProtocolMessage,
    data_types_message: DataTypesMessage,
    auth_message: AuthMessage,
}

impl FastAuthMessage {
    /// Creates a new FastAuthMessage structure and returns it.
    pub(crate) fn new() -> FastAuthMessage {
        FastAuthMessage {
            protocol_message: ProtocolMessage::new(),
            data_types_message: DataTypesMessage::new(),
            auth_message: AuthMessage::new(),
        }
    }

    /// Processes the second phase of authorization and performs post connect
    /// processing.
    pub(crate) fn process_auth_phase_two(
        &mut self,
        client: &mut Client,
    ) -> Result<DbInfo, Error> {
        client.process_message(&mut self.auth_message)?;
        client.post_connect(&mut self.auth_message)
    }
}

impl Message for FastAuthMessage {
    fn deserialize_ttc_message(
        &mut self,
        client: &Client,
        resp: &mut Response,
        message_type: u8,
    ) -> Result<(), Error> {
        match message_type {
            constants::TTC_MSG_TYPE_PROTOCOL => self
                .protocol_message
                .deserialize_ttc_message(client, resp, message_type),
            constants::TTC_MSG_TYPE_DATA_TYPES => self
                .data_types_message
                .deserialize_ttc_message(client, resp, message_type),
            _ => self.auth_message.deserialize_ttc_message(
                client,
                resp,
                message_type,
            ),
        }
    }

    fn is_final_ttc_message_type(
        &self,
        client: &Client,
        message_type: u8,
    ) -> bool {
        self.auth_message
            .is_final_ttc_message_type(client, message_type)
    }

    fn pre_deserialize(&mut self, client: &mut Client, _resp: &mut Response) {
        client.reset_ttc_field_version();
    }

    fn pre_process(&mut self, client: &mut Client) {
        self.protocol_message.pre_process(client);
        self.data_types_message.pre_process(client);
        self.auth_message.pre_process(client);
    }

    fn post_deserialize(
        &mut self,
        client: &mut Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        self.protocol_message.post_deserialize(client, resp)?;
        self.data_types_message.post_deserialize(client, resp)?;
        self.auth_message.post_deserialize(client, resp)
    }

    fn serialize(&self, client: &Client, buf: &mut WriteBuffer) {
        buf.write_u8(constants::TTC_MSG_TYPE_FAST_AUTH);
        buf.write_u8(TTC_FAST_AUTH_VERSION);
        buf.write_u8(TTC_SERVER_CONVERTS_CHARS); // flag 1
        buf.write_u8(0); // flag 2
        self.protocol_message.serialize(client, buf);
        buf.write_u16be(0); // server charset (unused)
        buf.write_u8(0); // server charset flag (unused)
        buf.write_u16be(0); // server ncharset (unused)
        buf.write_u8(constants::FAST_AUTH_TTC_FIELD_VERSION);
        self.data_types_message.serialize(client, buf);
        self.auth_message.serialize(client, buf);
    }
}
