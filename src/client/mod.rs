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
// mod.rs (client module)
//
// Defines the structures used for handling the requests to and the responses
// from the server.
//-----------------------------------------------------------------------------

mod capabilities;

use std::mem;
use std::net::SocketAddr;

use crate::config::Address;
use crate::config::Config;
use crate::config::Description;
use crate::constants;
use crate::db_info::DbInfo;
use crate::end_user_security_context::EndUserSecurityContext;
use crate::error::Error;
use crate::messages::AuthMessage;
use crate::messages::ConnectMessage;
use crate::messages::DataTypesMessage;
use crate::messages::EofMessage;
use crate::messages::FastAuthMessage;
use crate::messages::LogoffMessage;
use crate::messages::MarkerMessage;
use crate::messages::Message;
use crate::messages::ProtocolMessage;
use crate::packet::Packet;
use crate::response::Response;
use crate::statement::CachedStatement;
use crate::statement::StatementCache;
use crate::statement::StatementOptions;
use crate::transport::Transport;
use crate::write_buffer::WriteBuffer;

use capabilities::Capabilities;

const TTC_SECURITY_CONTEXT_ATTACH_FLAG: u32 = 0x01;
const TTC_END_USER_SECURITY_CONTEXT_KEY: &str = "ORCL_XS_AUTHZ_CONTEXT";

pub struct Client {
    transport: Transport,
    caps: capabilities::Capabilities,
    config: Config,
    combo_key: Option<[u8; 32]>,
    charset_id: u16,
    ncharset_id: u16,
    statement_cache: StatementCache,
    drcp_establish_session: bool,
    override_ttc_field_version: u8,
    pending_error_num: usize,
    pending_action: Option<Vec<u8>>,
    pending_client_identifier: Option<Vec<u8>>,
    pending_client_info: Option<Vec<u8>>,
    pending_db_op: Option<Vec<u8>>,
    pending_module: Option<Vec<u8>>,
    last_warning: Option<String>,
    security_context: Option<EndUserSecurityContext>,
}

pub(crate) type ClientRef = std::sync::Arc<std::sync::Mutex<Client>>;

impl Client {
    /// Performs a round trip to the database by sending the message and
    /// receiving back the database response.
    fn perform_round_trip(
        &mut self,
        message: &mut impl Message,
        client_ref_opt: Option<&ClientRef>,
    ) -> Result<Response, Error> {
        message.pre_process(self);
        self.send_message(message)?;
        self.receive_response(message, client_ref_opt)
    }

    /// Process a control packet received from the database.
    fn process_control_packet(&mut self, packet: Packet) -> Result<(), Error> {
        let packets = vec![packet];
        let mut resp = Response::new(&packets);
        let control_type = resp.read_u16be()?;
        if control_type == constants::TTC_CONTROL_TYPE_INBAND_NOTIF {
            resp.advance(4)?;
            self.pending_error_num = resp.read_u32be()? as usize;
        }
        Ok(())
    }

    /// Receives a data packet from the database. Control packets and marker
    /// packets are processed. Only data packets are returned.
    fn receive_data_packet(&mut self) -> Result<Packet, Error> {
        loop {
            match self.receive_packet() {
                Ok(packet_opt) => {
                    if let Some(packet) = packet_opt {
                        return Ok(packet);
                    }
                }
                Err(err) => {
                    if err.is_call_timeout_exceeded() {
                        return self.recover_from_error(err);
                    }
                    return Err(err);
                }
            }
        }
    }

    /// Receives a packet from the database and either processes it immediately
    /// (and returns None) or returns it directly for the caller to process.
    fn receive_packet(&mut self) -> Result<Option<Packet>, Error> {
        let packet = self.transport.receive_packet()?;
        match packet.packet_type {
            constants::PACKET_TYPE_CONTROL => {
                self.process_control_packet(packet)?;
                Ok(None)
            }
            constants::PACKET_TYPE_MARKER => {
                let packet =
                    self.reset().map_err(|_| self.unrecoverable_error())?;
                Ok(Some(packet))
            }
            _ => Ok(Some(packet)),
        }
    }

