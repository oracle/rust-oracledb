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
// cache.rs
//
// Caches database object types and parses their type descriptor (TDS).
//-----------------------------------------------------------------------------

use std::collections::HashMap;
use std::sync::Arc;

use super::DbObjectType;
use super::type_impl::{DbObjectTypeBody, DbObjectTypeIdentity};
use crate::bind_params::BindParameters;
use crate::client::{Client, ClientRef};
use crate::cursor::Cursor;
use crate::db_type::DbType;
use crate::db_value::ToDbValue;
use crate::error::Error;
use crate::exec_result::ExecResult;
use crate::metadata::Metadata;
use crate::read_buffer::ReadBuffer;
use crate::response::Response;
use crate::row::Row;
use crate::statement::StatementOptions;

const SQL_GET_COLUMNS: &str = r#"
    select
        column_name as "name",
        data_type as "type_name",
        data_type_owner as "type_owner",
        case
            when data_type in
                    ('CHAR', 'NCHAR', 'VARCHAR2', 'NVARCHAR2', 'RAW')
                then data_length
            else 0
        end as "max_size",
        case
            when data_precision is null and data_scale is null
                then 0
            when data_precision is null and data_type = 'NUMBER'
                then 38
            when data_precision is null and data_type != 'NUMBER'
                then 0
            else data_precision
        end as "precision",
        case
            when data_precision is null and data_scale is null
                    and data_type = 'NUMBER'
                then -127
            when data_scale is null
                then 0
            else data_scale
        end as "scale"
    from all_tab_cols
    where owner = :owner
      and table_name = substr(:name, 1, length(:name) - 8)
      and hidden_column != 'YES'
    order by column_id"#;

const SQL_GET_ELEMENT_OBJECT_TYPE_NO_PACKAGE: &str = r#"
    select
        elem_type_owner as "schema",
        null as "package_name",
        elem_type_name as "name"
    from all_coll_types
    where owner = :owner and type_name = :name"#;

const SQL_GET_ELEMENT_OBJECT_TYPE_WITH_PACKAGE: &str = r#"
    select
        elem_type_owner as "schema",
        elem_type_package as "package_name",
        elem_type_name as "name"
    from all_plsql_coll_types
    where owner = :owner
      and package_name = :package_name
      and type_name = :name"#;

const SQL_GET_ELEMENT_TYPE_NO_PACKAGE: &str = r#"
    select elem_type_name
    from all_coll_types
    where owner = :owner and type_name = :name"#;

const SQL_GET_ELEMENT_TYPE_WITH_PACKAGE: &str = r#"
    select elem_type_name
    from all_plsql_coll_types
    where owner = :owner
      and package_name = :package_name
      and type_name = :name"#;

const SQL_GET_METADATA_FOR_NAME: &str = r#"
    declare
        t_Instantiable              varchar2(3);
        t_SuperTypeOwner            varchar2(128);
        t_SuperTypeName             varchar2(128);
        t_SubTypeRefCursor          sys_refcursor;
        t_Pos                       pls_integer;
    begin
        :ret_val := dbms_pickler.get_type_shape(:full_name, :oid,
            :version, :tds, t_Instantiable, t_SuperTypeOwner,
            t_SuperTypeName, :attrs_rc, t_SubTypeRefCursor);
        :package_name := null;
        if substr(:full_name, length(:full_name) - 7) = '%ROWTYPE' then
            t_Pos := instr(:full_name, '.');
            :schema := substr(:full_name, 1, t_Pos - 1);
            :name := substr(:full_name, t_Pos + 1);
        else
            begin
                select owner, type_name
                into :schema, :name
                from all_types
                where type_oid = :oid;
            exception
            when no_data_found then
                begin
                    select owner, package_name, type_name
                    into :schema, :package_name, :name
                    from all_plsql_types
                    where type_oid = :oid;
                exception
                when no_data_found then
                    null;
                end;
            end;
        end if;
    end;"#;

