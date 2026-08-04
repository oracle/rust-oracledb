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
// sql_parser.rs
//
// Defines the structure representing the SQL parser that determines the bind
// variables found within statements.
//-----------------------------------------------------------------------------

use super::CachedStatement;
use crate::error::Error;
use crate::parser::Parser;

/// Represents the SQL parser that determines the bind variables found within
/// statements.
pub(super) struct SqlParser<'a> {
    parser: Parser<'a>,
    returning_keyword_found: bool,
}

impl SqlParser<'_> {
    pub fn new(source: &str) -> SqlParser<'_> {
        SqlParser {
            parser: Parser::new(source),
            returning_keyword_found: false,
        }
    }

    pub fn parse(
        &mut self,
        statement: &mut CachedStatement,
    ) -> Result<(), Error> {
        let mut initial_keyword_found = false;
        let mut last_was_string = false;
        let mut last_was_alpha = false;
        let mut last_ch = ' ';
        while let Some(ch) = self.parser.peek_char() {
            // look for certain keywords (initial keyword and the ones for
            // detecting DML returning statements
            let is_alpha = ch.is_ascii_alphabetic();
            if is_alpha && !last_was_alpha {
                self.parser.save_pos();
            } else if !is_alpha && last_was_alpha {
                let keyword = self.parser.save_pos();
                if !initial_keyword_found {
                    initial_keyword_found = true;
                    statement.determine_statement_type(keyword);
                } else if statement.is_dml && !self.returning_keyword_found {
                    if keyword.to_uppercase() == "RETURNING" {
                        self.returning_keyword_found = true;
                    }
                } else if self.returning_keyword_found
                    && keyword.to_uppercase() == "INTO"
                {
                    statement.is_returning = true;
                }
            }

            // need to keep track of whether the last token parsed was a string
            // (excluding whitespace) as if the last token parsed was a string
            // a following colon is not a bind variable but a part of the JSON
            // constant syntax
            if ch == '\'' {
                last_was_string = true;
                if last_ch.eq_ignore_ascii_case(&'q') {
                    self.parse_qstring()?;
                } else {
                    self.parse_quoted_string()?;
                }
            } else if !ch.is_whitespace() {
                if ch == '-' {
                    self.parse_single_line_comment()?;
                } else if ch == '/' {
                    self.parse_multiple_line_comment()?;
                } else if ch == '"' {
                    self.parse_quoted_string()?;
                } else if ch == ':' && !last_was_string {
                    self.parse_bind_name(statement)?;
                }
                last_was_string = false;
            }

            // advance to next character and track previous character
            last_was_alpha = is_alpha;
            self.parser.next_char();
            last_ch = ch;
        }
        Ok(())
    }

    /// Bind variables are identified as follows:
    /// - Quoted and non-quoted bind names are allowed.
    /// - Quoted bind names can contain any characters.
    /// - Non-quoted bind names must begin with an alphabetic character.
    /// - Non-quoted bind names can only contain alphanumeric characters, the
    ///   underscore, the dollar sign and the pound sign.
    /// - Non-quoted bind names cannot be Oracle Database Reserved Names (this
    ///   is left to the server to detct and return an appropriate error).
    fn parse_bind_name(
        &mut self,
        statement: &mut CachedStatement,
    ) -> Result<(), Error> {
        let mut quoted_name = false;
        let mut in_bind = false;
        let mut digits_only = false;
        self.parser.save_pos();
        self.parser.next_char();
        while let Some(ch) = self.parser.peek_char() {
            if !in_bind {
                if ch.is_whitespace() {
                    self.parser.next_char();
                    continue;
                } else if ch == '"' {
                    quoted_name = true;
                } else if ch.is_ascii_digit() {
                    digits_only = true;
                } else if !ch.is_alphabetic() {
                    self.parser.restore_pos();
                    break;
                }
                in_bind = true;
            } else if (digits_only && !ch.is_ascii_digit())
                || (quoted_name && ch == '"')
                || (!digits_only
                    && !quoted_name
                    && !ch.is_alphanumeric()
                    && ch != '_'
                    && ch != '$'
                    && ch != '#')
            {
                break;
            }
            self.parser.next_char();
        }
        if in_bind {
            let bind_name = if quoted_name {
                self.parser.save_pos()[1..].trim_start()[1..].to_string()
            } else if digits_only {
                self.parser.save_pos()[1..].trim_start().to_string()
            } else {
                self.parser.save_pos()[1..].trim_start().to_uppercase()
            };
            statement.add_bind(bind_name);
        }
        Ok(())
    }

    fn parse_multiple_line_comment(&mut self) -> Result<(), Error> {
        todo!();
    }

    fn parse_qstring(&mut self) -> Result<(), Error> {
        todo!();
    }

    fn parse_quoted_string(&mut self) -> Result<(), Error> {
        self.parser.save_pos();
        let quote_char = self.parser.next_char().unwrap();
        while let Some(ch) = self.parser.next_char() {
            if ch == quote_char {
                self.parser.save_pos();
                return Ok(());
            }
        }
        self.parser.restore_pos();
        Ok(())
    }

    fn parse_single_line_comment(&mut self) -> Result<(), Error> {
        let mut in_comment = false;
        self.parser.save_pos();
        self.parser.next_char();
        while let Some(ch) = self.parser.peek_char() {
            if !in_comment {
                if ch != '-' {
                    self.parser.restore_pos();
                    return Ok(());
                }
                in_comment = true;
            } else if ch == '\n' {
                break;
            }
            self.parser.next_char();
        }
        self.parser.save_pos();
        Ok(())
    }
}
