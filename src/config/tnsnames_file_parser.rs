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
// tnsnames_file_parser.rs
//
// Defines the structure used for parsing a tnsnames.ora file.
//-----------------------------------------------------------------------------

use super::connect_options::DescriptionList;
use super::connect_string_parser::parse_connect_string;

use crate::error::Error;
use crate::parser::Parser;

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::SystemTime;

static TNSNAMES_FILE_CACHE: LazyLock<Mutex<HashMap<String, TnsnamesFile>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Ensures that a file with the specified name is in the cache.
fn ensure_file_in_cache(
    cache: &mut HashMap<String, TnsnamesFile>,
    config_dir: &str,
    file_name: &String,
    files_in_progress: &mut Vec<String>,
) -> Result<(), Error> {
    if files_in_progress.contains(file_name) {
        let including_file_name = files_in_progress.pop().unwrap();
        return Err(Error::ifile_cycle_detected(
            including_file_name,
            file_name.into(),
        ));
    }
    if let Some(cached_file) = cache.get(file_name)
        && cached_file.is_current()?
    {
        return Ok(());
    }
    files_in_progress.push(file_name.into());
    load_file_in_cache(cache, config_dir, file_name, files_in_progress)?;
    files_in_progress.pop();
    Ok(())
}

/// Loads the file with the specified name into the cache.
fn load_file_in_cache(
    cache: &mut HashMap<String, TnsnamesFile>,
    config_dir: &str,
    file_name: &String,
    files_in_progress: &mut Vec<String>,
) -> Result<(), Error> {
    let mut file = TnsnamesFile::new(file_name)?;
    let contents = fs::read_to_string(file_name)?;
    let mut parser = Parser::new(&contents);
    loop {
        let keys = parse_keys(&mut parser);
        if keys.is_empty() {
            break;
        }
        let value = parse_value(&mut parser);
        if value.is_empty() {
            break;
        }
        if keys.len() == 1 && keys.first().unwrap() == "ifile" {
            let config_dir_path = Path::new(config_dir);
            let included_path = config_dir_path.join(&value);
            let included_file_name =
                included_path.to_str().unwrap().to_owned();
            ensure_file_in_cache(
                cache,
                config_dir,
                &included_file_name,
                files_in_progress,
            )?;
            let included_file = cache.get(&included_file_name).unwrap();
            for (key, value) in &included_file.entries {
                file.entries.insert(key.to_string(), value.to_string());
            }
        } else {
            for key in keys {
                file.entries.insert(key, value.clone());
            }
        }
    }
    cache.insert(file_name.into(), file);
    Ok(())
}

/// Parses keys from the file contents and returns them. This consists of
/// any non-whitespace characters separated by commas and followed by an
/// equals sign.
fn parse_keys(parser: &mut Parser) -> Vec<String> {
    let mut keys = Vec::<String>::new();
    skip_whitespace_and_comments(parser);
    while let Some(ch) = parser.next_char() {
        if ch.is_whitespace() || ch == '#' || ch == ',' || ch == '=' {
            let key = parser
                .save_pos()
                .trim_end_matches(ch)
                .to_lowercase()
                .to_string();
            let mut next_ch = ch;
            if ch.is_whitespace() || ch == '#' {
                let passed_line_boundary =
                    skip_whitespace_and_comments(parser);
                next_ch = parser.peek_char().unwrap_or(ch);
                if next_ch == ',' || next_ch == '=' {
                    parser.next_char();
                } else {
                    if !passed_line_boundary {
                        parser.skip_to_end_of_line();
                    }
                    continue;
                }
            }
            skip_whitespace_and_comments(parser);
            if key.is_empty() {
                keys.clear();
            } else {
                keys.push(key);
                if next_ch == '=' {
                    break;
                }
            }
        }
    }
    keys
}

