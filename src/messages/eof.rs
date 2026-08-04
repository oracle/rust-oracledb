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
// eof.rs
//
// Defines the structure used for sending the final EOF message to the server.
// -----------------------------------------------------------------------------

use crate::client::Client;
use crate::constants;
use crate::messages::Message;
use crate::write_buffer::WriteBuffer;

pub struct EofMessage {}

impl EofMessage {
    pub fn new() -> EofMessage {
        EofMessage {}
    }
}

impl Message for EofMessage {
    fn get_data_flags(&self) -> u16 {
        constants::TTC_DATA_FLAGS_EOF
    }

    fn serialize(&self, _client: &Client, _buf: &mut WriteBuffer) {}
}
