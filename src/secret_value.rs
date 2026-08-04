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
// secret_value.rs
//
// Defines the SecretValue struct used for storing values that are sensitive
// and should not be visible in memory dumps.
//-----------------------------------------------------------------------------

use rand::RngExt;

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct SecretValue {
    obfuscated_value: Vec<u8>,
    obfuscator: Vec<u8>,
}

impl SecretValue {
    /// Returns the original secret value.
    pub(crate) fn get_value(&self) -> Vec<u8> {
        xor_bytes(&self.obfuscated_value, &self.obfuscator)
    }

    /// Creates a new secret value from the specified value.
    pub(crate) fn new(value: &[u8]) -> Self {
        let mut obfuscator = vec![0u8; value.len()];
        rand::rng().fill(&mut obfuscator[..]);
        Self {
            obfuscated_value: xor_bytes(value, &obfuscator),
            obfuscator,
        }
    }
}

/// Utility function that performs an XOR of the input value with an
/// obfuscator. Performing once with the secret value creates an obfuscated
/// value. Performing a second time with the obfuscated value returns the
/// original secret value.
fn xor_bytes(value: &[u8], obfuscator: &[u8]) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::with_capacity(value.len());
    for (value_byte, obfuscator_byte) in value.iter().zip(obfuscator.iter()) {
        result.push(value_byte ^ obfuscator_byte);
    }
    result
}