const TTC_OBJ_TDS_TYPE_BINARY_DOUBLE: u8 = 45;
const TTC_OBJ_TDS_TYPE_BINARY_FLOAT: u8 = 37;
const TTC_OBJ_TDS_TYPE_BLOB: u8 = 30;
const TTC_OBJ_TDS_TYPE_BOOLEAN: u8 = 8;
const TTC_OBJ_TDS_TYPE_CHAR: u8 = 1;
const TTC_OBJ_TDS_TYPE_CLOB: u8 = 29;
const TTC_OBJ_TDS_TYPE_COLLECTION: u8 = 28;
const TTC_OBJ_TDS_TYPE_DATE: u8 = 2;
const TTC_OBJ_TDS_TYPE_EMBED_ADT_INFO: u8 = 44;
const TTC_OBJ_TDS_TYPE_END_EMBED_ADT: u8 = 40;
const TTC_OBJ_TDS_TYPE_FLOAT: u8 = 5;
const TTC_OBJ_TDS_TYPE_INTERVAL: u8 = 24;
const TTC_OBJ_TDS_TYPE_NUMBER: u8 = 6;
const TTC_OBJ_TDS_TYPE_OBJECT: u8 = 27;
const TTC_OBJ_TDS_TYPE_RAW: u8 = 19;
const TTC_OBJ_TDS_TYPE_START_EMBED_ADT: u8 = 39;
const TTC_OBJ_TDS_TYPE_SUBTYPE_MARKER: u8 = 43;
const TTC_OBJ_TDS_TYPE_TIMESTAMP: u8 = 21;
const TTC_OBJ_TDS_TYPE_TIMESTAMP_LTZ: u8 = 33;
const TTC_OBJ_TDS_TYPE_TIMESTAMP_TZ: u8 = 23;
const TTC_OBJ_TDS_TYPE_VARCHAR: u8 = 7;

struct DbObjectTypeLookup {
    attrs: Vec<Metadata>,
    is_row_type: bool,
    oid: Vec<u8>,
    tds: Vec<u8>,
    version: u16,
    identity: DbObjectTypeIdentity,
}

struct DbObjectParsedTds {
    attrs: Vec<DbObjectParsedTdsAttr>,
    element: Option<DbObjectParsedTdsAttr>,
    collection_type: Option<u8>,
}

/// Datatype details read from a TDS field description.
struct DbObjectParsedTdsAttr {
    db_type: &'static DbType,
    max_size: u32,
    precision: u8,
    scale: i8,
}

/// Returns the driver datatype corresponding to a dictionary type name.
fn db_type_from_name(name: &str) -> Result<&'static DbType, Error> {
    let db_type = match name {
        "BFILE" => crate::DB_TYPE_BFILE,
        "BINARY_DOUBLE" => crate::DB_TYPE_BINARY_DOUBLE,
        "BINARY_FLOAT" => crate::DB_TYPE_BINARY_FLOAT,
        "BINARY_INTEGER" | "PL/SQL BINARY INTEGER" | "PL/SQL PLS INTEGER" => {
            crate::DB_TYPE_BINARY_INTEGER
        }
        "BLOB" => crate::DB_TYPE_BLOB,
        "BOOLEAN" | "PL/SQL BOOLEAN" => crate::DB_TYPE_BOOLEAN,
        "CHAR" => crate::DB_TYPE_CHAR,
        "CLOB" => crate::DB_TYPE_CLOB,
        "DATE" => crate::DB_TYPE_DATE,
        "INTERVAL DAY TO SECOND" => crate::DB_TYPE_INTERVAL_DS,
        "INTERVAL YEAR TO MONTH" => crate::DB_TYPE_INTERVAL_YM,
        "NCHAR" => crate::DB_TYPE_NCHAR,
        "NCLOB" => crate::DB_TYPE_NCLOB,
        "NUMBER" => crate::DB_TYPE_NUMBER,
        "NVARCHAR2" => crate::DB_TYPE_NVARCHAR,
        "RAW" => crate::DB_TYPE_RAW,
        "ROWID" => crate::DB_TYPE_ROWID,
        "TIMESTAMP" => crate::DB_TYPE_TIMESTAMP,
        "TIMESTAMP WITH LOCAL TIME ZONE" | "TIMESTAMP WITH LOCAL TZ" => {
            crate::DB_TYPE_TIMESTAMP_LTZ
        }
        "TIMESTAMP WITH TIME ZONE" | "TIMESTAMP WITH TZ" => {
            crate::DB_TYPE_TIMESTAMP_TZ
        }
        "UROWID" => crate::DB_TYPE_UROWID,
        "VARCHAR2" => crate::DB_TYPE_VARCHAR,
        _ => {
            return Err(Error::unsupported_object_metadata_type(name));
        }
    };
    Ok(db_type)
}