    /// Called when a timeout occurs and attempts to recover from it by sending
    /// an interrupt to the server and waiting for its response. If any error
    /// occurs during the recovery process, the connection is deemed unusable
    /// and the connection closed and an error returned.
    fn recover_from_error(&mut self, err: Error) -> Result<Packet, Error> {
        self.send_marker(constants::MARKER_TYPE_INTERRUPT)
            .and_then(|_| self.reset())
            .map_err(|_| self.unrecoverable_error())
            .and_then(|_| Err(err))
    }

    /// Resets the transport after an error has taken place. All packets
    /// received prior to a reset marker packet being received are discarded.
    /// The packet received after the reset has completed is returned to the
    /// caller. Note that some databases return multiple reset markers so
    /// these are also accommodated.
    fn reset(&mut self) -> Result<Packet, Error> {
        self.send_marker(constants::MARKER_TYPE_RESET)?;
        let mut reset_received = false;
        loop {
            let packet = self.transport.receive_packet()?;
            if packet.has_reset_marker() {
                reset_received = true;
            } else if reset_received {
                return Ok(packet);
            }
        }
    }

    /// Sends a marker packet of the specified type to the database.
    fn send_marker(&mut self, marker_type: u8) -> Result<(), Error> {
        let mut message = MarkerMessage::new(marker_type);
        self.send_message(&mut message)
    }

    /// Sends a message to the database.
    fn send_message(
        &mut self,
        message: &mut impl Message,
    ) -> Result<(), Error> {
        let mut buf = WriteBuffer::new();
        self.write_piggybacks(&mut buf);
        message.serialize(self, &mut buf);
        self.transport.send_packets(
            message.get_packet_type(),
            message.get_packet_flags(),
            message.get_data_flags(),
            buf.get_buf(),
        )?;
        if message.extended_data_needed() {
            buf.clear();
            message.serialize_extended_data(self, &mut buf);
            self.transport.send_packets(
                constants::PACKET_TYPE_DATA,
                0,
                0,
                buf.get_buf(),
            )?;
        }
        Ok(())
    }

    /// Called when an unrecoverable error has taken place and the connection
    /// is no longer deemed usable. The connection is closed and an error is
    /// returned.
    fn unrecoverable_error(&mut self) -> Error {
        let _ = self.transport.close();
        Error::unable_to_recover()
    }

    /// Writes the close cursors piggyback.
    fn write_piggyback_close_cursors(&mut self, buf: &mut WriteBuffer) {
        buf.write_piggyback_header(self, constants::TTC_RPC_CLOSE_CURSORS);
        buf.write_u8(1); // pointer
        let cursors = self.statement_cache.take_cursors_to_close();
        let num_cursors: u32 = cursors.len().try_into().unwrap();
        buf.write_ub4(num_cursors);
        for cursor_id in cursors {
            buf.write_ub2(cursor_id);
        }
    }

