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
// test_3700_dbobject.rs
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use rstest::*;

#[rstest]
/// Tests recursive collection discovery and cached object identity.
fn test_3700(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _address_type_guard = common::create_type(
        &conn,
        "test_3700_address_type",
        "object (street varchar2(100), zip number(5))",
    )?;
    let _person_type_guard = common::create_type(
        &conn,
        "test_3700_person_type",
        "object (id number, address test_3700_address_type)",
    )?;
    let _people_type_guard = common::create_type(
        &conn,
        "test_3700_people_type",
        "table of test_3700_person_type",
    )?;
    let _numbers_type_guard = common::create_type(
        &conn,
        "test_3700_numbers_type",
        "varray(10) of number",
    )?;

    // Discover the collection first: neither its element nor that element's
    // nested address type has been cached yet.
    let people_type = conn.get_type("TEST_3700_PEOPLE_TYPE")?;
    assert!(people_type.is_collection());
    let person_type =
        people_type.element_metadata().unwrap().object_type()?.unwrap();
    let person_attrs = person_type.attributes();
    assert_eq!(person_attrs.len(), 2);
    let nested_type = person_attrs[1].object_type()?.unwrap();
    let address_attrs = nested_type.attributes();
    assert_eq!(address_attrs.len(), 2);
    assert_eq!(address_attrs[0].name(), "STREET");
    assert_eq!(address_attrs[1].name(), "ZIP");
    assert_eq!(person_type, conn.get_type("TEST_3700_PERSON_TYPE")?);
    assert_eq!(nested_type, conn.get_type("TEST_3700_ADDRESS_TYPE")?);
    assert_eq!(people_type, conn.get_type("TEST_3700_PEOPLE_TYPE")?);

    let numbers_type = conn.get_type("TEST_3700_NUMBERS_TYPE")?;
    assert!(numbers_type.is_collection());
    let element_type = numbers_type.element_metadata().unwrap();
    assert_eq!(element_type.db_type(), oracledb::DB_TYPE_NUMBER);

    Ok(())
}

#[rstest]
/// Tests looking up an object type that does not exist.
fn test_3701(conn: oracledb::Connection) {
    let name = "A TYPE THAT DOES NOT EXIST";
    let error = conn.get_type(name).unwrap_err();
    assert_eq!(
        error.kind(),
        &oracledb::ErrorKind::InvalidDbObjectTypeName(name.to_string())
    );
}

#[rstest]
/// Tests metadata discovery for package records and associative-array elements.
fn test_3702(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let package_name = "TEST_3702_PACKAGE";
    let _package_guard = common::create_package(
        &conn,
        package_name,
        r#"
        type record_type is record
            (id pls_integer, label nvarchar2(30), enabled boolean);
        type record_list is table of record_type index by binary_integer;
        type number_list is table of number index by binary_integer;
        "#,
    )?;
    let schema: String =
        conn.query_row("select user from dual", &[])?.get(0)?;
    let records_type =
        conn.get_type(&format!("{package_name}.RECORD_LIST"))?;
    assert!(records_type.is_collection());
    assert_eq!(records_type.name(), "RECORD_LIST");
    assert_eq!(records_type.schema(), schema);
    assert_eq!(records_type.package_name(), Some(package_name));
    assert!(records_type.attributes().is_empty());
    let element_type = records_type.element_metadata().unwrap();
    assert_eq!(element_type.db_type(), oracledb::DB_TYPE_OBJECT);
    let record_type = element_type.object_type()?.unwrap();
    assert!(!record_type.is_collection());
    assert!(record_type.element_metadata().is_none());
    assert_eq!(record_type.schema(), schema);
    assert_eq!(record_type.package_name(), Some(package_name));
    let attrs = record_type.attributes();
    assert_eq!(attrs.len(), 3);
    assert_eq!(attrs[0].name(), "ID");
    assert_eq!(attrs[0].db_type(), oracledb::DB_TYPE_BINARY_INTEGER);
    assert_eq!(attrs[1].name(), "LABEL");
    assert_eq!(attrs[1].db_type(), oracledb::DB_TYPE_NVARCHAR);
    assert!(attrs[1].max_size() >= 30);
    assert_eq!(attrs[2].name(), "ENABLED");
    assert_eq!(attrs[2].db_type(), oracledb::DB_TYPE_BOOLEAN);
    assert_eq!(
        record_type,
        conn.get_type(&format!("{package_name}.RECORD_TYPE"))?
    );
    let numbers_type =
        conn.get_type(&format!("{package_name}.NUMBER_LIST"))?;
    assert_eq!(
        numbers_type.element_metadata().unwrap().db_type(),
        oracledb::DB_TYPE_NUMBER
    );
    assert_eq!(
        records_type,
        conn.get_type(&format!("{schema}.{package_name}.RECORD_LIST"))?
    );
    Ok(())
}

