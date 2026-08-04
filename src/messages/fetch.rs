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
// fetch.rs
//
// Defines the structure used for sending and receiving the fetch message.
// -----------------------------------------------------------------------------

use crate::client::Client;
use crate::constants;
use crate::error::Error;
use crate::messages::Message;
use crate::response::Response;
use crate::row::RowData;
use crate::statement::CachedStatement;
use crate::write_buffer::WriteBuffer;

pub struct FetchMessage<'statement> {
    statement: &'statement CachedStatement,
    last_row: Option<RowData>,
}

impl FetchMessage<'_> {
    pub fn new<'statement>(
        statement: &'statement CachedStatement,
        last_row: Option<RowData>,
    ) -> FetchMessage<'statement> {
        FetchMessage {
            statement,
            last_row,
        }
    }
}

impl Message for FetchMessage<'_> {
    /// Deserializes a TTC row data message.
    fn deserialize_row_data(
        &mut self,
        client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        resp.deserialize_row_data(client, self.statement, true)
    }

    fn post_deserialize(
        &mut self,
        _client: &mut Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        resp.check_for_end_of_fetch(self.statement)
    }

    fn pre_deserialize(&mut self, _client: &mut Client, resp: &mut Response) {
        resp.set_prev_fetch_last_row(self.last_row.take());
        resp.set_num_columns(self.statement.out_metadata().len());
    }

    fn serialize(&self, client: &Client, buf: &mut WriteBuffer) {
        buf.write_function_header(client, constants::TTC_RPC_FETCH);
        buf.write_ub2(self.statement.cursor_id());
        buf.write_ub4(self.statement.options().fetch_array_size());
    }
}
