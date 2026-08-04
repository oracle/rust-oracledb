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
// connect_string_parser.rs
//
// Defines the structure used for parsing a connect string.
//-----------------------------------------------------------------------------

use super::connect_options::Address;
use super::connect_options::AddressList;
use super::connect_options::Description;
use super::connect_options::DescriptionList;
use super::tnsnames_file_parser::lookup_tns_alias;

use crate::constants;
use crate::error::Error;
use crate::parser::Parser;

// define the array of special characters that are allowed in a network name
const NETWORK_NAME_SPECIAL_CHARS: [char; 21] = [
    '"', '\'', '<', '>', '/', '\\', ',', '.', ':', ';', '-', '_', '$', '+',
    '*', '#', '&', '!', '%', '?', '@',
];

/// Returns whether a character belongs to a valid host name.
fn is_host_name_char(ch: char) -> bool {
    ch.is_alphabetic()
        || ch.is_ascii_digit()
        || ch == '-'
        || ch == '_'
        || ch == '.'
}

/// Returns whether a character belongs to a valid network keyword.
fn is_network_keyword_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'
}

/// Parses a descriptor key from the connect string which is subsequently
/// converted to lowercase.
fn parse_descriptor_key(parser: &mut Parser) -> Result<String, Error> {
    if let Some(token) = parser.parse_token(is_network_keyword_char) {
        Ok(token.to_lowercase())
    } else {
        Err(parser.parse_error())
    }
}

/// Parses a descriptor key/value pair from the connect string.
fn parse_descriptor_key_value_pair(
    parser: &mut Parser,
) -> Result<Node, Error> {
    parser.skip_whitespace();
    let key = parse_descriptor_key(parser)?;
    parser.skip_whitespace();
    if !parser.next_char_matches('=') {
        return Err(parser.parse_error());
    }
    parser.skip_whitespace();
    let value = parse_descriptor_value(parser)?;
    Ok(Node { key, value })
}

/// Parses a descriptor value from the connect string. This could be a
/// simple value or could contain multiple values and be considered a
/// container.
fn parse_descriptor_value(parser: &mut Parser) -> Result<NodeValue, Error> {
    let value = if let Some(value) = parser.parse_delimited_text('"', '"') {
        parser.skip_whitespace();
        NodeValue::Simple(value)
    } else if !parser.peek_char_matches('(') {
        let token = parser.parse_token(|ch| ch != ')');
        let value = token.unwrap_or_default().trim().to_string();
        NodeValue::Simple(value)
    } else {
        let mut child_nodes = Vec::<Node>::new();
        while !parser.peek_char_matches(')') && parser.next_char_matches('(') {
            let node = parse_descriptor_key_value_pair(parser)?;
            child_nodes.push(node);
            parser.skip_whitespace();
        }
        NodeValue::Container(child_nodes)
    };
    if parser.next_char_matches(')') {
        Ok(value)
    } else {
        Err(parser.parse_error())
    }
}

/// Parses a host name from the easy connect string. This consists of any
/// number of alphanumeric characters (or dashes, underscores or periods),
/// optionally enclosed by brackets (indicating IPv6 name resolution is
/// desired).
fn parse_easy_connect_host(parser: &mut Parser) -> Option<String> {
    let mut result = parser.parse_delimited_text('[', ']');
    if result.is_none() {
        result = parser.parse_token(is_host_name_char);
    }
    result
}

/// Parses the list of hosts from an easy connect string. This should be a
/// series of host names (optionally followed by a port number), separated
/// by commas or semicolons.
fn parse_easy_connect_hosts(
    parser: &mut Parser,
    protocol: Option<String>,
) -> Option<Description> {
    let mut port_index = 0;
    let mut description = Description::new();
    let mut address_list = AddressList::new();
    loop {
        let host = parse_easy_connect_host(parser);
        if host.is_none() {
            break;
        }
        address_list
            .addresses
            .push(Address::new(host, protocol.clone()));
        if let Some(ch) = parser.peek_char()
            && ch == ':'
        {
            parser.next_char();
            let port = parse_easy_connect_port(parser);
            if port.is_none() {
                break;
            }
            for address in &mut address_list.addresses[port_index..] {
                address.set_port(port);
            }
            port_index = address_list.addresses.len();
        }
        if let Some(ch) = parser.peek_char() {
            if ch == ';' {
                description.address_lists.push(address_list);
                address_list = AddressList::new();
                port_index = 0;
            } else if ch != ',' {
                description.address_lists.push(address_list);
                return Some(description);
            }
        }
    }
    None
}

/// Parses the instance name from an easy connect string. This is expected
/// to be a slash followd by a series of alphanumeric characters, dashes,
/// underscores or periods.
fn parse_easy_connect_instance_name(parser: &mut Parser) -> Option<String> {
    parser.parse_token_with_sep(is_network_keyword_char, '/')
}