#[rstest]
/// Tests attribute metadata for object and row types.
fn test_3703(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _address_type_guard = common::create_type(
        &conn,
        "test_3703_address_type",
        "object (street varchar2(100), zip number(5))",
    )?;
    let _person_type_guard = common::create_type(
        &conn,
        "test_3703_person_type",
        "object (id number, address test_3703_address_type)",
    )?;
    let _people_type_guard = common::create_type(
        &conn,
        "test_3703_people_type",
        "table of test_3703_person_type",
    )?;
    let table_name = "TEST_3703_ALL_TYPES_TAB";
    let sql_type_name = "TEST_3703_ALL_TYPES".to_string();
    let row_type_name = format!("{table_name}%ROWTYPE");
    let _table_guard = common::create_table_with_options(
        &conn,
        table_name,
        r#"
            NUMBERVALUE number,
            STRINGVALUE varchar2(60 byte),
            FIXEDCHARVALUE char(10 byte),
            NSTRINGVALUE nvarchar2(60),
            NFIXEDCHARVALUE nchar(10),
            RAWVALUE raw(16),
            INTVALUE integer,
            SMALLINTVALUE smallint,
            REALVALUE real,
            DECIMALVALUE number(20, 6),
            DOUBLEPRECISIONVALUE double precision,
            FLOATVALUE float,
            BINARYFLOATVALUE binary_float,
            BINARYDOUBLEVALUE binary_double,
            DATEVALUE date,
            TIMESTAMPVALUE timestamp(3),
            TIMESTAMPTZVALUE timestamp(4) with time zone,
            TIMESTAMPLTZVALUE timestamp(5) with local time zone,
            INTERVALDSVALUE interval day(4) to second(2),
            INTERVALYMVALUE interval year(3) to month,
            CLOBVALUE clob,
            NCLOBVALUE nclob,
            BLOBVALUE blob,
            SUBOBJECTVALUE test_3703_address_type,
            SUBOBJECTARRAY test_3703_people_type,
            INVISIBLEVALUE number invisible
            "#,
        "nested table SUBOBJECTARRAY store as test_3703_all_types_tab_nt",
    )?;
    let _type_guard = common::create_type(
        &conn,
        &sql_type_name,
        r#"
            object (
                NUMBERVALUE number,
                STRINGVALUE varchar2(60 byte),
                FIXEDCHARVALUE char(10 byte),
                NSTRINGVALUE nvarchar2(60),
                NFIXEDCHARVALUE nchar(10),
                RAWVALUE raw(16),
                INTVALUE integer,
                SMALLINTVALUE smallint,
                REALVALUE real,
                DECIMALVALUE number(20, 6),
                DOUBLEPRECISIONVALUE double precision,
                FLOATVALUE float,
                BINARYFLOATVALUE binary_float,
                BINARYDOUBLEVALUE binary_double,
                DATEVALUE date,
                TIMESTAMPVALUE timestamp(3),
                TIMESTAMPTZVALUE timestamp(4) with time zone,
                TIMESTAMPLTZVALUE timestamp(5) with local time zone,
                INTERVALDSVALUE interval day(4) to second(2),
                INTERVALYMVALUE interval year(3) to month,
                CLOBVALUE clob,
                NCLOBVALUE nclob,
                BLOBVALUE blob,
                SUBOBJECTVALUE test_3703_address_type,
                SUBOBJECTARRAY test_3703_people_type
            )
            "#,
    )?;

    let schema: String =
        conn.query_row("select user from dual", &[])?.get(0)?;
    for name in [&row_type_name, &sql_type_name] {
        let typ = conn.get_type(name)?;
        assert_eq!(typ.name(), name);
        assert_eq!(typ.schema(), schema);
        assert_eq!(typ.package_name(), None);
        assert!(!typ.is_collection());
        assert!(typ.element_metadata().is_none());

        let expected = vec![
            ("NUMBERVALUE", oracledb::DB_TYPE_NUMBER, 0, -127, 0),
            ("STRINGVALUE", oracledb::DB_TYPE_VARCHAR, 0, 0, 60),
            ("FIXEDCHARVALUE", oracledb::DB_TYPE_CHAR, 0, 0, 10),
            ("NSTRINGVALUE", oracledb::DB_TYPE_NVARCHAR, 0, 0, 120),
            ("NFIXEDCHARVALUE", oracledb::DB_TYPE_NCHAR, 0, 0, 20),
            ("RAWVALUE", oracledb::DB_TYPE_RAW, 0, 0, 16),
            ("INTVALUE", oracledb::DB_TYPE_NUMBER, 38, 0, 0),
            ("SMALLINTVALUE", oracledb::DB_TYPE_NUMBER, 38, 0, 0),
            ("REALVALUE", oracledb::DB_TYPE_NUMBER, 63, -127, 0),
            ("DECIMALVALUE", oracledb::DB_TYPE_NUMBER, 20, 6, 0),
            ("DOUBLEPRECISIONVALUE", oracledb::DB_TYPE_NUMBER, 126, -127, 0),
            ("FLOATVALUE", oracledb::DB_TYPE_NUMBER, 126, -127, 0),
            ("BINARYFLOATVALUE", oracledb::DB_TYPE_BINARY_FLOAT, 0, 0, 0),
            ("BINARYDOUBLEVALUE", oracledb::DB_TYPE_BINARY_DOUBLE, 0, 0, 0),
            ("DATEVALUE", oracledb::DB_TYPE_DATE, 0, 0, 0),
            ("TIMESTAMPVALUE", oracledb::DB_TYPE_TIMESTAMP, 0, 3, 0),
            ("TIMESTAMPTZVALUE", oracledb::DB_TYPE_TIMESTAMP_TZ, 0, 4, 0),
            ("TIMESTAMPLTZVALUE", oracledb::DB_TYPE_TIMESTAMP_LTZ, 0, 5, 0),
            ("INTERVALDSVALUE", oracledb::DB_TYPE_INTERVAL_DS, 4, 2, 0),
            ("INTERVALYMVALUE", oracledb::DB_TYPE_INTERVAL_YM, 3, 0, 0),
            ("CLOBVALUE", oracledb::DB_TYPE_CLOB, 0, 0, 0),
            ("NCLOBVALUE", oracledb::DB_TYPE_NCLOB, 0, 0, 0),
            ("BLOBVALUE", oracledb::DB_TYPE_BLOB, 0, 0, 0),
            ("SUBOBJECTVALUE", oracledb::DB_TYPE_OBJECT, 0, 0, 0),
            ("SUBOBJECTARRAY", oracledb::DB_TYPE_OBJECT, 0, 0, 0),
        ];
        let attrs = typ.attributes();
        assert_eq!(attrs.len(), expected.len());
        for (metadata, (name, db_type, precision, scale, max_size)) in
            attrs.iter().zip(expected)
        {
            assert_eq!(
                (
                    metadata.name(),
                    metadata.db_type(),
                    metadata.precision(),
                    metadata.scale(),
                    metadata.max_size()
                ),
                (name, db_type, precision, scale, max_size),
                "attribute {name}"
            );
            match name {
                "SUBOBJECTVALUE" => {
                    let address_type = metadata.object_type()?.unwrap();
                    assert_eq!(address_type.attributes().len(), 2);
                    assert_eq!(
                        address_type,
                        conn.get_type("TEST_3703_ADDRESS_TYPE")?
                    );
                }
                "SUBOBJECTARRAY" => {
                    let collection_type = metadata.object_type()?.unwrap();
                    assert!(collection_type.is_collection());
                    assert!(collection_type.attributes().is_empty());
                    let element_type =
                        collection_type.element_metadata().unwrap();
                    let person_type = element_type.object_type()?.unwrap();
                    let person_attrs = person_type.attributes();
                    assert_eq!(person_attrs.len(), 2);
                    assert_eq!(
                        person_attrs[1]
                            .object_type()?
                            .unwrap()
                            .attributes()
                            .len(),
                        2
                    );
                    assert_eq!(
                        collection_type,
                        conn.get_type("TEST_3703_PEOPLE_TYPE")?
                    );
                    assert_eq!(
                        person_type,
                        conn.get_type("TEST_3703_PERSON_TYPE")?
                    );
                }
                _ => assert!(metadata.object_type()?.is_none()),
            }
        }
    }
    Ok(())
}

