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

#[rstest]
/// Takes an object and reads its decoded nested values through shared
/// references.
fn test_3707(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let address_type_name = "TEST_3707_ADDRESS";
    let _address_type_guard = common::create_type(
        &conn,
        address_type_name,
        "object (street varchar2(100), zip number)",
    )?;
    let addresses_type_name = "TEST_3707_ADDRESSES";
    let _addresses_type_guard = common::create_type(
        &conn,
        addresses_type_name,
        &format!("table of {address_type_name}"),
    )?;
    let person_type_name = "TEST_3707_PERSON";
    let _person_type_guard = common::create_type(
        &conn,
        person_type_name,
        &format!(
            r#"
            object (
                id number,
                address {address_type_name},
                addresses {addresses_type_name}
            )
            "#,
        ),
    )?;
    let sql = format!(
        r#"
        select {person_type_name}(
            7,
            {address_type_name}('Main', 42),
            {addresses_type_name}({address_type_name}('Other', 43), null)
        ) from dual
        "#,
    );
    let mut row = conn.query_row(&sql, &[])?;
    let mut person: oracledb::DbObject = row.take(0)?;
    assert_eq!(person.get::<i32>("ID")?, 7);
    let address: oracledb::DbObject = person.take("ADDRESS")?;
    assert_eq!(address.get::<&str>("STREET")?, "Main");
    assert_eq!(address.get::<i32>("ZIP")?, 42);
    let addresses: oracledb::DbCollection = person.take("ADDRESSES")?;
    let mut elements: Vec<Option<oracledb::DbObject>> =
        addresses.try_into()?;
    //    let mut elements = addresses.to_vec::<Option<oracledb::DbObject>>()?;
    assert_eq!(elements.len(), 2);
    assert_eq!(elements[0].take().unwrap().get::<i32>("ZIP")?, 43);
    assert!(elements[1].is_none());
    assert_eq!(person.object_type(), &conn.get_type(person_type_name)?);
    Ok(())
}

#[rstest]
/// Distinguishes null objects, null attributes, and empty collections.
fn test_3708(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let obj_type_name = "TEST_3708_OBJ";
    let _obj_type_guard =
        common::create_type(&conn, obj_type_name, "object (value number)")?;
    let array_type_name = "TEST_3708_ARRAY";
    let _array_type_guard =
        common::create_type(&conn, array_type_name, "varray(10) of number")?;
    let sql = format!(
        r#"
        select
            cast(null as {obj_type_name}),
            {obj_type_name}(null),
            {array_type_name}(),
            {array_type_name}(1, null, 3)
        from dual
        "#,
    );
    let mut row = conn.query_row(&sql, &[])?;
    assert!(row.take::<Option<oracledb::DbObject>>(0)?.is_none());
    assert!(
        row.take::<oracledb::DbObject>(1)?.get::<Option<i32>>(0)?.is_none()
    );
    let empty_array: oracledb::DbCollection = row.take(2)?;
    assert!(TryInto::<Vec<i32>>::try_into(empty_array)?.is_empty());
    let array: oracledb::DbCollection = row.take(3)?;
    let values: Vec<Option<i32>> = array.try_into()?;
    assert_eq!(values, vec![Some(1), None, Some(3)]);
    Ok(())
}

#[rstest]
/// Fetches identical object values over several batches.
fn test_3709(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let obj_type_name = "TEST_3709_OBJ";
    let _obj_type_guard = common::create_type(
        &conn,
        obj_type_name,
        "object (value number, text varchar2(20))",
    )?;
    let sql = format!(
        "select {}(7, 'same') from dual connect by level <= 9",
        obj_type_name
    );
    let cursor = conn
        .statement(&sql)?
        .prefetch_rows(0)
        .fetch_array_size(2)
        .build()?
        .query(&[])?;
    let mut count = 0;
    for row in cursor {
        let mut row = row?;
        let value: oracledb::DbObject = row.take(0)?;
        assert_eq!(value.get::<i32>(0)?, 7);
        assert_eq!(value.get::<String>(1)?, "same");
        count += 1;
    }
    assert_eq!(count, 9);
    Ok(())
}