/// Parses the port number from the easy connect string. This consists of
/// one or more digits that can be represented as a 32-bit integer.
fn parse_easy_connect_port(parser: &mut Parser) -> Option<u16> {
    let mut num_chars = 0;
    while let Some(ch) = parser.peek_char() {
        if !ch.is_ascii_digit() {
            break;
        }
        parser.next_char();
        num_chars += 1;
    }
    if num_chars > 0 {
        let text = &parser.save_pos()[1..];
        text.parse::<u16>().ok()
    } else {
        parser.restore_pos();
        None
    }
}

/// Parses the protocol from an easy connect string. This should be a
/// series of alphabetic characters, dashes or underscores, followed by a
/// colon and two slashes. If such a string is found, it is returned;
/// otherwise, None is returned.
///
/// A degenerate protocol consisting only of two slashes is also permitted
/// for backwards compatibility and is simply discarded.
fn parse_easy_connect_protocol(parser: &mut Parser) -> Option<String> {
    let mut num_sep_chars = 0;
    let mut num_protocol_chars = 0;
    let mut found_colon = false;
    while let Some(ch) = parser.next_char() {
        if ch == ':' && !found_colon && num_protocol_chars > 0 {
            found_colon = true;
        } else if ch == '/' && (found_colon || num_protocol_chars == 0) {
            num_sep_chars += 1;
            if num_sep_chars == 2 {
                let text = &parser.save_pos()[..num_protocol_chars];
                if text.is_empty() {
                    return None;
                } else {
                    return Some(text.into());
                }
            }
        } else if !found_colon
            && num_sep_chars == 0
            && (ch.is_alphabetic() || ch == '-' || ch == '_')
        {
            num_protocol_chars += 1;
        } else {
            break;
        }
    }
    parser.restore_pos();
    None
}

/// Parses the server type from an easy connect string. This is expected
/// to be a colon followd by a series of alphabetic characters.
fn parse_easy_connect_server_type(parser: &mut Parser) -> Option<String> {
    parser.parse_token_with_sep(|ch| ch.is_ascii_alphabetic(), ':')
}

/// Parses the service name from an easy connect string. This is expected
/// to be a slash followd by a series of alphanumeric characters, dashes,
/// underscores or periods.
fn parse_easy_connect_service_name(parser: &mut Parser) -> Option<String> {
    parser.parse_token_with_sep(is_network_keyword_char, '/')
}

/// Attempts to parse an easy connect string from the connect string. This
/// is allowed to return None instead of an error, which will cause the
/// connect string to be treated as a tnsnames.ora file entry instead.
fn parse_easy_connect_string(parser: &mut Parser) -> Option<DescriptionList> {
    let protocol = parse_easy_connect_protocol(parser);
    match parse_easy_connect_hosts(parser, protocol) {
        Some(mut description) => {
            let service_name = parse_easy_connect_service_name(parser)?;
            description.set_service_name(Some(service_name));
            description
                .set_server_type(parse_easy_connect_server_type(parser));
            description
                .set_instance_name(parse_easy_connect_instance_name(parser));
            let mut description_list = DescriptionList::new();
            description_list.descriptions.push(description);
            Some(description_list)
        }
        None => None,
    }
}

/// Parses a full descriptor from the connect string. At this point it is
/// known that the connect string starts with an opening parenthesis so any
/// errors in parsing are immediately returned.
fn parse_full_descriptor(
    parser: &mut Parser,
) -> Result<DescriptionList, Error> {
    parser.next_char();
    let top_node = parse_descriptor_key_value_pair(parser)?;
    if top_node.key == "description_list" {
        let description_list = DescriptionList::new_from_node(&top_node)?;
        Ok(description_list)
    } else if top_node.key == "description" {
        let description = Description::new_from_node(&top_node)?;
        let mut description_list = DescriptionList::new();
        description_list.descriptions.push(description);
        Ok(description_list)
    } else {
        Err(Error::invalid_connect_string(parser.source().to_string()))
    }
}

/// Returns whether a character belongs to a valid network name.
pub(crate) fn is_network_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || NETWORK_NAME_SPECIAL_CHARS.contains(&ch)
}

/// Parses a connect string and returns a description list if the connect
/// string is a full descriptor or a valid easy connect string.
pub(crate) fn parse_connect_string(
    connect_string: &str,
) -> Result<Option<DescriptionList>, Error> {
    let mut parser = Parser::new(connect_string.trim());
    if parser.peek_char_matches('(') {
        let description_list = parse_full_descriptor(&mut parser)?;
        Ok(Some(description_list))
    } else {
        Ok(parse_easy_connect_string(&mut parser))
    }
}

/// Parses a connect string and returns the description list that defines how
/// the connection will be established, or returns an error if the connect
/// string is invalid in some way.
pub(crate) fn parse_connect_string_or_lookup_alias(
    connect_string: &str,
    config_dir_opt: &Option<String>,
) -> Result<DescriptionList, Error> {
    match parse_connect_string(connect_string)? {
        Some(description_list) => Ok(description_list),
        None => match config_dir_opt {
            Some(config_dir) => lookup_tns_alias(config_dir, connect_string),
            None => Err(Error::no_config_dir()),
        },
    }
}