#[rstest]
/// Tests that retained metadata keeps nested types alive after the connection
/// closes.
fn test_3704(mut conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let child_type_guard = common::create_type(
        &conn,
        "test_3704_child",
        "object (value number)",
    )?;
    let parent_type_guard = common::create_type(
        &conn,
        "test_3704_parent",
        "object (child test_3704_child)",
    )?;
    let parent_type = conn.get_type("TEST_3704_PARENT")?;
    let attrs = parent_type.attributes();
    let child_type = attrs[0].object_type()?.unwrap();
    let child_ref = std::sync::Arc::downgrade(&child_type);
    drop(parent_type_guard);
    drop(child_type_guard);

    // Keep the client allocated to check that close releases the cache itself.
    let statement = conn.statement("select 1 from dual")?;
    conn.close()?;

    assert_eq!(parent_type.attributes()[0].name(), "CHILD");
    assert!(std::sync::Arc::ptr_eq(
        &attrs[0].object_type()?.unwrap(),
        &child_type,
    ));
    drop(child_type);
    assert_eq!(
        attrs[0].object_type()?.unwrap().attributes()[0].name(),
        "VALUE"
    );
    drop(statement);
    drop(parent_type);
    assert!(child_ref.upgrade().is_none());
    Ok(())
}

#[rstest]
/// Tests object type resolution from describe metadata without fetching
/// values.
fn test_3705(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _type_guard =
        common::create_type(&conn, "test_3705_type", "object (value number)")?;
    let mut statement = conn
        .statement("select test_3705_type(1) as obj from dual")?
        .build()?;
    statement.ensure_fully_parsed()?;
    let metadata = &statement.out_metadata()[0];
    assert_eq!(metadata.name(), "OBJ");
    let typ = metadata.object_type()?.unwrap();
    assert_eq!(typ.attributes()[0].name(), "VALUE");
    assert_eq!(typ, conn.get_type("TEST_3705_TYPE")?);

    let mut result = conn.execute_named(
        r#"
        begin
            open :cursor for
                select test_3705_type(1) as obj
                from dual;
        end;
        "#,
        &[("cursor", &oracledb::DB_TYPE_CURSOR)],
    )?;
    let cursor: oracledb::Cursor = result.out_bind_data().take("cursor")?;
    assert_eq!(cursor.columns()[0].object_type()?.unwrap(), typ);
    Ok(())
}