    /// Writes the end-to-end attributes piggyback.
    fn write_piggyback_end_to_end(&mut self, buf: &mut WriteBuffer) {
        // determine which flags to send
        let mut flags = 0;
        if self.pending_action.is_some() {
            flags |= constants::TTC_END_TO_END_FLAGS_ACTION;
        }
        if self.pending_client_identifier.is_some() {
            flags |= constants::TTC_END_TO_END_FLAGS_CLIENT_IDENTIFIER;
        }
        if self.pending_client_info.is_some() {
            flags |= constants::TTC_END_TO_END_FLAGS_CLIENT_INFO;
        }
        if self.pending_db_op.is_some() {
            flags |= constants::TTC_END_TO_END_FLAGS_DB_OP;
        }
        if self.pending_module.is_some() {
            // setting the flags for module by itself results in an error so
            // always set the flag for action as well
            flags |= constants::TTC_END_TO_END_FLAGS_MODULE
                | constants::TTC_END_TO_END_FLAGS_ACTION;
        }

        // write initial packet data
        buf.write_piggyback_header(
            self,
            constants::TTC_RPC_SET_END_TO_END_ATTR,
        );
        buf.write_u8(0); // pointer (cidnam)
        buf.write_u8(0); // pointer (cidser)
        buf.write_ub4(flags);

        // write client identifier header info
        if let Some(value) = &self.pending_client_identifier {
            buf.write_u8(1); // pointer (client identifier)
            buf.write_ub4(value.len().try_into().unwrap());
        } else {
            buf.write_u8(0);
            buf.write_ub4(0);
        }

        // write module header info
        if let Some(value) = &self.pending_module {
            buf.write_u8(1); // pointer (module)
            buf.write_ub4(value.len().try_into().unwrap());
        } else {
            buf.write_u8(0);
            buf.write_ub4(0);
        }

        // write action header info
        if let Some(value) = &self.pending_action {
            buf.write_u8(1); // pointer (action)
            buf.write_ub4(value.len().try_into().unwrap());
        } else {
            buf.write_u8(0);
            buf.write_ub4(0);
        }

        // write unsupported bits
        buf.write_u8(0); // pointer (cideci)
        buf.write_ub4(0); // length (cideci)
        buf.write_u8(0); // cidcct
        buf.write_ub4(0); // cidecs

        // write client info header info
        if let Some(value) = &self.pending_client_info {
            buf.write_u8(1); // pointer (client info)
            buf.write_ub4(value.len().try_into().unwrap());
        } else {
            buf.write_u8(0);
            buf.write_ub4(0);
        }

        // write more unsupported bits
        buf.write_u8(0); // pointer (cidkstk)
        buf.write_ub4(0); // length (cidkstk)
        buf.write_u8(0); // pointer (cidktgt)
        buf.write_ub4(0); // length (cidktgt)

        // write database operation header info
        if let Some(value) = &self.pending_db_op {
            buf.write_u8(1); // pointer (database operation)
            buf.write_ub4(value.len().try_into().unwrap());
        } else {
            buf.write_u8(0);
            buf.write_ub4(0);
        }

        // write strings (and reset pending status)
        for pending_value in [
            self.pending_client_identifier.take(),
            self.pending_module.take(),
            self.pending_action.take(),
            self.pending_client_info.take(),
            self.pending_db_op.take(),
        ] {
            if let Some(value) = pending_value
                && !value.is_empty()
            {
                buf.write_bytes_with_length(&value);
            }
        }
    }

    /// Writes the Deep Data Security context piggyback expected by the TTC
    /// protocol.
    fn write_piggyback_end_user_security_context(
        &self,
        buf: &mut WriteBuffer,
        context: &EndUserSecurityContext,
    ) {
        let oson_bytes = context.oson_bytes();
        buf.write_piggyback_header(
            self,
            constants::TTC_RPC_END_USER_SECURITY_CONTEXT,
        );
        buf.write_ub4(TTC_SECURITY_CONTEXT_ATTACH_FLAG);
        buf.write_u8(1); // pointer
        buf.write_ub4(1); // number of key/value pairs

        buf.write_ub4(0); // flags
        buf.write_bytes_with_double_length(Some(
            TTC_END_USER_SECURITY_CONTEXT_KEY.as_bytes(),
        ));
        buf.write_bytes_with_double_length(None); // text
        buf.write_bytes_with_double_length(Some(&oson_bytes));
    }

    /// Writes all of the piggybacks for the given round trip.
    fn write_piggybacks(&mut self, buf: &mut WriteBuffer) {
        if let Some(context) = self.security_context.as_ref() {
            self.write_piggyback_end_user_security_context(buf, context);
        }
        if self.statement_cache.has_cursors_to_close()
            && !self.drcp_establish_session
        {
            self.write_piggyback_close_cursors(buf);
        }
        if self.pending_action.is_some()
            || self.pending_client_identifier.is_some()
            || self.pending_client_info.is_some()
            || self.pending_db_op.is_some()
            || self.pending_module.is_some()
        {
            self.write_piggyback_end_to_end(buf);
        }
    }

    /// Makes any necessary adjustment to the compile time capabilities based
    /// on the server's compile time capabilities.
    pub(crate) fn adjust_for_server_compile_caps(&mut self, caps: &[u8]) {
        self.caps.adjust_for_server_compile_caps(caps);
    }