/// Executes a named query and materializes all of its rows.
fn execute_named_rows(
    client: &mut Client,
    client_ref: &ClientRef,
    sql: &str,
    named_params: &[(&str, &dyn ToDbValue)],
) -> Result<Vec<Row>, Error> {
    let mut statement =
        client.get_statement(sql, true, &StatementOptions::new())?;
    let result = (|| {
        let checked_params = statement.check_named_binds(named_params)?;
        let binding = [checked_params.as_slice()];
        let params: BindParameters = binding.as_slice().into();
        statement.check_binds(&params)?;
        let response =
            client.execute(&mut statement, client_ref, params, false)?;
        client.fetch_all_rows(client_ref, &mut statement, response)
    })();
    client.return_statement(&statement);
    result
}

/// Looks up the collection element's nested object identity.
fn lookup_collection_element_object(
    client: &mut Client,
    client_ref: &ClientRef,
    typ: &DbObjectType,
) -> Result<DbObjectTypeIdentity, Error> {
    let owner = typ.schema();
    let name = typ.name();
    let package_name = typ.package_name();
    let rows = if let Some(package_name) = package_name {
        execute_named_rows(
            client,
            client_ref,
            SQL_GET_ELEMENT_OBJECT_TYPE_WITH_PACKAGE,
            &[
                ("owner", &owner as &dyn ToDbValue),
                ("package_name", &package_name as &dyn ToDbValue),
                ("name", &name as &dyn ToDbValue),
            ],
        )?
    } else {
        execute_named_rows(
            client,
            client_ref,
            SQL_GET_ELEMENT_OBJECT_TYPE_NO_PACKAGE,
            &[
                ("owner", &owner as &dyn ToDbValue),
                ("name", &name as &dyn ToDbValue),
            ],
        )?
    };
    let row = rows.into_iter().next().ok_or_else(Error::no_data_found)?;
    Ok(DbObjectTypeIdentity {
        schema: row.get("schema")?,
        package_name: row.get("package_name")?,
        name: row.get("name")?,
    })
}

/// Returns the datatype of a collection's character LOB element.
fn lookup_collection_element_type(
    client: &mut Client,
    client_ref: &ClientRef,
    typ: &DbObjectType,
) -> Result<&'static DbType, Error> {
    let owner = typ.schema();
    let name = typ.name();
    let package_name = typ.package_name();
    let sql = if package_name.is_some() {
        SQL_GET_ELEMENT_TYPE_WITH_PACKAGE
    } else {
        SQL_GET_ELEMENT_TYPE_NO_PACKAGE
    };
    let mut named_params: Vec<(&str, &dyn ToDbValue)> =
        vec![("owner", &owner), ("name", &name)];
    if let Some(package) = package_name.as_ref() {
        named_params.push(("package_name", package));
    }
    let name: String =
        execute_named_rows(client, client_ref, sql, &named_params)?
            .into_iter()
            .next()
            .ok_or_else(Error::no_data_found)?
            .get(0)?;
    db_type_from_name(&name)
}

/// Looks up table columns used by a `%ROWTYPE` descriptor.
fn lookup_row_type_attrs(
    client: &mut Client,
    client_ref: &ClientRef,
    identity: &DbObjectTypeIdentity,
) -> Result<Vec<Metadata>, Error> {
    let rows = execute_named_rows(
        client,
        client_ref,
        SQL_GET_COLUMNS,
        &[
            ("owner", &identity.schema as &dyn ToDbValue),
            ("name", &identity.name as &dyn ToDbValue),
        ],
    )?;
    let mut attrs = Vec::new();
    for mut row in rows {
        let attr_name: String = row.take("name")?;
        let mut type_name: String = row.take("type_name")?;
        let type_owner: Option<String> = row.take("type_owner")?;
        let max_size: u32 = row.get("max_size")?;
        let precision: u8 = row.get("precision")?;
        let scale: i8 = row.get("scale")?;
        let metadata = if let Some(schema) = type_owner {
            let object_identity = DbObjectTypeIdentity {
                schema,
                package_name: None,
                name: type_name,
            };
            let mut metadata = Metadata::new_object_field(
                attr_name,
                crate::DB_TYPE_OBJECT,
                0,
                0,
                0,
            );
            metadata.set_object_type_identity(object_identity);
            metadata
        } else {
            if let Some(start_pos) = type_name.find('(')
                && let Some(end_pos) = type_name.find(')')
                && end_pos > start_pos
            {
                type_name.replace_range(start_pos..=end_pos, "");
            }
            metadata_from_name(
                attr_name, &type_name, max_size, precision, scale,
            )?
        };
        attrs.push(metadata);
    }
    Ok(attrs)
}

