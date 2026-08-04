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
// execute.rs
//
// Defines the structure used for sending and receiving the execute message.
// -----------------------------------------------------------------------------

use crate::bind_params::BindParameters;
use crate::client::Client;
use crate::constants;
use crate::error::Error;
use crate::messages::Message;
use crate::response::Response;
use crate::rowid::Rowid;
use crate::statement::CachedStatement;
use crate::write_buffer::WriteBuffer;

pub struct ExecuteMessage<'statement, 'params> {
    scroll_operation: bool,
    parse_only: bool,
    batch_errors: bool,
    array_dml_row_counts: bool,
    num_execs: u32,
    num_fetch_columns: u32,
    autocommit: bool,
    fetch_orientation: u32,
    fetch_pos: u32,
    statement: &'statement mut CachedStatement,
    params: BindParameters<'params>,
}

impl ExecuteMessage<'_, '_> {
    pub fn new<'statement, 'params>(
        statement: &'statement mut CachedStatement,
        params: BindParameters<'params>,
    ) -> ExecuteMessage<'statement, 'params> {
        ExecuteMessage {
            parse_only: false,
            scroll_operation: false,
            batch_errors: false,
            array_dml_row_counts: false,
            num_execs: params.num_rows().try_into().unwrap(),
            num_fetch_columns: 0,
            autocommit: false,
            fetch_orientation: 0,
            fetch_pos: 0,
            statement,
            params,
        }
    }

    /// Writes bind metadata to the buffer.
    fn write_bind_metadata(&self, client: &Client, buf: &mut WriteBuffer) {
        for bind_info in self.statement.binds() {
            let metadata = bind_info.metadata.as_ref().unwrap();
            metadata.write_to_buf(buf, client);
        }
    }

    /// Writes bind parameter data to the buffer.
    fn write_bind_params(&self, buf: &mut WriteBuffer) {
        let binds = self.statement.binds();
        for row_index in 0..self.params.num_rows() {
            buf.write_u8(constants::TTC_MSG_TYPE_ROW_DATA);
            for (column_index, bind_info) in binds.iter().enumerate() {
                if bind_info.is_input_bind() {
                    self.params.write_to_buf(
                        row_index,
                        column_index,
                        bind_info,
                        buf,
                    );
                }
            }
        }
    }

    fn write_define_metadata(&self, client: &Client, buf: &mut WriteBuffer) {
        for metadata in self.statement.out_metadata() {
            metadata.write_to_buf(buf, client);
        }
    }