    /// Makes any necessary adjustment to the runtime capabilities based on the
    /// server's runtime capabilities.
    pub(crate) fn adjust_for_server_runtime_caps(&mut self, caps: &[u8]) {
        self.caps.adjust_for_server_runtime_caps(caps);
    }

    /// Changes the password of the currently logged on user.
    pub(crate) fn change_password(
        &mut self,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), Error> {
        let mut temp_config = self
            .config
            .clone()
            .set_password(old_password)
            .set_new_password(new_password);
        mem::swap(&mut temp_config, &mut self.config);
        let mut auth_message = AuthMessage::new();
        auth_message.set_combo_key(&self.combo_key.unwrap());
        let result = self.process_message(&mut auth_message);
        mem::swap(&mut temp_config, &mut self.config);
        result.map(|_| ())
    }

    /// Removes any Deep Data Security context stored on this client/session.
    pub(crate) fn clear_end_user_security_context(&mut self) {
        self.security_context = None;
    }

    /// Closes the connection to the database.
    pub(crate) fn close(&mut self) -> Result<(), Error> {
        let mut logoff_message = LogoffMessage::new();
        let mut eof_message = EofMessage::new();
        self.process_message(&mut logoff_message)?;
        self.send_message(&mut eof_message)?;
        self.transport.close()
    }

    /// Returns the configuration associated with the client.
    pub(crate) fn config(&self) -> &Config {
        &self.config
    }

    /// Establishes a connection to the database and returns the client object
    /// as well as database info.
    pub(crate) fn connect(&mut self) -> Result<DbInfo, Error> {
        let mut result = Err(Error::unexpected_result());
        let options = self.config.get_options()?;
        for option in options.iter() {
            match option.connect(self) {
                Ok(_) => {
                    return self.connect_phase_two();
                }
                Err(err) => {
                    result = Err(err);
                }
            }
        }
        result
    }

    /// Method for performing the required steps for establishing a connection
    /// within the scope of a retry. Once the accept packet has been received,
    /// no further retries are attempted.
    pub(crate) fn connect_phase_one(
        &mut self,
        sock_addr: SocketAddr,
        connect_data: &str,
        address: &Address,
        description: &Description,
    ) -> Result<(), Error> {
        self.transport.connect(sock_addr)?;
        if address.protocol() == "tcps" {
            self.transport.negotiate_tls(address.host(), &self.config)?;
        }
        let mut connect_message =
            ConnectMessage::new(connect_data, address, description);
        while !connect_message.accepted {
            self.process_message(&mut connect_message)?;
            if connect_message.tls_renegotiation_needed {
                self.transport.negotiate_tls(address.host(), &self.config)?;
            }
        }
        self.caps.adjust_for_protocol(
            connect_message.protocol_version,
            connect_message.protocol_flags,
        );
        self.transport.set_full_packet_size();
        Ok(())
    }

    /// Performs the second phase of connecting to the database. Any errors
    /// that take place during this phase are returned directly to the caller.
    pub(crate) fn connect_phase_two(&mut self) -> Result<DbInfo, Error> {
        // if fast authentication is possible, use it
        if self.caps.supports_fast_auth() {
            let mut fast_auth_message = FastAuthMessage::new();
            self.override_ttc_field_version =
                constants::FAST_AUTH_TTC_FIELD_VERSION;
            self.process_message(&mut fast_auth_message)?;
            fast_auth_message.process_auth_phase_two(self)

        // otherwise, do the normal authentication; disable end of response
        // for the first two messages as the server does not send an end of
        // response for those messages
        } else {
            let orig_value = self.caps.set_supports_end_of_response(false);
            let mut protocol_message = ProtocolMessage::new();
            let mut data_types_message = DataTypesMessage::new();
            let mut auth_message = AuthMessage::new();
            self.process_message(&mut protocol_message)?;
            self.process_message(&mut data_types_message)?;
            self.caps.set_supports_end_of_response(orig_value);
            self.process_message(&mut auth_message)?;
            self.post_connect(&mut auth_message)
        }
    }

    /// Returns the call timeout set on the connection or an error if the
    /// connection is not currently established.
    pub(crate) fn get_call_timeout(
        &self,
    ) -> Result<Option<std::time::Duration>, Error> {
        self.transport.get_read_timeout()
    }