/// Executes the metadata lookup and materializes its REF CURSOR rows.
fn lookup_type(
    client: &mut Client,
    client_ref: &ClientRef,
    full_name: &str,
) -> Result<DbObjectTypeLookup, Error> {
    let mut statement = client.get_statement(
        SQL_GET_METADATA_FOR_NAME,
        true,
        &StatementOptions::new(),
    )?;
    let mut response = Response::new();
    let mut cursor_statement = None;
    let result = (|| {
        let named_params = [
            ("ret_val", &crate::DB_TYPE_BINARY_INTEGER as &dyn ToDbValue),
            ("full_name", &full_name as &dyn ToDbValue),
            ("oid", &crate::DB_TYPE_RAW as &dyn ToDbValue),
            ("version", &crate::DB_TYPE_BINARY_INTEGER as &dyn ToDbValue),
            ("tds", &crate::DB_TYPE_RAW as &dyn ToDbValue),
            ("attrs_rc", &crate::DB_TYPE_CURSOR as &dyn ToDbValue),
            ("package_name", &crate::DB_TYPE_VARCHAR as &dyn ToDbValue),
            ("schema", &crate::DB_TYPE_VARCHAR as &dyn ToDbValue),
            ("name", &crate::DB_TYPE_VARCHAR as &dyn ToDbValue),
        ];
        let checked_params = statement.check_named_binds(&named_params)?;
        let binding = [checked_params.as_slice()];
        let params: BindParameters = binding.as_slice().into();
        statement.check_binds(&params)?;
        response =
            client.execute(&mut statement, client_ref, params, false)?;
        let mut values =
            ExecResult::new(&statement, &mut response).out_bind_data();
        cursor_statement = values
            .take::<Option<Cursor>>("attrs_rc")?
            .map(Cursor::into_cached_statement);
        let return_value: i32 = values.take("ret_val")?;
        let oid: Option<Vec<u8>> = values.take("oid")?;
        let version: Option<u16> = values.take("version")?;
        let tds: Option<Vec<u8>> = values.take("tds")?;
        let package_name: Option<String> = values.take("package_name")?;
        let schema: Option<String> = values.take("schema")?;
        let name: Option<String> = values.take("name")?;
        if return_value != 0 || name.is_none() || schema.is_none() {
            return Err(Error::invalid_db_object_type_name(full_name));
        }

        let identity = DbObjectTypeIdentity {
            name: name.unwrap(),
            package_name,
            schema: schema.unwrap(),
        };
        let is_row_type = full_name.ends_with("%ROWTYPE");
        let attrs = if is_row_type {
            lookup_row_type_attrs(client, client_ref, &identity)?
        } else {
            let cursor_statement = cursor_statement
                .as_mut()
                .ok_or_else(Error::unexpected_result)?;
            let rows = client.fetch_all_rows(
                client_ref,
                cursor_statement,
                Response::new(),
            )?;
            read_object_attrs(rows)?
        };
        Ok(DbObjectTypeLookup {
            attrs,
            is_row_type,
            oid: oid.unwrap_or_default(),
            tds: tds.unwrap_or_default(),
            version: version.unwrap_or_default(),
            identity,
        })
    })();
    response.cleanup_pending_values(client);
    if let Some(cursor_statement) = cursor_statement {
        client.return_statement(&cursor_statement);
    }
    client.return_statement(&statement);
    result
}

/// Combines dictionary datatype details with sizes supplied by the TDS.
fn merge_attribute_metadata(
    dictionary: &mut Metadata,
    tds: &DbObjectParsedTdsAttr,
) {
    let (precision, scale) = if tds.precision != 0 || tds.scale != 0 {
        (tds.precision, tds.scale)
    } else {
        (dictionary.precision(), dictionary.scale())
    };
    dictionary.set_object_field_details(
        dictionary.db_type(),
        tds.max_size,
        precision,
        scale,
    );
}

