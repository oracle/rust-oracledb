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
// connect_options.rs
//
// Defines the structures used for defining the options for connecting to the
// database that are found in a connect string.
//-----------------------------------------------------------------------------

use super::Config;
use super::connect_string_parser::Node;

use crate::client::Client;
use crate::error::Error;

use base64ct::Encoding;
use rand::RngExt;
use rand::prelude::IndexedRandom;
use rand::prelude::SliceRandom;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::thread;
use std::time::Duration;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Address {
    host: Option<String>,
    port: Option<u16>,
    protocol: Option<String>,
    https_proxy: Option<String>,
    https_proxy_port: Option<u16>,
}

impl Address {
    /// Returns a connect string fragment for the address.
    fn build_connect_string(&self) -> String {
        let mut parts = Vec::<String>::new();
        parts.push(format!("(PROTOCOL={})", self.protocol()));
        parts.push(format!("(HOST={})", self.host()));
        parts.push(format!("(PORT={})", self.port()));
        if let Some(https_proxy) = self.https_proxy.as_ref() {
            parts.push(format!("(HTTPS_PROXY={}", https_proxy));
        }
        if let Some(https_proxy_port) = self.https_proxy_port {
            parts.push(format!("(HTTPS_PROXY_PORT={}", https_proxy_port));
        }
        format!("(ADDRESS={})", parts.join(""))
    }

    /// Creates a new address from a full descriptor node and returns it.
    fn new_from_node(node: &Node) -> Result<Address, Error> {
        let mut address = Address::new(None, None);
        node.process_child_nodes(|n| address.process_nodes(n))?;
        Ok(address)
    }

    /// Processes nodes in the ADDRESS section of a full descriptor.
    fn process_nodes(&mut self, node: &Node) -> Result<(), Error> {
        match node.key() {
            "host" => {
                self.host = Some(node.as_str()?);
            }
            "https_proxy" => {
                self.https_proxy = Some(node.as_str()?);
            }
            "https_proxy_port" => {
                self.https_proxy_port = Some(node.as_u16()?);
            }
            "port" => {
                self.port = Some(node.as_u16()?);
            }
            "protocol" => {
                self.protocol = Some(node.as_str()?.to_lowercase());
            }
            _ => {}
        }
        Ok(())
    }

    /// Returns the host to use. If a host was not specified, it defaults to
    /// "localhost".
    pub(crate) fn host(&self) -> &str {
        self.host.as_deref().unwrap_or("localhost")
    }

    /// Creates a new address and returns it (from an Easy Connect string).
    pub(crate) fn new(
        host: Option<String>,
        protocol: Option<String>,
    ) -> Address {
        Address {
            host,
            port: None,
            protocol,
            https_proxy: None,
            https_proxy_port: None,
        }
    }

    /// Returns the port to sue. If a port was not specified, it defaults to
    /// 1521.
    pub(crate) fn port(&self) -> u16 {
        self.port.unwrap_or(1521)
    }

    /// Returns the protocol to use. If a protocol is not specified, it
    /// defaults to "tcp".
    pub(crate) fn protocol(&self) -> &str {
        self.protocol.as_deref().unwrap_or("tcp")
    }