    fn write_full_execute(&self, client: &Client, buf: &mut WriteBuffer) {
        // determine options and flags to use
        let mut options: u32 = 0;
        let mut exec_flags: u32 = 0;
        let mut num_iters: u32 = 0;
        let statement = &self.statement;
        let has_binds = statement.has_binds()
            && !statement.requires_define()
            && !self.parse_only;
        if statement.requires_define() {
            options |= constants::TTC_EXEC_OPTION_DEFINE;
        } else if !self.parse_only && !statement.sql().is_empty() {
            exec_flags |= constants::TTC_EXEC_FLAGS_IMPLICIT_RESULTSET;
            if !self.scroll_operation {
                options |= constants::TTC_EXEC_OPTION_EXECUTE;
            }
        }
        if !statement.has_cursor() || statement.is_ddl() {
            options |= constants::TTC_EXEC_OPTION_PARSE;
        }
        if statement.is_query() {
            if self.parse_only {
                options |= constants::TTC_EXEC_OPTION_DESCRIBE;
            } else {
                if !statement.has_cursor() || statement.requires_define() {
                    num_iters = self.statement.options().prefetch_rows();
                } else {
                    num_iters = self.statement.options().fetch_array_size();
                }
                if num_iters > 0 && !statement.no_prefetch() {
                    options |= constants::TTC_EXEC_OPTION_FETCH;
                }
            }
        }
        if !statement.is_plsql() && !self.parse_only {
            options |= constants::TTC_EXEC_OPTION_NOT_PLSQL;
        } else if statement.is_plsql() && has_binds {
            options |= constants::TTC_EXEC_OPTION_PLSQL_BIND;
        }
        if has_binds {
            options |= constants::TTC_EXEC_OPTION_BIND;
        }
        if self.batch_errors {
            options |= constants::TTC_EXEC_OPTION_BATCH_ERRORS;
        }
        if self.array_dml_row_counts {
            options |= constants::TTC_EXEC_FLAGS_DML_ROWCOUNTS;
        }
        if self.autocommit && !self.parse_only {
            options |= constants::TTC_EXEC_OPTION_COMMIT;
        }

        // write message
        buf.write_function_header(client, constants::TTC_RPC_EXECUTE);
        buf.write_ub4(options);
        buf.write_ub2(statement.cursor_id());
        if !statement.has_cursor() || statement.is_ddl() {
            buf.write_u8(1); // pointer (cursor id)
            buf.write_ub4(statement.sql_len());
        } else {
            buf.write_u8(0); // pointer (cursor id)
            buf.write_ub4(0); // SQL length
        }
        buf.write_u8(1); // pointer (vector)
        buf.write_ub4(13); // al8i4 array length
        buf.write_u8(0); // pointer (al8o4)
        buf.write_u8(0); // pointer (al8o4l)
        buf.write_ub4(0); // prefetch buffer size
        buf.write_ub4(num_iters); // number of rows to fetch
        buf.write_ub4(0x7fffffff); // maximum long size
        if has_binds {
            let num_binds: u32 = statement.binds().len().try_into().unwrap();
            buf.write_u8(1); // pointer (binds)
            buf.write_ub4(num_binds);
        } else {
            buf.write_u8(0); // pointer (binds)
            buf.write_ub4(0); // number of binds
        }
        buf.write_u8(0); // pointer (al8app)
        buf.write_u8(0); // pointer (al8txn)
        buf.write_u8(0); // pointer (al8txl)
        buf.write_u8(0); // pointer (al8kv)
        buf.write_u8(0); // pointer (al8kvl)
        if statement.requires_define() {
            buf.write_u8(1); // pointer (al8doac)
            buf.write_ub4(self.num_fetch_columns);
        } else {
            buf.write_u8(0); // pointer (al8doac)
            buf.write_u8(0); // number of defines
        }
        buf.write_ub4(0); // registration id
        buf.write_u8(0); // pointer (al8objlist)
        buf.write_u8(1); // pointer (al8objlen)
        buf.write_u8(0); // pointer (al8blv)
        buf.write_ub4(0); // al8blvl
        buf.write_u8(0); // pointer (al8dnam)
        buf.write_ub4(0); // al8dnaml
        buf.write_ub4(0); // al8regid_msb
        if self.array_dml_row_counts {
            buf.write_u8(1); // pointer (al8pidmlrc)
            buf.write_ub4(self.num_execs);
            buf.write_u8(1); // pointer (al8pidmlrcl)
        } else {
            buf.write_u8(0); // pointer (al8pidmlrc)
            buf.write_ub4(0); // al8pidmlrcbl
            buf.write_u8(0); // pointer (al8pidmlrcl)
        }
        if client.supports_ttc_field_version(constants::TTC_FIELD_VERSION_12_2)
        {
            buf.write_u8(0); // pointer (al8sqlsig)
            buf.write_ub4(0); // SQL signature length
            buf.write_u8(0); // pointer (SQL ID)
            buf.write_ub4(0); // allocated size of SQL ID
            buf.write_u8(0); // pointer (length of SQL ID)
        }
        if client
            .supports_ttc_field_version(constants::TTC_FIELD_VERSION_12_2_EXT1)
        {
            buf.write_u8(0); // pointer (chunk ids)
            buf.write_ub4(0); // number of chunk ids
        }
        if !statement.has_cursor() || statement.is_ddl() {
            if statement.sql().is_empty() {
                todo!();
            }
            let sql_bytes = statement.sql().as_bytes();
            buf.write_bytes_with_length(sql_bytes);
            buf.write_ub4(1); // al8i4[0] parse
        } else {
            buf.write_ub4(0); // al8i4[0] parse
        }
        if statement.is_query() {
            if statement.has_cursor() {
                buf.write_ub4(num_iters);
            } else {
                buf.write_ub4(0); // al8i4[1] execution count
            }
        } else {
            buf.write_ub4(self.num_execs);
        }
        buf.write_ub4(0); // al8i4[2]
        buf.write_ub4(0); // al8i4[3]
        buf.write_ub4(0); // al8i4[4]
        buf.write_ub4(0); // al8i4[5] SCN (part 1)
        buf.write_ub4(0); // al8i4[6] SCN (part 2)
        buf.write_ub4(statement.is_query() as u32);
        buf.write_ub4(0); // al8i4[8]
        buf.write_ub4(exec_flags);
        buf.write_ub4(self.fetch_orientation);
        buf.write_ub4(self.fetch_pos);
        buf.write_ub4(0); // al8i4[12]
        if statement.requires_define() {
            self.write_define_metadata(client, buf);
        } else if has_binds {
            self.write_bind_metadata(client, buf);
            if self.statement.has_input_binds() {
                self.write_bind_params(buf);
            }
        }
    }