/// Creates scalar metadata from dictionary names, preserving numeric aliases.
fn metadata_from_name(
    name: String,
    type_name: &str,
    max_size: u32,
    precision: u8,
    scale: i8,
) -> Result<Metadata, Error> {
    let (db_type, precision, scale) = match type_name {
        "INTEGER" | "SMALLINT" => (crate::DB_TYPE_NUMBER, 38, 0),
        "REAL" => (crate::DB_TYPE_NUMBER, 63, -127),
        "DOUBLE PRECISION" | "FLOAT" => (
            crate::DB_TYPE_NUMBER,
            if precision == 0 { 126 } else { precision },
            -127,
        ),
        value if value.starts_with("INTERVAL DAY TO SECOND") => {
            (crate::DB_TYPE_INTERVAL_DS, precision, scale)
        }
        _ => (db_type_from_name(type_name)?, precision, scale),
    };
    Ok(Metadata::new_object_field(name, db_type, max_size, precision, scale))
}

/// Parses collection and scalar field details from a TDS value.
fn parse_tds(
    bytes: &[u8],
    num_expected_attrs: usize,
) -> Result<DbObjectParsedTds, Error> {
    let mut input = ReadBuffer::from_bytes(bytes);
    let _end_offset = input.read_bytes(4)?;
    let _version_opcode_and_version = input.read_bytes(2)?;
    let _unknown = input.read_bytes(2)?;
    let num_attrs = input.read_u16be()? as usize;
    let _tds_attributes = input.read_bytes(1)?;
    let _start_adt_opcode = input.read_bytes(1)?;
    let _adt_number = input.read_bytes(2)?;
    let _index_table_offset = input.read_bytes(4)?;

    let pos = input.get_pos();
    let is_collection =
        num_attrs == 1 && input.read_u8()? == TTC_OBJ_TDS_TYPE_COLLECTION;
    if is_collection {
        let element_pos = input.read_u32be()? as usize;
        let _max_num_elements = input.read_u32be()?;
        let collection_type = input.read_u8()?;
        input.set_pos(element_pos);
        let element = parse_tds_attr(&mut input)?
            .ok_or_else(Error::unexpected_result)?;
        Ok(DbObjectParsedTds {
            attrs: Vec::new(),
            element: Some(element),
            collection_type: Some(collection_type),
        })
    } else {
        input.set_pos(pos);
        let mut attrs = Vec::with_capacity(num_expected_attrs);
        for _ in 0..num_expected_attrs {
            attrs.push(
                parse_tds_attr(&mut input)?
                    .ok_or_else(Error::unexpected_result)?,
            );
        }
        Ok(DbObjectParsedTds { attrs, element: None, collection_type: None })
    }
}