    /// Sets the port to the given value.
    pub(crate) fn set_port(&mut self, port: Option<u16>) {
        self.port = port;
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct AddressList {
    source_route: Option<bool>,
    load_balance: Option<bool>,
    failover: Option<bool>,
    pub(crate) addresses: Vec<Address>,
}

impl AddressList {
    /// Returns a connect string fragment for the address list.
    fn build_connect_string(&self) -> String {
        let mut parts = Vec::<String>::new();
        if !self.failover() {
            parts.push("(FAILOVER=OFF)".into());
        }
        if self.load_balance() {
            parts.push("(LOAD_BALANCE=ON)".into());
        }
        if self.source_route() {
            parts.push("(SOURCE_ROUTE=ON)".into());
        }
        for address in self.addresses.iter() {
            parts.push(address.build_connect_string());
        }
        if parts.len() == 1 {
            parts.pop().unwrap()
        } else {
            format!("(ADDRESS_LIST={})", parts.join(""))
        }
    }

    /// Returns whether failover should be used.
    fn failover(&self) -> bool {
        self.failover.unwrap_or(true)
    }

    /// Returns whether load balancing should be used.
    fn load_balance(&self) -> bool {
        self.load_balance.unwrap_or(false)
    }

    /// Creates a new address list from a full descriptor node and returns it.
    fn new_from_node(node: &Node) -> Result<AddressList, Error> {
        let mut address_list = AddressList::new();
        node.process_child_nodes(|n| address_list.process_nodes(n))?;
        Ok(address_list)
    }

    /// Processes nodes in the ADDRESS_LIST section of a full descriptor.
    fn process_nodes(&mut self, node: &Node) -> Result<(), Error> {
        match node.key() {
            "address" => {
                self.addresses.push(Address::new_from_node(node)?);
            }
            "failover" => {
                self.failover = Some(node.as_bool()?);
            }
            "load_balance" => {
                self.load_balance = Some(node.as_bool()?);
            }
            "source_route" => {
                self.source_route = Some(node.as_bool()?);
            }
            _ => {}
        }
        Ok(())
    }

    /// Returns whether source routing should be used.
    fn source_route(&self) -> bool {
        self.source_route.unwrap_or(false)
    }

    /// Returns whether the address list has any addresses which use the
    /// TCPS protocol.
    fn uses_tcps(&self) -> bool {
        for address in self.addresses.iter() {
            if let Some(protocol) = address.protocol.as_ref()
                && protocol == "tcps"
            {
                return true;
            }
        }
        false
    }

    /// Returns a new empty address list.
    pub(crate) fn new() -> AddressList {
        AddressList {
            source_route: None,
            load_balance: None,
            failover: None,
            addresses: Vec::<Address>::new(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Description {
    source_route: Option<bool>,
    load_balance: Option<bool>,
    failover: Option<bool>,
    expire_time: Option<u32>,
    retry_count: Option<u32>,
    retry_delay: Option<Duration>,
    sdu: Option<u32>,
    tcp_connect_timeout: Option<Duration>,
    service_name: Option<String>,
    instance_name: Option<String>,
    server_type: Option<String>,
    sid: Option<String>,
    cclass: Option<String>,
    connection_id: Option<String>,
    connection_id_prefix: Option<String>,
    pool_boundary: Option<String>,
    pool_name: Option<String>,
    purity: Option<u8>,
    ssl_server_dn_match: Option<bool>,
    use_tcp_fast_open: Option<bool>,
    use_sni: Option<bool>,
    ssl_server_cert_dn: Option<String>,
    wallet_location: Option<String>,
    pub(crate) address_lists: Vec<AddressList>,
}

impl Description {
    /// Builds the CID segment of a full descriptor.
    fn build_cid_segment(&self, config: &Config) -> String {
        format!(
            "(PROGRAM={})(HOST={})(USER={})",
            config.program(),
            config.machine(),
            config.osuser()
        )
    }

    /// Builds the DESCRIPTION segment of a full descriptor with connection
    /// identification.
    fn build_connect_data(&self, config: &Config) -> String {
        self.build_description_segment(Some(config))
    }

    /// Builds the CONNECT_DATA segment of a full descriptor.
    fn build_connect_data_segment(
        &self,
        config_opt: Option<&Config>,
    ) -> String {
        let mut parts = Vec::<String>::new();
        if let Some(service_name) = self.service_name.as_ref() {
            parts.push(format!("(SERVICE_NAME={})", service_name));
        }
        if let Some(instance_name) = self.instance_name.as_ref() {
            parts.push(format!("(INSTANCE_NAME={})", instance_name));
        } else if let Some(sid) = self.sid.as_ref() {
            parts.push(format!("(SID={})", sid));
        }
        if let Some(server_type) = self.server_type.as_ref() {
            parts.push(format!("(SERVER={})", server_type));
        }
        if self.use_tcp_fast_open() {
            parts.push("(USE_TCP_FAST_OPEN=ON)".into());
        }
        if let Some(pool_boundary) = self.pool_boundary.as_ref() {
            parts.push(format!("(POOL_BOUNDARY={})", pool_boundary));
        }
        if let Some(config) = config_opt {
            parts.push(self.build_cid_segment(config));
        }
        if let Some(connection_id) = self.connection_id.as_deref() {
            parts.push(format!("(CONNECTION_ID={})", connection_id));
        }
        format!("(CONNECT_DATA={})", parts.join(""))
    }

    /// Builds the DESCRIPTION segment of a full descriptor without any
    /// connection identification.
    fn build_connect_string(&self) -> String {
        self.build_description_segment(None)
    }

    /// Builds the DESCRIPTION segment of a full descriptor.
    fn build_description_segment(
        &self,
        config_opt: Option<&Config>,
    ) -> String {
        let mut parts = Vec::<String>::new();
        if !self.failover() {
            parts.push("(FAILOVER=OFF)".into());
        }
        if self.load_balance() {
            parts.push("(LOAD_BALANCE=ON)".into());
        }
        if self.source_route() {
            parts.push("(SOURCE_ROUTE=ON)".into());
        }
        if let Some(retry_count) = self.retry_count {
            parts.push(format!("(RETRY_COUNT={})", retry_count));
        }
        if let Some(retry_delay) = self.retry_delay {
            parts.push(format!("(RETRY_DELAY={})", retry_delay.as_secs()));
        }
        if let Some(expire_time) = self.expire_time {
            parts.push(format!("(EXPIRE_TIME={})", expire_time));
        }
        if self.use_sni() {
            parts.push("(USE_SNI=ON)".into());
        }
        if let Some(sdu) = self.sdu {
            parts.push(format!("(SDU={})", sdu));
        }
        let mut uses_tcps = false;
        for address_list in self.address_lists.iter() {
            parts.push(address_list.build_connect_string());
            if !uses_tcps {
                uses_tcps = address_list.uses_tcps();
            }
        }
        parts.push(self.build_connect_data_segment(config_opt));
        if uses_tcps {
            parts.push(self.build_security_segment());
        }
        format!("(DESCRIPTION={})", parts.join(""))
    }

    /// Builds the SECURITY segment of a full descriptor.
    fn build_security_segment(&self) -> String {
        let mut parts = Vec::<String>::new();
        if self.ssl_server_dn_match() {
            parts.push("(SSL_SERVER_DN_MATCH=ON)".into());
        }
        if let Some(ssl_server_cert_dn) = self.ssl_server_cert_dn.as_ref() {
            parts.push(format!("(SSL_SERVER_CERT_DN={})", ssl_server_cert_dn));
        }
        if let Some(wallet_location) = self.wallet_location.as_ref() {
            parts.push(format!("(MY_WALLET_DIRECTORY={})", wallet_location));
        }
        format!("(SECURITY={})", parts.join(""))
    }

    /// Returns whether failover should be used.
    fn failover(&self) -> bool {
        self.failover.unwrap_or(true)
    }

    /// Returns whether load balancing should be used.
    fn load_balance(&self) -> bool {
        self.load_balance.unwrap_or(false)
    }

    /// Processes nodes in the CONNECT_DATA section of a full descriptor.
    fn process_connect_data_nodes(
        &mut self,
        node: &Node,
    ) -> Result<(), Error> {
        match node.key() {
            "connection_id_prefix" => {
                self.connection_id_prefix = Some(node.as_str()?);
            }
            "instance_name" => {
                self.instance_name = Some(node.as_str()?);
            }
            "pool_boundary" => {
                self.pool_boundary = Some(node.as_str()?);
            }
            "pool_name" => {
                self.pool_name = Some(node.as_str()?);
            }
            "pool_connection_class" => {
                self.cclass = Some(node.as_str()?);
            }
            "pool_purity" => {
                self.purity = Some(node.as_purity()?);
            }
            "server" => {
                self.server_type = Some(node.as_server_type()?);
            }
            "service_name" => {
                self.service_name = Some(node.as_str()?);
            }
            "sid" => {
                self.sid = Some(node.as_str()?);
            }
            "use_tcp_fast_open" => {
                self.use_tcp_fast_open = Some(node.as_bool()?);
            }
            _ => {}
        }
        Ok(())
    }

    /// Processes nodes in the DESCRIPTION section of a full descriptor.
    fn process_nodes(&mut self, node: &Node) -> Result<(), Error> {
        match node.key() {
            "address_list" => {
                self.address_lists.push(AddressList::new_from_node(node)?);
            }
            "connect_data" => {
                node.process_child_nodes(|n| {
                    self.process_connect_data_nodes(n)
                })?;
            }
            "expire_time" => {
                self.expire_time = Some(node.as_u32()?);
            }
            "failover" => {
                self.failover = Some(node.as_bool()?);
            }
            "load_balance" => {
                self.load_balance = Some(node.as_bool()?);
            }
            "retry_count" => {
                self.retry_count = Some(node.as_u32()?);
            }
            "retry_delay" => {
                let value = node.as_u64()?;
                self.retry_delay = Some(Duration::from_secs(value));
            }
            "sdu" => {
                let value = node.as_u32()?;
                self.sdu = Some(value.clamp(512, 2097152));
            }
            "security" => {
                node.process_child_nodes(|n| self.process_security_nodes(n))?;
            }
            "source_route" => {
                self.source_route = Some(node.as_bool()?);
            }
            "use_sni" => {
                self.use_sni = Some(node.as_bool()?);
            }
            _ => {}
        }
        Ok(())
    }

    /// Processes nodes in the SECURITY section of a full descriptor.
    fn process_security_nodes(&mut self, node: &Node) -> Result<(), Error> {
        match node.key() {
            "ssl_server_cert_dn" => {
                self.ssl_server_cert_dn = Some(node.as_str()?);
            }
            "ssl_server_dn_match" => {
                self.ssl_server_dn_match = Some(node.as_bool()?);
            }
            "wallet_location" => {
                self.wallet_location = Some(node.as_str()?);
            }
            _ => {}
        }
        Ok(())
    }

    /// Returns the number of times to retry the connection with this
    /// description.
    fn retry_count(&self) -> u32 {
        self.retry_count.unwrap_or(0)
    }

    /// Returns the duration to wait before retrying the connection with this
    /// description again.
    fn retry_delay(&self) -> Duration {
        self.retry_delay.unwrap_or(Duration::from_secs(1))
    }

    /// Sets the connection id used for identifying connections to the
    /// database.
    fn set_connection_id(&mut self, connection_id: &str) {
        if let Some(prefix) = self.connection_id_prefix.as_deref() {
            self.connection_id = Some(prefix.to_owned() + connection_id);
        } else {
            self.connection_id = Some(connection_id.into());
        }
    }

    /// Returns whether source routing should be used.
    fn source_route(&self) -> bool {
        self.source_route.unwrap_or(false)
    }

    /// Returns whether SSL server DN matching should take place.
    fn ssl_server_dn_match(&self) -> bool {
        self.ssl_server_dn_match.unwrap_or(true)
    }

    /// Returns whether special SNI processing should take place.
    fn use_sni(&self) -> bool {
        self.use_sni.unwrap_or(false)
    }

    /// Returns whether TCP fast open should be used.
    fn use_tcp_fast_open(&self) -> bool {
        self.use_tcp_fast_open.unwrap_or(false)
    }

    /// Returns the CONNECTION_ID associated with the description.
    pub(crate) fn connection_id(&self) -> &str {
        self.connection_id.as_deref().unwrap_or("")
    }

    /// Creates a new empty description and returns it.
    pub(crate) fn new() -> Description {
        Description {
            source_route: None,
            load_balance: None,
            failover: None,
            expire_time: None,
            retry_count: None,
            retry_delay: None,
            sdu: None,
            tcp_connect_timeout: None,
            service_name: None,
            instance_name: None,
            server_type: None,
            sid: None,
            cclass: None,
            connection_id: None,
            connection_id_prefix: None,
            pool_boundary: None,
            pool_name: None,
            purity: None,
            ssl_server_dn_match: None,
            use_tcp_fast_open: None,
            use_sni: None,
            ssl_server_cert_dn: None,
            wallet_location: None,
            address_lists: Vec::<AddressList>::new(),
        }
    }

    /// Creates a new description from a full descriptor node and returns it.
    pub(crate) fn new_from_node(node: &Node) -> Result<Description, Error> {
        let mut description = Description::new();
        node.process_child_nodes(|n| description.process_nodes(n))?;
        Ok(description)
    }

    /// Returns the size of the SDU to use.
    pub(crate) fn sdu(&self) -> u32 {
        self.sdu.unwrap_or(8192)
    }

    /// Returns the service name associated with the description.
    pub(crate) fn service_name(&self) -> &str {
        self.service_name.as_deref().unwrap_or("")
    }

    /// Sets the instance name.
    pub(crate) fn set_instance_name(&mut self, value: Option<String>) {
        self.instance_name = value;
    }

    /// Sets the server type.
    pub(crate) fn set_server_type(&mut self, value: Option<String>) {
        self.server_type = value;
    }

    /// Sets the service name.
    pub(crate) fn set_service_name(&mut self, value: Option<String>) {
        self.service_name = value;
    }

    /// Returns the SID associated with the description.
    pub(crate) fn sid(&self) -> &str {
        self.sid.as_deref().unwrap_or("")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct DescriptionList {
    source_route: Option<bool>,
    load_balance: Option<bool>,
    failover: Option<bool>,
    pub(crate) descriptions: Vec<Description>,
}

impl DescriptionList {
    /// Returns whether failover should be used.
    fn failover(&self) -> bool {
        self.failover.unwrap_or(true)
    }

    /// Returns whether load balancing should be used.
    fn load_balance(&self) -> bool {
        self.load_balance.unwrap_or(false)
    }

    /// Processes nodes in the DESCRIPTION_LIST section of a full descriptor.
    fn process_nodes(&mut self, node: &Node) -> Result<(), Error> {
        match node.key() {
            "description" => {
                self.descriptions.push(Description::new_from_node(node)?);
            }
            "failover" => {
                self.failover = Some(node.as_bool()?);
            }
            "load_balance" => {
                self.load_balance = Some(node.as_bool()?);
            }
            "source_route" => {
                self.source_route = Some(node.as_bool()?);
            }
            _ => {}
        }
        Ok(())
    }

    /// Returns whether source routing should be used.
    fn source_route(&self) -> bool {
        self.source_route.unwrap_or(false)
    }

    /// Builds the DESCRRIPTION_LIST segment of a full descriptor.
    pub(crate) fn build_connect_string(&self) -> String {
        let mut parts = Vec::<String>::new();
        if !self.failover() {
            parts.push("(FAILOVER=OFF)".into());
        }
        if self.load_balance() {
            parts.push("(LOAD_BALANCE=ON)".into());
        }
        if self.source_route() {
            parts.push("(SOURCE_ROUTE=ON)".into());
        }
        for description in self.descriptions.iter() {
            parts.push(description.build_connect_string());
        }
        if parts.len() == 1 {
            parts.pop().unwrap()
        } else {
            format!("(DESCRIPTION_LIST={})", parts.join(""))
        }
    }

    /// Returns the list of description options to use when attempting to
    /// connect to the database.
    pub(crate) fn get_options(
        &self,
        config: &Config,
    ) -> Result<Vec<DescriptionOption>, Error> {
        let mut connection_id_buf = [0u8; 16];
        rand::rng().fill(&mut connection_id_buf);
        let connection_id =
            base64ct::Base64Unpadded::encode_string(&connection_id_buf);
        let mut options = Vec::<DescriptionOption>::new();
        let descriptions = calc_active_children(
            &self.descriptions,
            self.source_route(),
            self.failover(),
            self.load_balance(),
        );
        for mut description in descriptions {
            description.set_connection_id(&connection_id);
            options.push(DescriptionOption::new(description, config)?);
        }
        Ok(options)
    }

    /// Creates a new empty description list and returns it.
    pub(crate) fn new() -> DescriptionList {
        DescriptionList {
            source_route: None,
            load_balance: None,
            failover: None,
            descriptions: Vec::<Description>::new(),
        }
    }

    /// Creates a new description list from a full descriptor node and returns
    /// it.
    pub(crate) fn new_from_node(
        node: &Node,
    ) -> Result<DescriptionList, Error> {
        let mut description_list = DescriptionList::new();
        node.process_child_nodes(|n| description_list.process_nodes(n))?;
        Ok(description_list)
    }

    /// Returns the maximum SDU defined for all descriptions.
    pub(crate) fn sdu(&self) -> usize {
        let mut sdu: usize = 0;
        for description in &self.descriptions {
            sdu = sdu.max(description.sdu() as usize);
        }
        sdu
    }
}

pub(crate) struct AddressOption {
    address: Address,
    sock_addr: SocketAddr,
}

impl AddressOption {
    /// Attemps to establish a connection to the database using the given
    /// socket address.
    fn connect(
        &self,
        client: &mut Client,
        description_option: &DescriptionOption,
    ) -> Result<(), Error> {
        client.connect_phase_one(
            self.sock_addr,
            &description_option.connect_data,
            &self.address,
            &description_option.description,
        )
    }

    /// Creates a new address option for the specified socket address and
    /// returns it.
    fn new(address: Address, sock_addr: SocketAddr) -> AddressOption {
        AddressOption { address, sock_addr }
    }
}

pub(crate) struct DescriptionOption {
    description: Description,
    connect_data: String,
    address_options: Vec<AddressOption>,
}

impl DescriptionOption {
    /// Creates a new description option for the description. Each of the host
    /// names in the addresses associated with the description are resolved and
    /// stored as separate address options.
    fn new(
        description: Description,
        config: &Config,
    ) -> Result<DescriptionOption, Error> {
        let connect_data = description.build_connect_data(config);
        let mut address_options = Vec::<AddressOption>::new();
        let address_lists = calc_active_children(
            &description.address_lists,
            description.source_route(),
            description.failover(),
            description.load_balance(),
        );
        for address_list in address_lists {
            let addresses = calc_active_children(
                &address_list.addresses,
                address_list.source_route(),
                address_list.failover(),
                address_list.load_balance(),
            );
            for address in addresses {
                let addrs_iter =
                    (address.host(), address.port()).to_socket_addrs()?;
                for addr in addrs_iter {
                    address_options
                        .push(AddressOption::new(address.clone(), addr));
                }
            }
        }
        Ok(DescriptionOption {
            description,
            connect_data,
            address_options,
        })
    }

    /// Attempts to establish a connection to the database using the
    /// information found in the option.
    pub(crate) fn connect(&self, client: &mut Client) -> Result<(), Error> {
        let num_attempts = self.description.retry_count() + 1;
        let retry_delay = self.description.retry_delay();
        let mut result = Err(Error::unexpected_result());
        for _ in 0..num_attempts {
            for option in self.address_options.iter() {
                result = option.connect(client, self);
                if result.is_ok() {
                    return result;
                }
            }
            thread::sleep(retry_delay);
        }
        result
    }
}

/// Calculates the list of active children from a set of children.
fn calc_active_children<T>(
    children: &[T],
    source_route: bool,
    failover: bool,
    load_balance: bool,
) -> Vec<T>
where
    T: Clone,
{
    // if only one child is present, that child is considered active
    if children.len() == 1 {
        children.to_vec()

    // for source route, only the first child is considered active
    } else if source_route {
        children[..1].to_vec()

    // for failover with load balance, all of the children are active but
    // are processed in a random order
    } else if failover && load_balance {
        let mut active_children = children.to_vec();
        active_children.shuffle(&mut rand::rng());
        active_children

    // for failover without load balance, all of the children are active and
    // are processed in the same order
    } else if failover {
        children.to_vec()

    // without failover, load balance indicates that only one of the children
    // is considered active and which one is selected randomly
    } else if load_balance {
        vec![children.choose(&mut rand::rng()).unwrap().clone()]

    // without failover or load balance, just the first child is active
    } else {
        children[..1].to_vec()
    }
}
