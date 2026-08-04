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
// parser.rs
//
// Defines the structure representing a parser of a string.
//-----------------------------------------------------------------------------

use crate::error::Error;

pub(crate) struct Parser<'a> {
    source: &'a str,
    pos: usize,
    temp_pos: usize,
}

impl Parser<'_> {
    /// Returns true if there are no more characters remaining in the string.
    pub(crate) fn eof(&self) -> bool {
        self.peek_char().is_none()
    }

    /// Creates a new parser for the given string and returns it.
    pub(crate) fn new(source: &str) -> Parser<'_> {
        Parser {
            source,
            pos: 0,
            temp_pos: 0,
        }
    }

    /// Consumes the next character in the string and returns it.
    pub(crate) fn next_char(&mut self) -> Option<char> {
        match self.source[self.temp_pos..].chars().next() {
            None => None,
            Some(ch) => {
                self.temp_pos += ch.len_utf8();
                Some(ch)
            }
        }
    }

    /// Consumes the next character in the string and returns whether it
    /// matches the specified character.
    pub(crate) fn next_char_matches(&mut self, match_ch: char) -> bool {
        if let Some(ch) = self.next_char() {
            ch == match_ch
        } else {
            false
        }
    }

    /// Parses delimited text and returns the text between the delimiters (or
    /// None if no such delimited text is found).
    pub(crate) fn parse_delimited_text(
        &mut self,
        start_sep: char,
        end_sep: char,
    ) -> Option<String> {
        if self.peek_char_matches(start_sep) {
            self.temp_pos += start_sep.len_utf8();
            while let Some(ch) = self.next_char() {
                if ch == end_sep {
                    let text = self.save_pos();
                    return Some(text[1..text.len() - 1].into());
                }
            }
        }
        self.restore_pos();
        None
    }

    /// Returns a parse error showing the line and position in that line that
    /// caused the issue.
    pub(crate) fn parse_error(&self) -> Error {
        let start_line_pos = self.source[..self.temp_pos]
            .rfind('\n')
            .map_or(0, |i| i + 1);
        let end_line_pos = self.source[self.temp_pos..]
            .find('\n')
            .map_or(self.source.len(), |i| self.temp_pos + i);
        let text = self.source[start_line_pos..end_line_pos].to_string();
        Error::parse_error(text, self.temp_pos - start_line_pos + 1)
    }

    /// Parses a simple SQL name from the text. A simple SQL name may be quoted
    /// (and not contain any embedded quotes) or it may be an alphabetic
    /// character followed by any number of alphanumeric characters or one of
    /// the characters '_', '$' and '#'.
    pub(crate) fn parse_simple_sql_name(&mut self) -> Option<String> {
        if let Some(quoted_value) = self.parse_delimited_text('"', '"') {
            if quoted_value.is_empty() || quoted_value.contains('"') {
                self.restore_pos();
                None
            } else {
                Some(quoted_value)
            }
        } else {
            let first_ch = self.next_char().unwrap_or(' ');
            if !first_ch.is_alphabetic() {
                self.restore_pos();
                return None;
            }
            while let Some(ch) = self.peek_char() {
                if !ch.is_alphanumeric() && ch != '_' && ch != '$' && ch != '#'
                {
                    break;
                }
                self.next_char();
            }
            let text = self.save_pos();
            Some(text.into())
        }
    }

    /// Parses a token from the text that contains one or more characters
    /// for which the specified function returns true. This function allows for
    /// any number of characters to be scanned before this function is called.
    pub(crate) fn parse_token(
        &mut self,
        f: fn(char) -> bool,
    ) -> Option<String> {
        let mut num_chars = 0;
        while let Some(ch) = self.peek_char() {
            if !f(ch) {
                break;
            }
            self.temp_pos += ch.len_utf8();
            num_chars += 1;
        }
        if num_chars == 0 {
            self.restore_pos();
            None
        } else {
            let text = self.save_pos();
            Some(text[text.len() - num_chars..].into())
        }
    }

    /// Like parse_token() but requires the separator first.
    pub(crate) fn parse_token_with_sep(
        &mut self,
        f: fn(char) -> bool,
        sep: char,
    ) -> Option<String> {
        if let Some(ch) = self.next_char()
            && ch == sep
        {
            return self.parse_token(f);
        }
        self.restore_pos();
        None
    }

    /// Returns the current character without advancing the text position.
    pub(crate) fn peek_char(&self) -> Option<char> {
        self.source[self.temp_pos..].chars().next()
    }

    /// Returns true if the current character being considered matches.
    pub(crate) fn peek_char_matches(&self, match_ch: char) -> bool {
        if let Some(ch) = self.peek_char() {
            ch == match_ch
        } else {
            false
        }
    }

    pub(crate) fn restore_pos(&mut self) {
        self.temp_pos = self.pos;
    }

    pub(crate) fn source(&self) -> &str {
        self.source
    }

    pub(crate) fn save_pos(&mut self) -> &str {
        let result = &self.source[self.pos..self.temp_pos];
        self.pos = self.temp_pos;
        result
    }

    /// Skips all characters until the next end-of-line character and saves the
    /// position.
    pub(crate) fn skip_to_end_of_line(&mut self) {
        while let Some(ch) = self.next_char() {
            if ch == '\n' {
                break;
            }
        }
        self.pos = self.temp_pos;
    }

    /// Skips all whitespace and saves the position.
    pub(crate) fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if !ch.is_whitespace() {
                break;
            }
            self.temp_pos += ch.len_utf8();
        }
        self.pos = self.temp_pos;
    }
}