/// Parses one field description from a TDS buffer.
fn parse_tds_attr(
    input: &mut ReadBuffer,
) -> Result<Option<DbObjectParsedTdsAttr>, Error> {
    let attr_type = loop {
        let attr_type = input.read_u8()?;
        if attr_type == TTC_OBJ_TDS_TYPE_EMBED_ADT_INFO {
            input.read_bytes(1)?;
        } else if attr_type != TTC_OBJ_TDS_TYPE_SUBTYPE_MARKER {
            break attr_type;
        }
    };

    let mut csfrm = 0;
    let mut max_size = 0;
    let mut precision = 0;
    let mut scale = 0;
    let ora_type_num = match attr_type {
        TTC_OBJ_TDS_TYPE_NUMBER => {
            precision = input.read_i8()? as u8;
            scale = input.read_i8()?;
            crate::constants::ORA_TYPE_NUM_NUMBER
        }
        TTC_OBJ_TDS_TYPE_FLOAT => {
            input.read_bytes(1)?;
            crate::constants::ORA_TYPE_NUM_NUMBER
        }
        TTC_OBJ_TDS_TYPE_VARCHAR | TTC_OBJ_TDS_TYPE_CHAR => {
            max_size = input.read_u16be()?.into();
            csfrm = input.read_u8()? & 0x7f;
            input.read_bytes(2)?;
            if attr_type == TTC_OBJ_TDS_TYPE_VARCHAR {
                crate::constants::ORA_TYPE_NUM_VARCHAR
            } else {
                crate::constants::ORA_TYPE_NUM_CHAR
            }
        }
        TTC_OBJ_TDS_TYPE_RAW => {
            max_size = input.read_u16be()?.into();
            crate::constants::ORA_TYPE_NUM_RAW
        }
        TTC_OBJ_TDS_TYPE_BINARY_FLOAT => {
            crate::constants::ORA_TYPE_NUM_BINARY_FLOAT
        }
        TTC_OBJ_TDS_TYPE_BINARY_DOUBLE => {
            crate::constants::ORA_TYPE_NUM_BINARY_DOUBLE
        }
        TTC_OBJ_TDS_TYPE_DATE => crate::constants::ORA_TYPE_NUM_DATE,
        TTC_OBJ_TDS_TYPE_TIMESTAMP => {
            scale = input.read_i8()?;
            crate::constants::ORA_TYPE_NUM_TIMESTAMP
        }
        TTC_OBJ_TDS_TYPE_TIMESTAMP_LTZ => {
            scale = input.read_i8()?;
            crate::constants::ORA_TYPE_NUM_TIMESTAMP_LTZ
        }
        TTC_OBJ_TDS_TYPE_TIMESTAMP_TZ => {
            scale = input.read_i8()?;
            crate::constants::ORA_TYPE_NUM_TIMESTAMP_TZ
        }
        TTC_OBJ_TDS_TYPE_BOOLEAN => crate::constants::ORA_TYPE_NUM_BOOLEAN,
        TTC_OBJ_TDS_TYPE_CLOB => {
            csfrm = crate::constants::CS_FORM_IMPLICIT;
            crate::constants::ORA_TYPE_NUM_CLOB
        }
        TTC_OBJ_TDS_TYPE_BLOB => crate::constants::ORA_TYPE_NUM_BLOB,
        TTC_OBJ_TDS_TYPE_OBJECT => {
            input.read_bytes(5)?;
            crate::constants::ORA_TYPE_NUM_OBJECT
        }
        TTC_OBJ_TDS_TYPE_INTERVAL => {
            let interval_type = input.read_u8()?;
            scale = input.read_i8()?;
            precision = input.read_u8()?;
            match interval_type {
                7 => crate::constants::ORA_TYPE_NUM_INTERVAL_YM,
                10 => crate::constants::ORA_TYPE_NUM_INTERVAL_DS,
                _ => return Err(Error::unsupported_tds_type(attr_type)),
            }
        }
        TTC_OBJ_TDS_TYPE_START_EMBED_ADT => {
            while parse_tds_attr(input)?.is_some() {}
            crate::constants::ORA_TYPE_NUM_OBJECT
        }
        TTC_OBJ_TDS_TYPE_END_EMBED_ADT => return Ok(None),
        _ => {
            return Err(Error::unsupported_tds_type(attr_type));
        }
    };
    let db_type = DbType::from_ora_type_and_csfrm(ora_type_num, csfrm);
    Ok(Some(DbObjectParsedTdsAttr { db_type, max_size, precision, scale }))
}

/// Reads the attribute rows returned by `DBMS_PICKLER.GET_TYPE_SHAPE`.
fn read_object_attrs(rows: Vec<Row>) -> Result<Vec<Metadata>, Error> {
    let mut attrs = Vec::new();
    for row in rows {
        let name: Option<String> = row.get(1)?;
        let Some(name) = name else {
            continue;
        };
        let type_owner: Option<String> = row.get(4)?;
        let metadata = if let Some(schema) = type_owner {
            let identity = DbObjectTypeIdentity {
                schema,
                package_name: row.get(5)?,
                name: row.get(3)?,
            };
            let mut metadata = Metadata::new_object_field(
                name,
                crate::DB_TYPE_OBJECT,
                0,
                0,
                0,
            );
            metadata.set_object_type_identity(identity);
            metadata
        } else {
            let type_name: String = row.get(3)?;
            metadata_from_name(name, &type_name, 0, 0, 0)?
        };
        attrs.push(metadata);
    }
    Ok(attrs)
}

/// Owns discovered types under the client lock.
#[derive(Default)]
pub(crate) struct DbObjectTypeCache {
    partial_types: Vec<Arc<DbObjectType>>,
    types_by_name: HashMap<String, Arc<DbObjectType>>,
}

