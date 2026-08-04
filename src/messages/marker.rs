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
// marker.rs
//
// Defines the structure used for sending and receiving marker packets.
// -----------------------------------------------------------------------------

use crate::client::Client;
use crate::constants;
use crate::error::Error;
use crate::messages::Message;
use crate::response::Response;
use crate::write_buffer::WriteBuffer;

pub struct MarkerMessage {
    pub marker_type: u8,
}

impl MarkerMessage {
    pub(crate) fn new(marker_type: u8) -> MarkerMessage {
        MarkerMessage { marker_type }
    }
}

impl Message for MarkerMessage {
    fn deserialize(
        &mut self,
        _client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        resp.advance(2)?;
        self.marker_type = resp.read_u8()?;
        Ok(())
    }

    fn get_packet_type(&self) -> u8 {
        constants::PACKET_TYPE_MARKER
    }

    fn serialize(&self, _client: &Client, buf: &mut WriteBuffer) {
        buf.write_u8(1);
        buf.write_u8(0);
        buf.write_u8(self.marker_type);
    }
}