#[rstest]
/// Tests consistent XMLTYPE metadata for attributes, row types, elements and
/// columns.
fn test_3706(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let _obj_type_guard = common::create_type(
        &conn,
        "test_3706_obj_type",
        "object (doc sys.xmltype)",
    )?;
    let _collection_type_guard = common::create_type(
        &conn,
        "test_3706_collection_type",
        "table of sys.xmltype",
    )?;
    let _table_guard =
        common::create_table(&conn, "test_3706_tab", "doc sys.xmltype")?;
    for name in ["TEST_3706_OBJ_TYPE", "TEST_3706_TAB%ROWTYPE"] {
        let typ = conn.get_type(name)?;
        let metadata = &typ.attributes()[0];
        assert_eq!(metadata.db_type(), oracledb::DB_TYPE_XMLTYPE);
        assert!(metadata.object_type()?.is_none());
    }
    let typ = conn.get_type("TEST_3706_COLLECTION_TYPE")?;
    let element = typ.element_metadata().unwrap();
    assert_eq!(element.db_type(), oracledb::DB_TYPE_XMLTYPE);
    assert!(element.object_type()?.is_none());

    let mut statement = conn
        .statement("select xmltype('<root/>') as doc from dual")?
        .build()?;
    statement.ensure_fully_parsed()?;
    let metadata = &statement.out_metadata()[0];
    assert_eq!(metadata.db_type(), oracledb::DB_TYPE_XMLTYPE);
    assert!(metadata.object_type()?.is_none());
    Ok(())
}
