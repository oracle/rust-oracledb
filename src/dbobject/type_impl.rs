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
// type_impl.rs
//
// Implementation details for database object types.
//-----------------------------------------------------------------------------

use std::fmt;
use std::sync::OnceLock;

use crate::error::Error;
use crate::metadata::Metadata;

/// Identifies a database object type by its qualified name.
#[derive(Clone, Debug)]
pub(crate) struct DbObjectTypeIdentity {
    pub(crate) name: String,
    pub(crate) package_name: Option<String>,
    pub(crate) schema: String,
}

impl DbObjectTypeIdentity {
    /// Reports whether the identity refers to the built-in XML type.
    pub(crate) fn is_xmltype(&self) -> bool {
        self.schema == "SYS"
            && self.package_name.is_none()
            && self.name == "XMLTYPE"
    }

    /// Returns a quoted name suitable for database type discovery.
    pub(crate) fn lookup_name(&self) -> String {
        let (name, suffix) = match self.name.strip_suffix("%ROWTYPE") {
            Some(name) => (name, "%ROWTYPE"),
            None => (self.name.as_str(), ""),
        };
        let schema = self.schema.replace('"', "\"\"");
        let name = name.replace('"', "\"\"");
        match &self.package_name {
            Some(package) => {
                let package = package.replace('"', "\"\"");
                format!("\"{schema}\".\"{package}\".\"{name}\"{suffix}")
            }
            None => format!("\"{schema}\".\"{name}\"{suffix}"),
        }
    }

    /// Returns the fully qualified database object type name.
    pub(super) fn fqn(&self) -> String {
        match &self.package_name {
            Some(package) => {
                format!("{}.{package}.{}", self.schema, self.name)
            }
            None => format!("{}.{}", self.schema, self.name),
        }
    }
}

/// Describes a named object or collection type.
///
/// Discovery completes the metadata before returning a shared type handle.
/// Metadata retains shared handles to nested types after the connection
/// closes.
pub struct DbObjectType {
    body: OnceLock<DbObjectTypeBody>,
    identity: DbObjectTypeIdentity,
}

/// Metadata populated once discovery has decoded a type.
pub(super) struct DbObjectTypeBody {
    pub(super) attrs: Vec<Metadata>,
    pub(super) element: Option<Metadata>,
    pub(super) oid: Vec<u8>,
    pub(super) version: u16,
    pub(super) collection_type: Option<u8>,
}

impl fmt::Debug for DbObjectType {
    /// Formats the identity without traversing nested metadata.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DbObjectType").field("fqn", &self.fqn()).finish()
    }
}

impl DbObjectType {
    /// Returns the body completed before the public handle is returned.
    fn body(&self) -> &DbObjectTypeBody {
        self.body.get().expect("get_type completes all reachable metadata")
    }

    /// Reports whether discovery has populated this type's own metadata.
    pub(crate) fn is_fully_populated(&self) -> bool {
        self.body.get().is_some()
    }

    /// Returns the collection kind supplied by the type descriptor.
    pub(super) fn collection_type(&self) -> Option<u8> {
        self.body().collection_type
    }

    /// Returns the quoted name used to discover this type.
    pub(super) fn lookup_name(&self) -> String {
        self.identity.lookup_name()
    }

    /// Creates an identity whose body will be populated during discovery.
    pub(super) fn new_internal(identity: DbObjectTypeIdentity) -> Self {
        Self { identity, body: OnceLock::new() }
    }

    /// Returns the type OID used in metadata descriptors.
    pub(crate) fn oid(&self) -> &[u8] {
        &self.body().oid
    }

    /// Publishes the type's attribute and collection element metadata once.
    pub(super) fn set_body(
        &self,
        body: DbObjectTypeBody,
    ) -> Result<(), Error> {
        self.body.set(body).map_err(|_| Error::unexpected_result())
    }

    /// Returns the type version used in metadata descriptors.
    pub(crate) fn version(&self) -> u16 {
        self.body().version
    }

    /// Returns the attributes in declaration order.
    pub fn attributes(&self) -> &[Metadata] {
        &self.body().attrs
    }

    /// Returns collection element metadata, or None for a record type.
    pub fn element_metadata(&self) -> Option<&Metadata> {
        self.body().element.as_ref()
    }

    /// Returns the fully qualified database object type name.
    pub fn fqn(&self) -> String {
        self.identity.fqn()
    }

    /// Reports whether the resolved type is a collection.
    pub fn is_collection(&self) -> bool {
        self.body().element.is_some()
    }

    /// Returns the database type name.
    pub fn name(&self) -> &str {
        &self.identity.name
    }

    /// Returns the owning package name, when applicable.
    pub fn package_name(&self) -> Option<&str> {
        self.identity.package_name.as_deref()
    }

    /// Returns the owning schema name.
    pub fn schema(&self) -> &str {
        &self.identity.schema
    }
}

impl PartialEq for DbObjectType {
    /// Compares the identity of cached type instances.
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
