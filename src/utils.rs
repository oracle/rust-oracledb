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
// utils.rs
//
// Defines a number of utility functions.
//-----------------------------------------------------------------------------

use crate::error::Error;
use crate::parser::Parser;

/// Encodes a Rust string as big-endian UTF-16 bytes.
pub(crate) fn string_to_utf16be_bytes(value: &str) -> Vec<u8> {
    let mut bytes = Vec::<u8>::new();
    for code_unit in value.encode_utf16() {
        bytes.extend_from_slice(&code_unit.to_be_bytes());
    }
    bytes
}

/// Encodes a Rust string as little-endian UTF-16 bytes.
pub(crate) fn string_to_utf16le_bytes(value: &str) -> Vec<u8> {
    let mut bytes = Vec::<u8>::new();
    for code_unit in value.encode_utf16() {
        bytes.extend_from_slice(&code_unit.to_le_bytes());
    }
    bytes
}

/// Decodes big-endian UTF-16 bytes into a Rust string.
pub(crate) fn utf16be_bytes_to_string(bytes: &[u8]) -> String {
    let num_code_units = bytes.len() / 2;
    let mut code_units = Vec::<u16>::with_capacity(num_code_units);
    for i in 0..num_code_units {
        let offset = i * 2;
        let code_unit =
            u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap());
        code_units.push(code_unit);
    }
    String::from_utf16(&code_units).unwrap()
}

/// Decodes little-endian UTF-16 bytes into a Rust string.
pub(crate) fn utf16le_bytes_to_string(bytes: &[u8]) -> String {
    let num_code_units = bytes.len() / 2;
    let mut code_units = Vec::<u16>::with_capacity(num_code_units);
    for i in 0..num_code_units {
        let offset = i * 2;
        let code_unit =
            u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        code_units.push(code_unit);
    }
    String::from_utf16(&code_units).unwrap()
}

/// Returns the supplied string as a new string that is safe to include as a
/// literal in a SQL statement.
pub fn enquote_literal(value: &str) -> String {
    let mut result = String::new();
    result.push('\'');
    for ch in value.chars() {
        result.push(ch);
        if ch == '\'' {
            result.push(ch);
        }
    }
    result.push('\'');
    result
}

/// Returns the supplied string as a new string that is safe to include as a
/// quoted name in a SQL statement. Since embedded quotes are not allowed in
/// quoted identifiers, an error is returned if embedded quotes are detected.
/// If the capitalize parameter is true, the string is converted to uppercase.
pub fn enquote_name(value: &str, capitalize: bool) -> Result<String, Error> {
    if value.contains('"') {
        Err(Error::name_has_embedded_quotes())
    } else {
        let adjusted_value = if capitalize {
            value.to_uppercase()
        } else {
            value.to_string()
        };
        Ok(format!("\"{}\"", adjusted_value))
    }
}

/// Returns a boolean indicating if the supplied string contains a valid
/// qualified SQL name. Leading and trailing spaces are ignored. The name must
/// be one or more simple SQL names separated by periods (and any amount of
/// whitespace), optionally followed by the '@' symbol and an additional simple
/// SQL name referring to a database link name.
pub fn is_qualified_sql_name(value: &str) -> bool {
    let mut num_parts = 0;
    let mut parser = Parser::new(value.trim());
    while parser.parse_simple_sql_name().is_some() {
        num_parts += 1;
        parser.skip_whitespace();
        let ch_opt = parser.next_char();
        parser.skip_whitespace();
        match ch_opt {
            Some(ch) => {
                if ch == '@' {
                    return parser.parse_simple_sql_name().is_some()
                        && parser.next_char().is_none();
                } else if ch != '.' {
                    return false;
                }
            }
            None => break,
        }
    }
    num_parts > 0 && parser.next_char().is_none()
}

/// Returns a boolean indicating if the supplied string contains a valid simple
/// SQL name. Leading and trailing spaces are ignored. If the value is not
/// quoted, the first character must be alphabetic and the remaining characters
/// must be alphanumeric or contain the characters '_', '$' or '#'. A quoted
/// name may not contain embedded quotes and no characters other than
/// whitespace are allowed outside the quotes.
pub fn is_simple_sql_name(value: &str) -> bool {
    let mut parser = Parser::new(value.trim());
    parser.parse_simple_sql_name().is_some() && parser.next_char().is_none()
}