/// Parses the value from the file contents and returns it. This consists of
/// at least a single line of text, with continuation permitted if the contents
/// contain parentheses (full connect descriptor).
fn parse_value(parser: &mut Parser) -> String {
    let mut parts = Vec::<String>::new();
    let mut num_parens = 0;
    let full_descriptor = parser.peek_char_matches('(');
    while !parser.eof() {
        while let Some(ch) = parser.next_char() {
            if ch == '\n' || ch == '#' {
                let part = parser.save_pos().trim_end_matches(ch);
                if !part.is_empty() {
                    parts.push(part.to_string());
                }
                if ch == '#' {
                    parser.skip_to_end_of_line();
                }
                break;
            } else if ch == '(' {
                num_parens += 1;
            } else if ch == ')' && num_parens > 0 {
                num_parens -= 1;
            }
        }
        if (num_parens == 0 || !full_descriptor) && !parts.is_empty() {
            break;
        }
    }
    parts.join("\n")
}

/// Skips all whitespace and comments. Returns a boolean indicating if a
/// line boundary was passed.
fn skip_whitespace_and_comments(parser: &mut Parser) -> bool {
    let mut passed_line_boundary = false;
    while let Some(ch) = parser.peek_char() {
        if ch == '\n' {
            passed_line_boundary = true;
        } else if ch == '#' {
            parser.skip_to_end_of_line();
            passed_line_boundary = true;
        } else if !ch.is_whitespace() {
            break;
        }
        parser.next_char();
    }
    parser.save_pos();
    passed_line_boundary
}

/// Looks up the TNS alias in the tnsnames.ora file found in the given
/// configuration directory and returns it (or an error if the file cannot be
/// read or the alias is not found in the file).
pub(crate) fn lookup_tns_alias(
    config_dir: &str,
    alias: &str,
) -> Result<DescriptionList, Error> {
    let file_name_buf = Path::new(config_dir).join("tnsnames.ora");
    let file_name = file_name_buf.to_str().unwrap().to_owned();
    let mut cache = TNSNAMES_FILE_CACHE.lock().unwrap();
    let mut files_in_progress = Vec::<String>::new();
    ensure_file_in_cache(
        &mut cache,
        config_dir,
        &file_name,
        &mut files_in_progress,
    )?;
    let file = cache.get(&file_name).unwrap();
    file.lookup_alias(alias)
}

struct TnsnamesFile {
    file_name: String,
    entries: HashMap<String, String>,
    included_files: Vec<TnsnamesFile>,
    mtime: SystemTime,
}

impl TnsnamesFile {
    /// Returns whether or not the contents of the file are current or not.
    /// This is assumed to be true if the mtime of the file matches and all of
    /// the included files are also current.
    fn is_current(&self) -> Result<bool, Error> {
        let metadata = fs::metadata(&self.file_name)?;
        let mtime = metadata.modified()?;
        if mtime != self.mtime {
            return Ok(false);
        }
        for included_file in self.included_files.iter() {
            if !included_file.is_current()? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Lookup an alias in the file and return an error if the alias cannot be
    /// found in the file or the alias refers to an invalid connect string.
    fn lookup_alias(&self, alias: &str) -> Result<DescriptionList, Error> {
        let lookup_value = alias.to_lowercase();
        match self.entries.get(&lookup_value) {
            Some(connect_string) => {
                match parse_connect_string(connect_string)? {
                    Some(description_list) => Ok(description_list),
                    None => Err(Error::invalid_connect_string(
                        connect_string.clone(),
                    )),
                }
            }
            None => Err(Error::tns_alias_not_found(
                self.file_name.clone(),
                alias.to_string(),
            )),
        }
    }

    /// Examines the cache to see if the particular file is present and has not
    /// been changed. If missing or updated, the file is read and stored in the
    /// cache.
    fn new(file_name: &str) -> Result<TnsnamesFile, Error> {
        let metadata = fs::metadata(file_name)?;
        let mtime = metadata.modified()?;
        Ok(TnsnamesFile {
            file_name: file_name.into(),
            entries: HashMap::new(),
            included_files: Vec::new(),
            mtime,
        })
    }
}