    /// Returns the compile time capabilities.
    pub(crate) fn get_compile_caps(&self) -> &[u8] {
        self.caps.compile_caps()
    }

    /// Returns the last warning that was generated by the client.
    pub(crate) fn get_last_warning(&self) -> Option<String> {
        self.last_warning.clone()
    }

    /// Returns the runtime capabilities.
    pub(crate) fn get_runtime_caps(&self) -> &[u8] {
        self.caps.runtime_caps()
    }

    /// Gets a statement from the cache or creates (and possibly caches) a new
    /// one and returns it.
    pub(crate) fn get_statement(
        &mut self,
        sql: &str,
        cache_statement: bool,
        options: &StatementOptions,
    ) -> Result<CachedStatement, Error> {
        let mut info = self.statement_cache.get_statement(
            sql,
            cache_statement,
            options,
        )?;
        if self.drcp_establish_session {
            info.clear_cursor();
        }
        Ok(info)
    }

    /// Creates a new client and returns it.
    pub(crate) fn new(config: Config) -> Self {
        let cache_size = config.stmtcachesize();
        let sdu = config.get_sdu();
        Self {
            transport: Transport::new(sdu),
            caps: Capabilities::new(),
            config,
            combo_key: None,
            charset_id: 0,
            ncharset_id: 0,
            statement_cache: StatementCache::new(cache_size),
            drcp_establish_session: false,
            override_ttc_field_version: 0,
            pending_error_num: 0,
            pending_action: None,
            pending_client_identifier: None,
            pending_client_info: None,
            pending_db_op: None,
            pending_module: None,
            last_warning: None,
            security_context: None,
        }
    }

    /// Runs activities after the auth message has been processed. The auth
    /// message contains information about the database which is retained.
    pub(crate) fn post_connect(
        &mut self,
        auth_message: &mut AuthMessage,
    ) -> Result<DbInfo, Error> {
        let db_info = DbInfo::new(self, auth_message);
        self.combo_key = auth_message.take_combo_key();
        let max_open_cursors = db_info.get_max_open_cursors();
        if max_open_cursors < self.statement_cache.max_size() {
            self.statement_cache.resize(max_open_cursors);
        }
        Ok(db_info)
    }

    /// Processes a single message and receives back the response. If the
    /// message requires resending, that is done and only the second response
    /// is returned.
    pub(crate) fn process_message(
        &mut self,
        message: &mut impl Message,
    ) -> Result<Response, Error> {
        self.process_message_with_ref(message, None)
    }

    /// Processes a single message and receives back the response. An optional
    /// 'ClientRef' can be provided and will be placed in the response for
    /// later use.
    pub(crate) fn process_message_with_ref(
        &mut self,
        message: &mut impl Message,
        client_ref_opt: Option<&ClientRef>,
    ) -> Result<Response, Error> {
        let mut response = self.perform_round_trip(message, client_ref_opt)?;
        if message.resend_needed() {
            response = self.perform_round_trip(message, client_ref_opt)?;
        }
        Ok(response)
    }

    /// Returns the list of packets making up the response from the server (if
    /// the database is capable of indicating the end of its response) or a
    /// single data packet which may not be the entire response.
    pub(crate) fn receive_packets(&mut self) -> Result<Vec<Packet>, Error> {
        let mut packets = Vec::<Packet>::new();
        let supports_end_of_response = self.supports_end_of_response();
        loop {
            let packet = self.receive_data_packet()?;
            let has_end_of_response = packet.has_end_of_response();
            packets.push(packet);
            if !supports_end_of_response || has_end_of_response {
                break;
            }
        }
        Ok(packets)
    }