#[rstest]
/// Reads LOB attributes after their object image has been decoded.
fn test_3710(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    use std::io::Read;
    let obj_type_name = "TEST_3710_OBJ";
    let _obj_type_guard = common::create_type(
        &conn,
        obj_type_name,
        "object (text clob, national_text nclob, data blob)",
    )?;
    let sql = format!(
        r#"
        select
            {}(
                to_clob('hello'),
                to_nclob('world'),
                to_blob(hextoraw('010203'))
            )
        from dual connect by level <= 2
        "#,
        obj_type_name,
    );
    let cursor = conn
        .statement(&sql)?
        .prefetch_rows(0)
        .fetch_array_size(1)
        .build()?
        .query(&[])?;
    for row_result in cursor {
        let mut row = row_result?;
        let mut value: oracledb::DbObject = row.take(0)?;
        let mut text: oracledb::Lob = value.take("TEXT")?;
        let mut result = String::new();
        text.read_to_string(&mut result).unwrap();
        assert_eq!(result, "hello");
        let mut national: oracledb::Lob = value.take("NATIONAL_TEXT")?;
        result.clear();
        national.read_to_string(&mut result).unwrap();
        assert_eq!(result, "world");
        let mut binary: oracledb::Lob = value.take("DATA")?;
        let mut bytes = Vec::new();
        binary.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, [1, 2, 3]);
    }
    Ok(())
}

#[rstest]
/// Fetches the supported scalar attribute encodings.
fn test_3711(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let obj_type_name = "TEST_3711_OBJ";
    let _obj_type_guard = common::create_type(
        &conn,
        obj_type_name,
        r#"
        object (
            n number,
            s varchar2(60),
            c char(10),
            ns nvarchar2(60),
            nc nchar(10),
            r raw(16),
            bf binary_float,
            bd binary_double,
            d date,
            ts timestamp,
            tz timestamp with time zone,
            ltz timestamp with local time zone
        )"#,
    )?;
    let mut row = conn.query_row(
        &format!(
            r#"
        select {}(
            12.5,
            'hello',
            'fixed',
            N'national',
            N'fixed',
            hextoraw('010203'),
            to_binary_float(1.25),
            to_binary_double(2.5),
            date '2020-01-02',
            timestamp '2021-01-02 03:04:05.123456',
            to_timestamp_tz('2022-01-02 03:04:05 +02:00',
                            'YYYY-MM-DD HH24:MI:SS TZH:TZM'),
            timestamp '2023-01-02 03:04:05'
        )
        from dual
        "#,
            obj_type_name,
        ),
        &[],
    )?;
    let value: oracledb::DbObject = row.take(0)?;
    assert_eq!(value.get::<f64>("N")?, 12.5);
    assert_eq!(value.get::<String>("S")?, "hello");
    assert_eq!(value.get::<String>("C")?, "fixed     ");
    assert_eq!(value.get::<String>("NS")?, "national");
    assert_eq!(value.get::<String>("NC")?, "fixed     ");
    assert_eq!(value.get::<Vec<u8>>("R")?, [1, 2, 3]);
    assert_eq!(value.get::<f32>("BF")?, 1.25);
    assert_eq!(value.get::<f64>("BD")?, 2.5);
    assert_eq!(value.get::<oracledb::OracleTimestamp>("D")?.year(), 2020);
    assert_eq!(
        value.get::<oracledb::OracleTimestamp>("TS")?.nanoseconds(),
        123456000
    );
    assert_eq!(value.get::<oracledb::OracleTimestamp>("TZ")?.year(), 2022);
    assert_eq!(value.get::<oracledb::OracleTimestamp>("LTZ")?.year(), 2023);
    Ok(())
}

