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
// packet.rs
//
// Defines the structure corresponding to a full packet.
//-----------------------------------------------------------------------------

use crate::constants;

pub(crate) struct Packet<'buf>(&'buf [u8]);

impl<'buf> Packet<'buf> {
    pub(crate) fn data(&self) -> &'buf [u8] {
        let header_size = Self::header_size(self.packet_type());
        &self.0[header_size..]
    }

    /// Returns the data flags (bytes 9 and 10 of the header for data packets).
    pub(crate) fn data_flags(&self) -> u16 {
        if self.packet_type() == constants::PACKET_TYPE_DATA {
            u16::from_be_bytes(self.0[8..10].try_into().unwrap())
        } else {
            0
        }
    }

    /// Returns whether the packet contains the end of response marker.
    /// marker is sent for non-data packets, nor for renegotiation packets,
    /// nor for the case
    pub(crate) fn has_end_of_response(&self) -> bool {
        let data = self.data();
        let data_flags = self.data_flags();
        self.packet_type() != constants::PACKET_TYPE_DATA
            || data_flags & constants::TTC_DATA_FLAGS_END_OF_RESPONSE != 0
            || data_flags & constants::TTC_DATA_FLAGS_EOF != 0
            || (data.len() == 1
                && data[0] == constants::TTC_MSG_TYPE_END_OF_RESPONSE)
            || (data.len() > 1
                && data[0] == constants::TTC_MSG_TYPE_RENEGOTIATE)
    }

    /// Returns a boolean indicating if the packet refers to a marker packet
    /// containing a reset marker type.
    pub(crate) fn has_reset_marker(&self) -> bool {
        if self.packet_type() == constants::PACKET_TYPE_MARKER {
            self.0[10] == constants::MARKER_TYPE_RESET
        } else {
            false
        }
    }

    /// Returns the size of the header. Data packets are 10 bytes (because they
    /// include the data flags, unlike other packets).
    pub(crate) fn header_size(packet_type: u8) -> usize {
        if packet_type == constants::PACKET_TYPE_DATA { 10 } else { 8 }
    }

    /// Returns a new packet from the packet buffer.
    pub(crate) fn new(bytes: &'buf [u8]) -> Self {
        Self(bytes)
    }

    /// Returns the packet type (byte 5 of the header).
    pub(crate) fn packet_type(&self) -> u8 {
        self.0[4]
    }

    /// Returns the packet flags (byte 6 of the header).
    pub(crate) fn packet_flags(&self) -> u8 {
        self.0[5]
    }
}
