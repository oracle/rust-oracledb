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
// connect.rs
//
// Defines the structure used for sending and receiving the connect message.
// This is the first message sent to the database while establishing a
// connection.
// -----------------------------------------------------------------------------

use crate::client::Client;
use crate::config::Address;
use crate::config::Description;
use crate::constants;
use crate::error::Error;
use crate::messages::Message;
use crate::response::Response;
use crate::write_buffer::WriteBuffer;

// Global Service Options
const GSO_DONT_CARE: u16 = 0x0001;

// NSI constants
const NSI_DISABLE_NA: u8 = 0x04;
const NSI_NA_REQUIRED: u8 = 0x10;
const NSI_SUPPORT_SECURITY_RENEG: u8 = 0x80;

// other constants
const PROTOCOL_CHARACTERISTICS: u16 = 0x4f98;
const MAX_CONNECT_DATA: usize = 230;

pub struct ConnectMessage<'a> {
    pub connect_data: &'a str,
    description: &'a Description,
    address: &'a Address,
    pub packet_flags: u8,
    pub sdu: u32,
    pub protocol_version: u16,
    pub protocol_flags: u32,
    pub accepted: bool,
    pub tls_renegotiation_needed: bool,
    pub redirect_data: Option<String>,
    pub redirect_data_len: u16,
}

impl ConnectMessage<'_> {
    pub fn new<'a>(
        connect_data: &'a str,
        address: &'a Address,
        description: &'a Description,
    ) -> ConnectMessage<'a> {
        ConnectMessage {
            connect_data,
            address,
            description,
            packet_flags: 0,
            sdu: description.sdu(),
            protocol_version: 0,
            protocol_flags: 0,
            accepted: false,
            tls_renegotiation_needed: false,
            redirect_data: None,
            redirect_data_len: 0,
        }
    }

    fn process_accept_packet(
        &mut self,
        resp: &mut Response,
    ) -> Result<(), Error> {
        self.protocol_version = resp.read_u16be()?;
        if self.protocol_version < constants::PROTOCOL_VERSION_12 {
            return Err(Error::server_version_not_supported());
        }
        resp.advance(12)?;
        let flags1: u8 = resp.read_u8()?;
        if flags1 & NSI_NA_REQUIRED != 0 {
            todo!();
        }
        resp.advance(9)?;
        self.sdu = resp.read_u32be()?;
        if self.protocol_version >= constants::PROTOCOL_VERSION_18 {
            resp.advance(5)?;
            self.protocol_flags = resp.read_u32be()?;
        }
        self.accepted = true;
        Ok(())
    }

    /// Processes a refuse packet sent by the listener.
    fn process_refuse_packet(
        &mut self,
        resp: &mut Response,
    ) -> Result<(), Error> {
        resp.advance(2)?;
        let mut error_num: usize = 0;
        let message_len = resp.read_u16be()? as usize;
        if message_len > 0 {
            let message = resp.read_utf8(message_len)?;
            if let Some(start_pos) = message.find("(ERR=")
                && let Some(end_pos) = message[start_pos..].find(")")
            {
                let error_num_str =
                    &message[start_pos + 5..start_pos + end_pos];
                error_num = error_num_str.parse::<usize>().unwrap();
            }
        }
        let connection_id = self.description.connection_id().to_string();
        if error_num == 0 {
            Err(Error::unexpected_refuse(connection_id))
        } else if error_num == constants::DB_ERR_NUM_INVALID_SERVICE_NAME {
            Err(Error::invalid_service_name(
                connection_id,
                self.description.service_name().to_string(),
                self.address.host().to_string(),
                self.address.port(),
            ))
        } else if error_num == constants::DB_ERR_NUM_INVALID_SID {
            Err(Error::invalid_sid(
                connection_id,
                self.description.sid().to_string(),
                self.address.host().to_string(),
                self.address.port(),
            ))
        } else {
            Err(Error::listener_refused_connection(
                connection_id,
                self.address.host().to_string(),
                self.address.port(),
                error_num,
            ))
        }
    }
}

impl Message for ConnectMessage<'_> {
    fn deserialize(
        &mut self,
        _client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        self.accepted = false;
        self.tls_renegotiation_needed = false;
        match resp.get_packet_type() {
            constants::PACKET_TYPE_ACCEPT => {
                self.process_accept_packet(resp)?;
            }
            constants::PACKET_TYPE_REFUSE => {
                self.process_refuse_packet(resp)?;
            }
            constants::PACKET_TYPE_RESEND => {
                self.tls_renegotiation_needed = resp.get_packet_flags()
                    & constants::PACKET_FLAGS_TLS_RENEG
                    != 0;
            }
            constants::PACKET_TYPE_REDIRECT => {
                self.redirect_data_len = resp.read_u16be()?;
            }
            constants::PACKET_TYPE_DATA => {
                self.redirect_data = Some(
                    resp.read_utf8(self.redirect_data_len as usize)?
                        .to_string(),
                );
            }
            _ => {
                todo!()
            }
        }
        Ok(())
    }

    /// Returns whether extended data (in a separate packet) needs to be sent.
    fn extended_data_needed(&self) -> bool {
        self.connect_data.len() > MAX_CONNECT_DATA
    }

    /// Returns the packet flags to use in the packet header.
    fn get_packet_flags(&self) -> u8 {
        self.packet_flags
    }

    /// Returns the packet type to use in the packet header.
    fn get_packet_type(&self) -> u8 {
        constants::PACKET_TYPE_CONNECT
    }

    /// Serializes the message.
    fn serialize(&self, client: &Client, buf: &mut WriteBuffer) {
        let short_sdu: u16 = {
            if self.sdu > u16::MAX as u32 {
                u16::MAX
            } else {
                self.sdu.try_into().unwrap()
            }
        };
        let nsi_flags = NSI_SUPPORT_SECURITY_RENEG | NSI_DISABLE_NA;
        buf.write_u16be(constants::PROTOCOL_VERSION_23);
        buf.write_u16be(constants::PROTOCOL_VERSION_MIN);
        buf.write_u16be(GSO_DONT_CARE);
        buf.write_u16be(short_sdu);
        buf.write_u16be(short_sdu);
        buf.write_u16be(PROTOCOL_CHARACTERISTICS);
        buf.write_u16be(0); // line turanound
        buf.write_u16be(1); // value of 1
        buf.write_u16be(self.connect_data.len() as u16);
        buf.write_u16be(74); // offset to connect data
        buf.write_u32be(0); // max receivable data
        buf.write_u8(nsi_flags);
        buf.write_u8(nsi_flags);
        buf.write_u64be(0); // obsolete
        buf.write_u64be(0); // obsolete
        buf.write_u64be(0); // obsolete
        buf.write_u32be(self.sdu);
        buf.write_u32be(self.sdu);
        buf.write_u32be(0); // connect flags 1
        buf.write_u32be(0); // connect flags 1
        if !self.extended_data_needed() {
            self.serialize_extended_data(client, buf);
        }
    }

    /// Serializes the message.
    fn serialize_extended_data(
        &self,
        _client: &Client,
        buf: &mut WriteBuffer,
    ) {
        buf.write_bytes(self.connect_data.as_bytes());
    }
}