/// Returns a string that has sanitized the input so that it fits the
/// requirements for network names.
pub(crate) fn sanitize_network_name(value: &str) -> String {
    let trimmed_value = value.trim_matches(['"', '\'']);
    let mut output: String = trimmed_value
        .chars()
        .map(|c| if is_network_name_char(c) { c } else { '?' })
        .collect();
    if let Some(ch) = output.chars().last()
        && ch == '\\'
    {
        output.pop();
        output.push('?');
    }
    output
}

/// Validates that a string will satisfy the requirements of a network name.
pub(crate) fn validate_network_name(value: &str) -> Result<(), Error> {
    let sanitized = sanitize_network_name(value);
    if sanitized == value {
        Ok(())
    } else {
        Err(Error::invalid_network_name(value.into()))
    }
}

enum NodeValue {
    Simple(String),
    Container(Vec<Node>),
}

pub(crate) struct Node {
    key: String,
    value: NodeValue,
}

impl Node {
    /// Returns the node as a simple value, or an error if the node refers to
    /// a container.
    fn as_simple_value(&self, expected_type: &str) -> Result<&str, Error> {
        match &self.value {
            NodeValue::Simple(value) => Ok(value),
            NodeValue::Container(_) => Err(self.invalid_value(expected_type)),
        }
    }

    /// Returns whether the node has a non-empty value
    fn has_value(&self) -> bool {
        match &self.value {
            NodeValue::Simple(value) => !value.is_empty(),
            NodeValue::Container(value) => !value.is_empty(),
        }
    }

    /// Returns an error indicating that the value for the node is invalid.
    fn invalid_value(&self, expected_type: &str) -> Error {
        Error::invalid_descriptor_node(self.key.clone(), expected_type.into())
    }

    /// Returns the node as a boolean value, or an error if the node refers to
    /// a container.
    pub(crate) fn as_bool(&self) -> Result<bool, Error> {
        let text_value = self.as_simple_value("bool")?;
        Ok(matches!(
            text_value.trim().to_lowercase().as_str(),
            "on" | "yes" | "true"
        ))
    }

    /// Returns the node as a pool purity value, or an error if the node refers
    /// to a container or contains an invalid purity value.
    pub(crate) fn as_purity(&self) -> Result<u8, Error> {
        let text_value = self.as_simple_value("purity")?;
        match text_value.to_lowercase().as_str() {
            "new" => Ok(constants::PURITY_NEW),
            "self" => Ok(constants::PURITY_SELF),
            _ => Err(self.invalid_value("purity")),
        }
    }

    /// Returns the node as a server type, or an error if the node refers to a
    /// container or contains an invalid server type.
    pub(crate) fn as_server_type(&self) -> Result<String, Error> {
        let value = self.as_simple_value("server_type")?;
        match value.to_lowercase().as_str() {
            "dedicated" | "pooled" | "shared" => Ok(value.to_string()),
            _ => Err(self.invalid_value("server_type")),
        }
    }

    /// Returns the node as a string, or an error if the node refers to a
    /// container.
    pub(crate) fn as_str(&self) -> Result<String, Error> {
        let value = self.as_simple_value("String")?;
        Ok(value.to_string())
    }

    /// Returns the node as a u16, or an error if the node refers to a
    /// container or does not contain a valid u16.
    pub(crate) fn as_u16(&self) -> Result<u16, Error> {
        let text_value = self.as_simple_value("u16")?;
        match text_value.parse::<u16>() {
            Ok(value) => Ok(value),
            Err(_) => Err(self.invalid_value("u16")),
        }
    }

    /// Returns the node as a u32, or an error if the node refers to a
    /// container or does not contain a valid u32.
    pub(crate) fn as_u32(&self) -> Result<u32, Error> {
        let text_value = self.as_simple_value("u32")?;
        match text_value.parse::<u32>() {
            Ok(value) => Ok(value),
            Err(_) => Err(self.invalid_value("u32")),
        }
    }

    /// Returns the node as a u64, or an error if the node refers to a
    /// container or does not contain a valid u64.
    pub(crate) fn as_u64(&self) -> Result<u64, Error> {
        let text_value = self.as_simple_value("u64")?;
        match text_value.parse::<u64>() {
            Ok(value) => Ok(value),
            Err(_) => Err(self.invalid_value("u64")),
        }
    }

    /// Returns the key for the node
    pub(crate) fn key(&self) -> &str {
        self.key.as_str()
    }

    /// Processes a container using the specified function. Only nodes that
    /// contain values are processed.
    pub(crate) fn process_child_nodes(
        &self,
        mut f: impl FnMut(&Node) -> Result<(), Error>,
    ) -> Result<(), Error> {
        match &self.value {
            NodeValue::Simple(_) => {}
            NodeValue::Container(child_nodes) => {
                for child_node in child_nodes {
                    if child_node.has_value() {
                        f(child_node)?;
                    }
                }
            }
        }
        Ok(())
    }
}