#[rstest]
/// Discovers and decodes object and collection columns returned by a REF
/// CURSOR.
fn test_3712(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let obj_type_name = "TEST_3712_OBJ";
    let _obj_type_guard =
        common::create_type(&conn, obj_type_name, "object (value number)")?;
    let array_type_name = "TEST_3712_ARRAY";
    let _array_type_guard =
        common::create_type(&conn, array_type_name, "varray(10) of number")?;
    let mut result = conn.execute_named(
        &format!(
            "begin open :cursor for select {}(7), {}(1, 2) from dual; end;",
            obj_type_name, array_type_name
        ),
        &[("cursor", &oracledb::DB_TYPE_CURSOR)],
    )?;
    let mut cursor: oracledb::Cursor =
        result.out_bind_data().take("cursor")?;
    let mut row = cursor.next().unwrap()?;
    let obj: oracledb::DbObject = row.take(0)?;
    assert_eq!(obj.get::<i32>("VALUE")?, 7);
    let array: oracledb::DbCollection = row.take(1)?;
    let values: Vec<i32> = array.try_into()?;
    assert_eq!(values, [1, 2]);
    assert!(cursor.next().is_none());
    Ok(())
}

#[rstest]
/// Fetches short and long string attributes using object-image lengths.
fn test_3713(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let obj_type_name = "TEST_3713_OBJ";
    let _obj_type_guard = common::create_type(
        &conn,
        obj_type_name,
        "object (text_value varchar2(1000), trailing_value number)",
    )?;
    let sql = format!("select {}(:1, 42) from dual", obj_type_name);
    for length in [100, 256, 512] {
        let expected = "x".repeat(length);
        let mut row = conn.query_row(&sql, &[&expected])?;
        let value: oracledb::DbObject = row.take(0)?;
        assert_eq!(value.get::<String>("TEXT_VALUE")?, expected);
        assert_eq!(value.get::<i32>("TRAILING_VALUE")?, 42);
    }
    Ok(())
}

#[rstest]
/// Fetches nested images exceeding 255 bytes with only short scalar payloads.
fn test_3714(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let obj_type_name = "TEST_3714_OBJ";
    let _obj_type_guard = common::create_type(
        &conn,
        obj_type_name,
        "object (first_value varchar2(100), second_value varchar2(100),
                 third_value varchar2(100), trailing_value number)",
    )?;
    let array_type_name = "TEST_3714_ARRAY";
    let _array_type_guard = common::create_type(
        &conn,
        array_type_name,
        &format!("table of {}", obj_type_name),
    )?;
    let sql = format!(
        "select {}({}(:1, :2, :3, 42), {}('after', 'nested', 'image', 99))
         from dual",
        array_type_name, obj_type_name, obj_type_name,
    );
    // Object elements in a collection have their own length-prefixed image.
    // Three 100-byte attributes make that image longer than 255 bytes without
    // requiring a long length prefix for any individual scalar attribute.
    for length in [10, 100] {
        let first = "a".repeat(length);
        let second = "b".repeat(length);
        let third = "c".repeat(length);
        let mut row = conn.query_row(&sql, &[&first, &second, &third])?;
        let collection: oracledb::DbCollection = row.take(0)?;
        let values: Vec<oracledb::DbObject> = collection.try_into()?;
        assert_eq!(values.len(), 2);
        assert_eq!(values[0].get::<String>("FIRST_VALUE")?, first);
        assert_eq!(values[0].get::<String>("SECOND_VALUE")?, second);
        assert_eq!(values[0].get::<String>("THIRD_VALUE")?, third);
        assert_eq!(values[0].get::<i32>("TRAILING_VALUE")?, 42);
        assert_eq!(values[1].get::<String>("FIRST_VALUE")?, "after");
        assert_eq!(values[1].get::<String>("SECOND_VALUE")?, "nested");
        assert_eq!(values[1].get::<String>("THIRD_VALUE")?, "image");
        assert_eq!(values[1].get::<i32>("TRAILING_VALUE")?, 99);
    }
    Ok(())
}
