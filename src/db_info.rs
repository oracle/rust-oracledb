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
// db_info.rs
//
// Defines the structure used for providing information about the database.
//-----------------------------------------------------------------------------

use crate::client::Client;
use crate::messages::AuthMessage;
use crate::ora_version::OracleVersion;

pub(crate) struct DbInfo {
    max_open_cursors: usize,
    max_identifier_length: usize,
    session_id: usize,
    serial_num: usize,
    db_domain: String,
    db_name: String,
    service_name: String,
    instance_name: String,
    server_version: OracleVersion,
}

impl DbInfo {
    /// Returns the domain of the database.
    pub(crate) fn get_db_domain(&self) -> &str {
        &self.db_domain
    }

    /// Returns the name of the database.
    pub(crate) fn get_db_name(&self) -> &str {
        &self.db_name
    }

    /// Returns the instance name used to connect to the database.
    pub(crate) fn get_instance_name(&self) -> &str {
        &self.instance_name
    }

    /// Returns the maximum number of bytes allowed to be used in identifiers.
    pub(crate) fn get_max_identifier_length(&self) -> usize {
        self.max_identifier_length
    }

    /// Returns the maximum number of open cursors allowed by the database.
    pub(crate) fn get_max_open_cursors(&self) -> usize {
        self.max_open_cursors
    }

    /// Returns the serial number of the connection to the database.
    pub(crate) fn get_serial_num(&self) -> usize {
        self.serial_num
    }

    /// Returns the version of the database.
    pub(crate) fn get_server_version(&self) -> OracleVersion {
        self.server_version.clone()
    }

    /// Returns the service name used to connect to the database.
    pub(crate) fn get_service_name(&self) -> &str {
        &self.service_name
    }

    /// Returns the session id of the connection to the database.
    pub(crate) fn get_session_id(&self) -> usize {
        self.session_id
    }

    /// Returns a new structure populated from the client and auth message.
    pub(crate) fn new(client: &Client, auth_message: &AuthMessage) -> DbInfo {
        DbInfo {
            max_open_cursors: auth_message.get_max_open_cursors(),
            max_identifier_length: auth_message.get_max_identifier_length(),
            session_id: auth_message.get_session_id(),
            serial_num: auth_message.get_serial_num(),
            db_domain: auth_message.get_db_domain(),
            db_name: auth_message.get_db_name(),
            service_name: auth_message.get_service_name(),
            instance_name: auth_message.get_instance_name(),
            server_version: auth_message.get_server_version(client),
        }
    }
}