impl DbObjectTypeCache {
    /// Finds a type in either collection or queues a new partial identity.
    fn get_or_create_partial(
        &mut self,
        identity: &DbObjectTypeIdentity,
    ) -> Arc<DbObjectType> {
        let canonical = identity.fqn();
        let cached = self.types_by_name.get(&canonical).or_else(|| {
            self.partial_types.iter().find(|typ| typ.fqn() == canonical)
        });
        if let Some(typ) = cached {
            return typ.clone();
        }
        let typ = Arc::new(DbObjectType::new_internal(identity.clone()));
        self.partial_types.push(typ.clone());
        typ
    }

    /// Completes every type reachable through attributes and collection
    /// elements.
    fn populate_partial_types(
        &mut self,
        client: &mut Client,
        client_ref: &ClientRef,
    ) -> Result<(), Error> {
        let mut index = 0;
        while let Some(typ) = self.partial_types.get(index).cloned() {
            if !typ.is_fully_populated() {
                let lookup =
                    lookup_type(client, client_ref, &typ.lookup_name())?;
                self.populate_type(client, client_ref, &typ, lookup)?;
            }
            index += 1;
        }
        for typ in self.partial_types.drain(..) {
            self.types_by_name.insert(typ.fqn(), typ);
        }
        Ok(())
    }

    /// Populates one type and records the identities of its nested types.
    fn populate_type(
        &mut self,
        client: &mut Client,
        client_ref: &ClientRef,
        typ: &DbObjectType,
        lookup: DbObjectTypeLookup,
    ) -> Result<(), Error> {
        let parsed = if lookup.is_row_type {
            None
        } else {
            Some(parse_tds(&lookup.tds, lookup.attrs.len())?)
        };
        let mut attrs = lookup.attrs;
        for (index, metadata) in attrs.iter_mut().enumerate() {
            if let Some(parsed) = &parsed {
                merge_attribute_metadata(metadata, &parsed.attrs[index]);
            }
            metadata.resolve_object_type(|identity| {
                Ok(self.get_or_create_partial(identity))
            })?;
        }
        let collection_type =
            parsed.as_ref().and_then(|parsed| parsed.collection_type);
        let element = if let Some(mut field) =
            parsed.and_then(|parsed| parsed.element)
        {
            if field.db_type == crate::DB_TYPE_CLOB {
                field.db_type =
                    lookup_collection_element_type(client, client_ref, typ)?;
            }
            let object_type = if field.db_type == crate::DB_TYPE_OBJECT {
                let identity =
                    lookup_collection_element_object(client, client_ref, typ)?;
                if identity.is_xmltype() {
                    field.db_type = crate::DB_TYPE_XMLTYPE;
                    None
                } else {
                    Some(self.get_or_create_partial(&identity))
                }
            } else {
                None
            };
            let mut metadata = Metadata::new_object_field(
                String::new(),
                field.db_type,
                field.max_size,
                field.precision,
                field.scale,
            );
            if let Some(typ) = object_type {
                metadata.set_object_type(typ);
            }
            Some(metadata)
        } else {
            None
        };

        typ.set_body(DbObjectTypeBody {
            attrs,
            element,
            oid: lookup.oid,
            version: lookup.version,
            collection_type,
        })?;
        Ok(())
    }

    /// Removes all types discovered for the client.
    pub(crate) fn clear(&mut self) {
        self.partial_types.clear();
        self.types_by_name.clear();
    }

    /// Returns a completed type, discovering it and its nested types on a miss.
    pub(crate) fn get_type(
        &mut self,
        client: &mut Client,
        client_ref: &ClientRef,
        name: &str,
    ) -> Result<Arc<DbObjectType>, Error> {
        if let Some(typ) = self.types_by_name.get(name) {
            return Ok(Arc::clone(typ));
        }

        let result = (|| {
            let lookup = lookup_type(client, client_ref, name)?;
            let typ = self.get_or_create_partial(&lookup.identity);
            if !typ.is_fully_populated() {
                self.populate_type(client, client_ref, &typ, lookup)?;
            }
            self.populate_partial_types(client, client_ref)?;
            if name != typ.fqn() {
                self.types_by_name.insert(name.to_string(), Arc::clone(&typ));
            }
            Ok(typ)
        })();
        if result.is_err() {
            self.partial_types.clear();
        }
        result
    }
}
