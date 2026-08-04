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
// options.rs
//
// Defines the structure representing statement options.
//-----------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct StatementOptions {
    prefetch_rows: u32,
    fetch_array_size: u32,
    fetch_lobs: bool,
}

impl StatementOptions {
    pub(crate) fn new() -> StatementOptions {
        StatementOptions {
            prefetch_rows: 2,
            fetch_array_size: 100,
            fetch_lobs: false,
        }
    }

    pub(crate) fn fetch_array_size(&self) -> u32 {
        self.fetch_array_size
    }

    pub(crate) fn fetch_lobs(&self) -> bool {
        self.fetch_lobs
    }

    pub(crate) fn prefetch_rows(&self) -> u32 {
        self.prefetch_rows
    }

    pub(crate) fn set_fetch_array_size(&mut self, value: u32) {
        self.fetch_array_size = value;
    }

    pub(crate) fn set_fetch_lobs(&mut self) {
        self.fetch_lobs = true;
    }

    pub(crate) fn set_prefetch_rows(&mut self, value: u32) {
        self.prefetch_rows = value;
    }
}
