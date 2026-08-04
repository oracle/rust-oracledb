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
// end_user_security_context.rs
//
// Defines the structures used for creating End User Security Context payloads
// used by Oracle Deep Data Security.
//-----------------------------------------------------------------------------

//! End user security contexts are validated once, serialized to OSON, and then
//! attached to a connection when Deep Data Security is enabled.

use std::collections::HashMap;

use crate::error::Error;
use crate::json::{JsonValue, encode_oson};
use crate::secret_value::SecretValue;

// end user security context constants
const MAX_END_USER_SECURITY_CONTEXT_LENGTH: usize = 65535;

/// End-user identity for a Deep Data Security context.
#[derive(Clone)]
pub enum EndUserIdentity {
    /// Token issued by an external identity provider.
    Token(String),

    /// Database-managed end user.
    DatabaseUser { name: String, key: Option<String> },
}

impl EndUserIdentity {
    /// Validates the identity fields required for Deep Data Security.
    fn validate(&self) -> Result<(), Error> {
        match self {
            EndUserIdentity::Token(token) => {
                if token.is_empty() {
                    return Err(Error::invalid_end_user_security_context(
                        "end user token must be non-empty",
                    ));
                }
            }
            EndUserIdentity::DatabaseUser { name, key } => {
                if name.is_empty() {
                    return Err(Error::invalid_end_user_security_context(
                        "database user name must be non-empty",
                    ));
                }
                if let Some(key) = key
                    && key.is_empty()
                {
                    return Err(Error::invalid_end_user_security_context(
                        "database user key must be non-empty if specified",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Validated, pre-serialized Deep Data Security payload.
#[derive(Clone)]
pub struct EndUserSecurityContext {
    oson_bytes: SecretValue,
}

/// Builder used to add optional data roles and attributes before encoding.
pub struct EndUserSecurityContextBuilder {
    identity: EndUserIdentity,
    database_access_token: String,
    data_roles: Option<Vec<String>>,
    attributes: Option<HashMap<String, JsonValue>>,
}

impl EndUserSecurityContext {
    /// Returns the raw OSON payload for the protocol layer.
    pub(crate) fn oson_bytes(&self) -> Vec<u8> {
        self.oson_bytes.get_value()
    }

    /// Creates a builder for a Deep Data Security context with optional roles
    /// and attributes.
    pub fn builder(
        identity: EndUserIdentity,
        database_access_token: String,
    ) -> EndUserSecurityContextBuilder {
        EndUserSecurityContextBuilder {
            identity,
            database_access_token,
            data_roles: None,
            attributes: None,
        }
    }

    /// Creates a Deep Data Security context without optional data roles or
    /// attributes.
    pub fn new(
        identity: EndUserIdentity,
        database_access_token: String,
    ) -> Result<Self, Error> {
        Self::builder(identity, database_access_token).build()
    }
}

impl EndUserSecurityContextBuilder {
    /// Sets the attribute-value pairs for the Deep Data Security context.
    pub fn attributes(
        mut self,
        attributes: HashMap<String, JsonValue>,
    ) -> Self {
        self.attributes = Some(attributes);
        self
    }

    /// Validates the builder fields required before encoding.
    fn validate(&self) -> Result<(), Error> {
        if self.database_access_token.is_empty() {
            return Err(Error::invalid_end_user_security_context(
                "database_access_token must be non-empty",
            ));
        }
        self.identity.validate()
    }

    /// Validates inputs and encodes the Deep Data Security context to OSON.
    pub fn build(self) -> Result<EndUserSecurityContext, Error> {
        self.validate()?;

        let mut payload = HashMap::new();
        payload
            .insert("ver".to_string(), JsonValue::String("1.0".to_string()));

        match self.identity {
            EndUserIdentity::Token(token) => {
                payload.insert(
                    "end_user_token".to_string(),
                    JsonValue::String(token),
                );
            }
            EndUserIdentity::DatabaseUser { name, key } => {
                payload.insert(
                    "end_user_name".to_string(),
                    JsonValue::String(name),
                );
                if let Some(key) = key {
                    payload.insert(
                        "end_user_contextid".to_string(),
                        JsonValue::String(key),
                    );
                }
            }
        }

        payload.insert(
            "database_access_token".to_string(),
            JsonValue::String(self.database_access_token),
        );

        if let Some(data_roles) = self.data_roles {
            payload.insert(
                "data_roles".to_string(),
                JsonValue::JsonArray(
                    data_roles.into_iter().map(JsonValue::String).collect(),
                ),
            );
        }

        if let Some(attributes) = self.attributes {
            let attributes = attributes
                .into_iter()
                .map(|(name, values)| {
                    let mut attribute = HashMap::new();
                    attribute
                        .insert("name".to_string(), JsonValue::String(name));
                    attribute.insert("values".to_string(), values);
                    JsonValue::JsonObject(attribute)
                })
                .collect::<Vec<JsonValue>>();

            payload.insert(
                "attributes".to_string(),
                JsonValue::JsonArray(attributes),
            );
        }

        let oson_bytes = encode_oson(&JsonValue::JsonObject(payload));
        if oson_bytes.len() > MAX_END_USER_SECURITY_CONTEXT_LENGTH {
            return Err(Error::invalid_end_user_security_context_length(
                oson_bytes.len(),
            ));
        }

        Ok(EndUserSecurityContext {
            oson_bytes: SecretValue::new(&oson_bytes),
        })
    }

    /// Sets the data roles for the Deep Data Security context.
    pub fn data_roles(mut self, roles: Vec<String>) -> Self {
        self.data_roles = Some(roles);
        self
    }
}
