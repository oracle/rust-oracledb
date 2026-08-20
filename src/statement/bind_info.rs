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
// bind_info.rs
//
// Defines the structure representing bind information.
//-----------------------------------------------------------------------------

use crate::constants;
use crate::db_type::DbType;
use crate::error::Error;
use crate::metadata::Metadata;

/// Defines the structure representing bind information.
#[derive(Clone)]
pub(crate) struct BindInfo {
    pub(crate) name: String,
    pub(crate) is_return_bind: bool,
    pub(crate) metadata: Option<Metadata>,
    pub(crate) bind_direction: u8,
}

impl BindInfo {
    /// Checks and sets the metadata for the bind. The replace flag simply
    /// causes the new metadata to be used without checking. If the new
    /// parameter value is incompatible with the previous metadata set for the
    /// bind, an error is returned.
    pub(crate) fn check_and_set_metadata(
        &mut self,
        replace: bool,
        desired_type: &'static DbType,
        desired_max_size: usize,
        binds_changed: &mut bool,
    ) -> Result<(), Error> {
        let mut matches: bool = false;
        if let Some(metadata) = self.metadata.as_ref() {
            if metadata.db_type() == desired_type {
                matches = metadata.max_size() as usize >= desired_max_size;
            } else if !replace {
                return Err(Error::different_types(
                    metadata.db_type(),
                    desired_type,
                ));
            }
        }

        // set metadata for the bind variable if no match has taken place
        if !matches {
            *binds_changed = true;
            let mut metadata =
                Metadata::new_scalar(desired_type, desired_max_size);
            if self.is_return_bind {
                metadata.set_is_array(true);
            }
            self.metadata = Some(metadata);
        }
        Ok(())
    }

    /// Returns whether or not the bind is an input bind.
    pub(crate) fn is_input_bind(&self) -> bool {
        !self.is_return_bind
            && self.bind_direction & constants::TTC_BIND_DIR_INPUT != 0
    }

    /// Returns whether or not the bind is an output bind.
    pub(crate) fn is_output_bind(&self) -> bool {
        self.is_return_bind
            || self.bind_direction & constants::TTC_BIND_DIR_OUTPUT != 0
    }
}
