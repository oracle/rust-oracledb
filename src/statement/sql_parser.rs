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

    /// Parses a multiple line comment. At this point it is known that the two
    /// leading characters (/*) denoting a multiple line comment have been
    /// detected. All characters up to the terminating characters (*/) are
    /// discarded.
    fn parse_multiple_line_comment(&mut self) -> Result<(), Error> {
        let mut last_ch = ' ';
        self.parser.next_char();
        while let Some(ch) = self.parser.peek_char() {
            if ch == '/' && last_ch == '*' {
                self.parser.save_pos();
                return Ok(());
            }
            self.parser.next_char();
            last_ch = ch;
        }
        Err(self.parser.parse_error())
    }

    /// Parses a q-string which consists of the characters "q" and a single
    /// quote followed by a start separator, any text that does not contain the
    /// end separator and the end separator and ending quote. The following are
    /// examples that demonstrate this:
    ///     - q'[...]'
    ///     - q'{...}'
    ///     - q'<...>'
    ///     - q'(...)'
    ///     ' q?...?' (where ? is any character)
    /// At this point it is known that a q-string has been started
    fn parse_qstring(&mut self) -> Result<(), Error> {
        self.parser.next_char();
        if let Some(ch) = self.parser.next_char() {
            let end_sep = match ch {
                '[' => ']',
                '{' => '}',
                '<' => '>',
                '(' => ')',
                ch => ch,
            };
            while let Some(ch) = self.parser.next_char() {
                if ch == end_sep
                    && let Some(next_ch) = self.parser.peek_char()
                    && next_ch == '\''
                {
                    self.parser.save_pos();
                    return Ok(());
                }
            }
        }
        Err(self.parser.parse_error())
    }

    /// Parses a quoted string. All characters up to the next quote character
    /// are discarded.
    fn parse_quoted_string(&mut self) -> Result<(), Error> {
        self.parser.save_pos();
        let quote_char = self.parser.next_char().unwrap();
        while let Some(ch) = self.parser.peek_char() {
            if ch == quote_char {
                self.parser.save_pos();
                return Ok(());
            }
            self.parser.next_char();
        }
        Err(self.parser.parse_error())
    }

    /// Parses a single line comment. At this point it is known that the two
    /// leading dashes have been found and all remaining characters up to the
    /// end of the current line are discarded.
    fn parse_single_line_comment(&mut self) {
        self.parser.skip_to_end_of_line();
    }

    /// Creates a new SQL parser for the given SQL.
    pub(crate) fn new(source: &str) -> SqlParser<'_> {
        SqlParser {
            parser: Parser::new(source),
            returning_keyword_found: false,
        }
    }

    /// Parses the statement and stores the resuls of the parse into the given
    /// cached statement.
    pub(crate) fn parse(
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
                if ch == '-' && last_ch == '-' {
                    self.parse_single_line_comment();
                } else if ch == '*' && last_ch == '/' {
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
}
