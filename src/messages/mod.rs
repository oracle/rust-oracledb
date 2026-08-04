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
// mod.rs (messages module)
//
// Defines the structures used for managing the protocol between the database
// and the client.
//-----------------------------------------------------------------------------

mod auth;
mod commit;
mod connect;
mod data_types;
mod eof;
mod execute;
mod fast_auth;
mod fetch;
mod lob_op;
mod logoff;
mod marker;
mod ping;
mod protocol;
mod rollback;

use crate::client::Client;
use crate::constants;
use crate::error::Error;
use crate::response::Response;
use crate::write_buffer::WriteBuffer;

pub(crate) trait Message {
    /// Deserializes a message. The default consists of deserializing a series
    /// of TTC messages.
    fn deserialize(
        &mut self,
        client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error>
    where
        Self: Message,
    {
        loop {
            let message_type = resp.read_u8()?;
            self.deserialize_ttc_message(client, resp, message_type)?;
            if self.is_final_ttc_message_type(client, message_type) {
                break;
            }
        }
        Ok(())
    }

    /// Deserializes a TTC describe information message.
    fn deserialize_describe_info(
        &mut self,
        _client: &Client,
        _resp: &mut Response,
    ) -> Result<(), Error> {
        let feature = "deserializing describe info".to_string();
        Err(Error::not_implemented(feature))
    }

    /// Deserializes a TTC I/O vector message.
    fn deserialize_io_vector(
        &mut self,
        _client: &Client,
        _resp: &mut Response,
    ) -> Result<(), Error> {
        let feature = "deserializing IO vector".to_string();
        Err(Error::not_implemented(feature))
    }

    /// Deserializes a TTC LOB data message.
    fn deserialize_lob_data(
        &mut self,
        _client: &Client,
        _resp: &mut Response,
    ) -> Result<(), Error> {
        let feature = "deserializing LOB data".to_string();
        Err(Error::not_implemented(feature))
    }

    /// Deserializes a TTC return parameters message.
    fn deserialize_return_parameters(
        &mut self,
        _client: &Client,
        _resp: &mut Response,
    ) -> Result<(), Error> {
        let feature = "deserializing return parameters".to_string();
        Err(Error::not_implemented(feature))
    }

    /// Deserializes a TTC row data message.
    fn deserialize_row_data(
        &mut self,
        _client: &Client,
        _resp: &mut Response,
    ) -> Result<(), Error> {
        let feature = "deserializing row data".to_string();
        Err(Error::not_implemented(feature))
    }

    /// Deserializes a TTC row header message.
    fn deserialize_row_header(
        &mut self,
        _client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        let _flags = resp.read_u8()?;
        let _num_requests = resp.read_ub2()?;
        let _iter_num = resp.read_ub4()?;
        let _num_iters = resp.read_ub4()?;
        let _buffer_len = resp.read_ub2()?;
        resp.read_bit_vector()?;
        let _rxhrid = resp.read_bytes_with_length()?;
        Ok(())
    }

    /// Deserializes a TTC status message.
    fn deserialize_status(
        &mut self,
        _client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        let _call_status = resp.read_ub4()?;
        let _seq_num = resp.read_ub2()?;
        Ok(())
    }

    /// Deserializes one TTC message after its type has already been read.
    fn deserialize_ttc_message(
        &mut self,
        client: &Client,
        resp: &mut Response,
        message_type: u8,
    ) -> Result<(), Error>
    where
        Self: Message,
    {
        match message_type {
            constants::TTC_MSG_TYPE_BIT_VECTOR => {
                resp.deserialize_bit_vector()
            }
            constants::TTC_MSG_TYPE_DESCRIBE_INFO => {
                self.deserialize_describe_info(client, resp)
            }
            constants::TTC_MSG_TYPE_END_OF_RESPONSE => Ok(()),
            constants::TTC_MSG_TYPE_ERROR => resp.read_error_info(client),
            constants::TTC_MSG_TYPE_IO_VECTOR => {
                self.deserialize_io_vector(client, resp)
            }
            constants::TTC_MSG_TYPE_LOB_DATA => {
                self.deserialize_lob_data(client, resp)
            }
            constants::TTC_MSG_TYPE_PARAMETER => {
                self.deserialize_return_parameters(client, resp)
            }
            constants::TTC_MSG_TYPE_ROW_DATA => {
                self.deserialize_row_data(client, resp)
            }
            constants::TTC_MSG_TYPE_ROW_HEADER => {
                self.deserialize_row_header(client, resp)
            }
            constants::TTC_MSG_TYPE_SERVER_SIDE_PIGGYBACK => {
                resp.deserialize_server_side_piggyback()
            }
            constants::TTC_MSG_TYPE_STATUS => {
                self.deserialize_status(client, resp)
            }
            constants::TTC_MSG_TYPE_WARNING => resp.deserialize_warning(),
            _ => Err(Error::unknown_ttc_message_type(message_type)),
        }
    }

    /// Returns whether extended data is required for this message. Some
    /// messages (like the connect data message) require a new packet header.
    fn extended_data_needed(&self) -> bool {
        false
    }

    /// Returns the data flags to use in the packet header.
    fn get_data_flags(&self) -> u16 {
        0
    }

    /// Returns the packet type to use in the packet header.
    fn get_packet_type(&self) -> u8 {
        constants::PACKET_TYPE_DATA
    }

    /// Returns the packet flags to use in the packet header.
    fn get_packet_flags(&self) -> u8 {
        0
    }

    /// Returns whether the TTC message type that is being processed is the
    /// final message type and no further data is expected.
    fn is_final_ttc_message_type(
        &self,
        client: &Client,
        message_type: u8,
    ) -> bool {
        if client.supports_end_of_response() {
            message_type == constants::TTC_MSG_TYPE_END_OF_RESPONSE
        } else {
            message_type == constants::TTC_MSG_TYPE_ERROR
                || message_type == constants::TTC_MSG_TYPE_STATUS
        }
    }

    /// Called after the data sent by the database has been deserialized. The
    /// default processing is to check the response for an error.
    fn post_deserialize(
        &mut self,
        _client: &mut Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        resp.check_for_error()
    }

    /// Called just before the message is sent to the database for processing.
    fn pre_process(&mut self, _client: &mut Client) {}

    /// Called just before the data sent by the database is deserialized.
    fn pre_deserialize(&mut self, _client: &mut Client, _resp: &mut Response) {
    }

    /// Returns whether or not a message needs to be sent a second time.
    fn resend_needed(&self) -> bool {
        false
    }

    /// Serializes the message to the write buffer. All messages must implement
    /// this method.
    fn serialize(&self, client: &Client, buf: &mut WriteBuffer);

    /// Serializes the message's extended data to the write buffer. Only
    /// messages which require extended data need to implement this.
    fn serialize_extended_data(
        &self,
        _client: &Client,
        _buf: &mut WriteBuffer,
    ) {
    }
}

pub(crate) use auth::AuthMessage;
pub(crate) use commit::CommitMessage;
pub(crate) use connect::ConnectMessage;
pub(crate) use data_types::DataTypesMessage;
pub(crate) use eof::EofMessage;
pub(crate) use execute::ExecuteMessage;
pub(crate) use fast_auth::FastAuthMessage;
pub(crate) use fetch::FetchMessage;
pub(crate) use lob_op::{LobOp, LobOpMessage};
pub(crate) use logoff::LogoffMessage;
pub(crate) use marker::MarkerMessage;
pub(crate) use ping::PingMessage;
pub(crate) use protocol::ProtocolMessage;
pub(crate) use rollback::RollbackMessage;