    /// Returns the response of the database to the message sent by the client.
    pub(crate) fn receive_response(
        &mut self,
        message: &mut impl Message,
        client_ref_opt: Option<&ClientRef>,
    ) -> Result<Response, Error> {
        let mut packets = self.receive_packets()?;
        loop {
            let mut response = Response::new(&packets);
            if let Some(client_ref) = client_ref_opt {
                response.set_client_ref(client_ref.clone());
            }
            message.pre_deserialize(self, &mut response);
            if let Err(e) = message.deserialize(self, &mut response) {
                if e.is_out_of_data() {
                    packets.extend(self.receive_packets()?);
                    continue;
                }
                return Err(e);
            }
            message.post_deserialize(self, &mut response)?;
            if let Some(warning) = response.take_warning() {
                self.last_warning = Some(warning);
            }
            return Ok(response);
        }
    }

    /// Returns whether the client should be closed based on the pending error
    /// number returned by the latest control packet.
    pub(crate) fn requires_close(&self) -> bool {
        self.pending_error_num == constants::DB_ERR_NUM_SESSION_SHUTDOWN
    }

    /// Resets the TTC field version back to the default value. The necessity
    /// of a reset value is due to the intricacies surrounding fast
    /// authentication.
    pub(crate) fn reset_ttc_field_version(&mut self) {
        self.override_ttc_field_version = 0;
    }

    /// Returns a statement to the statement cache for subsequent use, or
    /// adds it to the list of cursors to close.
    pub(crate) fn return_statement(&mut self, statement: &CachedStatement) {
        self.statement_cache.return_statement(statement);
    }

    /// Sets the call timeout to use on the connection or an error if the
    /// connection is not currently established.
    pub(crate) fn set_call_timeout(
        &self,
        duration: Option<std::time::Duration>,
    ) -> Result<(), Error> {
        self.transport.set_read_timeout(duration)
    }

    /// Sets the ids of the character sets in use by the database. The first
    /// one is the character set used for CHAR data and the second one is the
    /// character set used for NCHAR data.
    pub(crate) fn set_charset_ids(
        &mut self,
        charset_id: u16,
        ncharset_id: u16,
    ) {
        self.charset_id = charset_id;
        self.ncharset_id = ncharset_id;
    }

    /// Stores a Deep Data Security context after validating connection and
    /// server support.
    pub(crate) fn set_end_user_security_context(
        &mut self,
        context: EndUserSecurityContext,
    ) -> Result<(), Error> {
        if !self.transport.uses_tls() {
            return Err(Error::end_user_security_context_requires_tcps());
        }
        if !self.caps.supports_end_user_security_context() {
            return Err(Error::unsupported_deep_data_security_feature());
        }
        self.security_context = Some(context);
        Ok(())
    }

    /// Sets the pending end-to-end attribute (action) which will be sent on
    /// the next round trip to the database.
    pub(crate) fn set_pending_action(&mut self, value: &str) {
        self.pending_action = Some(value.as_bytes().to_vec());
    }

    /// Sets the pending end-to-end attribute (client_identifier) which will be
    /// sent on the next round trip to the database.
    pub(crate) fn set_pending_client_identifier(&mut self, value: &str) {
        self.pending_client_identifier = Some(value.as_bytes().to_vec());
    }

    /// Sets the pending end-to-end attribute (client_info) which will be sent
    /// on the next round trip to the database.
    pub(crate) fn set_pending_client_info(&mut self, value: &str) {
        self.pending_client_info = Some(value.as_bytes().to_vec());
    }

    /// Sets the pending end-to-end attribute (db_op) which will be sent on
    /// the next round trip to the database.
    pub(crate) fn set_pending_db_op(&mut self, value: &str) {
        self.pending_db_op = Some(value.as_bytes().to_vec());
    }

    /// Sets the pending end-to-end attribute (module) which will be sent on
    /// the next round trip to the database.
    pub(crate) fn set_pending_module(&mut self, value: &str) {
        self.pending_module = Some(value.as_bytes().to_vec());
    }

    /// Returns whether the database supports the "end of response" flag.
    pub(crate) fn supports_end_of_response(&self) -> bool {
        self.caps.supports_end_of_response()
    }

    /// Returns whether the client supports a particular TTC field version.
    pub(crate) fn supports_ttc_field_version(&self, version: u8) -> bool {
        if self.override_ttc_field_version != 0 {
            self.override_ttc_field_version >= version
        } else {
            self.caps.supports_ttc_field_version(version)
        }
    }
}