    /// Writes the re-execute message to the buffer.
    fn write_reexecute(&self, client: &Client, buf: &mut WriteBuffer) {
        // determine options and flags to use
        let fn_type: u8;
        let mut options_1: u32 = 0;
        let options_2: u32 = 0;
        let num_iters: u32;
        if self.statement.is_query()
            && self.statement.options().prefetch_rows() > 0
        {
            fn_type = constants::TTC_RPC_REEXECUTE_AND_FETCH;
            num_iters = self.statement.options().prefetch_rows();
            options_1 |= constants::TTC_EXEC_OPTION_EXECUTE;
        } else {
            fn_type = constants::TTC_RPC_REEXECUTE;
            num_iters = self.num_execs;
        }

        // write message
        buf.write_function_header(client, fn_type);
        buf.write_ub2(self.statement.cursor_id());
        buf.write_ub4(num_iters);
        buf.write_ub4(options_1);
        buf.write_ub4(options_2);
        if self.statement.has_input_binds() {
            self.write_bind_params(buf);
        }
    }
}

impl Message for ExecuteMessage<'_, '_> {
    fn deserialize_describe_info(
        &mut self,
        client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        resp.read_bytes_with_length()?;
        self.statement.populate_from_describe_info(client, resp)?;
        let num_metadata = self.statement.out_metadata().len();
        self.num_fetch_columns = num_metadata.try_into().unwrap();
        Ok(())
    }

    fn deserialize_io_vector(
        &mut self,
        _client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        let _flags = resp.read_u8()?;
        let _num_requests = resp.read_ub2()? as u32;
        let _num_iters = resp.read_ub4()?;
        let _num_binds = _num_iters * 256 + _num_requests;
        let _num_iters_this_time = resp.read_ub4()?;
        let _uac_buffer_length = resp.read_ub2()?;
        resp.read_bit_vector()?;
        let _rowid: Option<Rowid> = {
            let num_bytes = resp.read_ub2()?;
            if num_bytes == 0 {
                None
            } else {
                Some(Rowid::deserialize(resp)?)
            }
        };
        self.statement.set_bind_directions(resp)
    }

    fn deserialize_return_parameters(
        &mut self,
        _client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        for _ in 0..resp.read_ub2()? {
            resp.read_ub4()?; // al804l
        }
        let mut num_bytes = resp.read_ub2()?;
        if num_bytes > 0 {
            resp.advance(num_bytes.into())?; // al8txl
        }
        let num_pairs = resp.read_ub2()?;
        resp.process_keyword_value_pairs(num_pairs)?;
        num_bytes = resp.read_ub2()?;
        if num_bytes > 0 {
            resp.advance(num_bytes.into())?; // registration
        }
        if self.array_dml_row_counts {
            todo!();
        }
        Ok(())
    }

    /// Deserializes a TTC row data message.
    fn deserialize_row_data(
        &mut self,
        client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        resp.deserialize_row_data(
            client,
            self.statement,
            self.statement.is_query(),
        )
    }

    fn post_deserialize(
        &mut self,
        _client: &mut Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        self.statement.set_cursor_id(resp.get_cursor_id());
        resp.check_for_end_of_fetch(self.statement)
    }

    fn resend_needed(&self) -> bool {
        self.statement.requires_define()
    }

    fn serialize(&self, client: &Client, buf: &mut WriteBuffer) {
        if !self.statement.has_cursor()
            || self.statement.no_prefetch()
            || self.statement.binds_changed()
            || self.parse_only
            || self.statement.requires_define()
            || self.statement.is_ddl()
        {
            self.write_full_execute(client, buf);
        } else {
            self.write_reexecute(client, buf);
        }
    }
}
