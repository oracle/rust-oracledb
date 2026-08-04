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
// test_2100_json()
//-----------------------------------------------------------------------------

mod common;

use common::conn;
use oracledb;
use rstest::*;
use std::collections::HashMap;
use std::str::FromStr;

/// Validates the array fetched from the database.
fn validate_array(json: oracledb::JsonValue) {
    if let oracledb::JsonValue::JsonArray(array) = json {
        for (ix, child_value) in array.into_iter().enumerate() {
            if let oracledb::JsonValue::JsonObject(map) = child_value {
                // validate number value
                if let Some(oracledb::JsonValue::Number(value)) =
                    map.get("a_key")
                {
                    let expected_value = (ix + 1).to_string();
                    assert_eq!(value.to_string(), expected_value);
                } else {
                    panic!("expected a_key value missing or wrong type");
                }

                // validate string value
                if let Some(oracledb::JsonValue::String(value)) =
                    map.get("b_key")
                {
                    let expected_value = format!("value {}", ix + 1);
                    assert_eq!(value.to_string(), expected_value);
                } else {
                    panic!("expected b_key value missing or wrong type");
                }
            } else {
                panic!("child value is not a JSON object!");
            }
        }
    } else {
        panic!("no JSON array fetched!");
    };
}

/// Validates the object fetched from the database.
fn validate_obj(json: oracledb::JsonValue) {
    if let oracledb::JsonValue::JsonObject(map) = json {
        // test simple scalar values
        assert!(matches!(
            map.get("true_key").expect("missing true_key"),
            oracledb::JsonValue::Boolean(true)
        ));
        assert!(matches!(
            map.get("false_key").expect("missing false_key"),
            oracledb::JsonValue::Boolean(false)
        ));
        assert!(matches!(
            map.get("null_key").expect("missing null_key"),
            oracledb::JsonValue::Null
        ));
        assert!(matches!(
            map.get("bfloat_key").expect("missing bfloat_key"),
            oracledb::JsonValue::BinaryFloat(12.625)
        ));
        assert!(matches!(
            map.get("bdouble_key").expect("missing bdouble_key"),
            oracledb::JsonValue::BinaryDouble(25.25)
        ));

        // test strings that are less than 31 bytes in length
        if let Some(oracledb::JsonValue::String(value)) = map.get("str_key") {
            assert_eq!(value, &String::from("str_value"));
        } else {
            panic!("expected str_key value missing or wrong type");
        }

        // test date
        if let Some(oracledb::JsonValue::Timestamp(value)) =
            map.get("date_key")
        {
            let expected_value =
                oracledb::OracleTimestamp::new_date(2026, 2, 28);
            assert_eq!(value, &expected_value);
        } else {
            panic!("expected date_key value missing or wrong type");
        }

        // test timestamp that can be represented as a date
        if let Some(oracledb::JsonValue::Timestamp(value)) =
            map.get("tstamp7_key")
        {
            let expected_value =
                oracledb::OracleTimestamp::new_date(2026, 3, 1);
            assert_eq!(value, &expected_value);
        } else {
            panic!("expected tstamp7_key value missing or wrong type");
        }

        // test timestamp that cannot be represented as a date
        if let Some(oracledb::JsonValue::Timestamp(value)) =
            map.get("tstamp_key")
        {
            let expected_value = oracledb::OracleTimestamp::new_timestamp(
                2026,
                3,
                2,
                16,
                49,
                25,
                125_000_000,
            );
            assert_eq!(value, &expected_value);
        } else {
            panic!("expected tstamp_key value missing or wrong type");
        }

        // test timestamp with time zone
        if let Some(oracledb::JsonValue::Timestamp(value)) =
            map.get("tstamp_tz_key")
        {
            let expected_value = oracledb::OracleTimestamp::new_timestamp_tz(
                2026,
                3,
                3,
                18,
                31,
                25,
                745_000_000,
                0,
                0,
            );
            assert_eq!(value, &expected_value);
        } else {
            panic!("expected tstamp_tz_key value missing or wrong type");
        }

        // test interval days to seconds
        if let Some(oracledb::JsonValue::IntervalDS(value)) =
            map.get("interval_ds_key")
        {
            let expected_value =
                oracledb::OracleIntervalDS::new(8, 5, 15, 0, 0);
            assert_eq!(value, &expected_value);
        } else {
            panic!("expected interval_ds_key value missing or wrong type");
        }

        // test interval years to months
        if let Some(oracledb::JsonValue::IntervalYM(value)) =
            map.get("interval_ym_key")
        {
            let expected_value = oracledb::OracleIntervalYM::new(1, 9);
            assert_eq!(value, &expected_value);
        } else {
            panic!("expected interval_ym_key value missing or wrong type");
        }

        // test raw values less than 31 bytes in length
        if let Some(oracledb::JsonValue::Raw(value)) = map.get("raw_key") {
            let expected_value = String::from("Raw value").as_bytes().to_vec();
            assert_eq!(value, &expected_value);
        } else {
            panic!("expected raw_key value missing or wrong type");
        }

        // test number values
        if let Some(oracledb::JsonValue::Number(value)) = map.get("num_key") {
            let expected_value =
                String::from("12345678901234567890123456789012345");
            assert_eq!(value.to_string(), expected_value);
        } else {
            panic!("expected num_key value missing or wrong type");
        }

        // test string values that exceed 31 bytes but are less than 255 bytes
        if let Some(oracledb::JsonValue::String(value)) =
            map.get("string_u8_key")
        {
            let expected_value = String::from("A").repeat(64);
            assert_eq!(value.to_string(), expected_value);
        } else {
            panic!("expected string_u8_key value missing or wrong type");
        }

        // test string values that exceed 255 bytes but are less than 65535
        // bytes
        if let Some(oracledb::JsonValue::String(value)) =
            map.get("string_u16_key")
        {
            let expected_value = String::from("A").repeat(500);
            assert_eq!(value.to_string(), expected_value);
        } else {
            panic!("expected string_u16_key value missing or wrong type");
        }
    } else {
        panic!("no JSON object fetched!");
    };
}

/// Validates a JSON object containing a vector fetched from and bound to the
/// database
fn validate_vector(returned: oracledb::JsonValue, expected: &[f32]) {
    // a JSON object should be returned
    let map = match returned {
        oracledb::JsonValue::JsonObject(map) => map,
        _ => panic!("expected JSON object"),
    };

    // the JSON object should contain the key "vector"
    let vector = match map.get("vector") {
        Some(oracledb::JsonValue::Vector(v)) => v,
        other => panic!("expected vector, got {other:?}"),
    };
    match vector {
        oracledb::Vector::Dense(oracledb::VectorData::Float32(vals)) => {
            assert_eq!(vals.as_slice(), expected);
        }
        other => panic!("unexpected vector value: {other:?}"),
    }
}

#[rstest]
/// test fetching JSON object with all scalar data types
fn test_2100(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_json_supported(&conn) {
        return Ok(());
    }
    let row = conn.query_row(
        r#"
        select json_object(
            'str_key' value 'str_value',
            'true_key' value true,
            'false_key' value false,
            'null_key' value null,
            'bfloat_key' value to_binary_float(12.625),
            'bdouble_key' value to_binary_double(25.25),
            'date_key' value to_date('20260228', 'YYYYMMDD'),
            'tstamp7_key' value to_timestamp('20260301', 'YYYYMMDD'),
            'tstamp_key' value to_timestamp('20260302 16:49:25.125',
                'YYYYMMDD HH24:MI:SS.FF3'),
            'tstamp_tz_key' value to_timestamp_tz('20260303 18:31:25.745',
                'YYYYMMDD HH24:MI:SS.FF3'),
            'interval_ds_key' value to_dsinterval('8 05:15:00'),
            'interval_ym_key' value to_yminterval('1-9'),
            'raw_key' value utl_raw.cast_to_raw('Raw value'),
            'num_key' value 12345678901234567890123456789012345,
            'string_u8_key' value rpad('A', 64, 'A'),
            'string_u16_key' value rpad('A', 500, 'A')
            returning json
        )
        "#,
        &[],
    )?;
    let json: oracledb::JsonValue = row.get(0)?;
    validate_obj(json);
    Ok(())
}

#[rstest]
/// test fetching JSON array of objects with shared keys
fn test_2101(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_json_supported(&conn) {
        return Ok(());
    }
    let row = conn.query_row(
        r#"
        select json_array(
            json_object(
                'a_key' value 1,
                'b_key' value 'value 1'
            ),
            json_object(
                'a_key' value 2,
                'b_key' value 'value 2'
            ),
            json_object(
                'a_key' value 3,
                'b_key' value 'value 3'
            )
            returning json
        )
        "#,
        &[],
    )?;
    let json: oracledb::JsonValue = row.get(0)?;
    validate_array(json);
    Ok(())
}

#[rstest]
/// test binding and fetching JSON object
fn test_2102(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_json_supported(&conn) {
        return Ok(());
    }
    let mut map: HashMap<String, oracledb::JsonValue> = HashMap::new();
    map.insert("true_key".to_string(), oracledb::JsonValue::Boolean(true));
    map.insert("false_key".to_string(), oracledb::JsonValue::Boolean(false));
    map.insert("null_key".to_string(), oracledb::JsonValue::Null);
    map.insert(
        "bfloat_key".to_string(),
        oracledb::JsonValue::BinaryFloat(12.625),
    );
    map.insert(
        "bdouble_key".to_string(),
        oracledb::JsonValue::BinaryDouble(25.25),
    );
    let str_value = String::from("str_value");
    map.insert(
        "str_key".to_string(),
        oracledb::JsonValue::String(str_value),
    );
    let date_value = oracledb::OracleTimestamp::new_date(2026, 2, 28);
    map.insert(
        "date_key".to_string(),
        oracledb::JsonValue::Timestamp(date_value),
    );
    let tstamp7_value = oracledb::OracleTimestamp::new_date(2026, 3, 1);
    map.insert(
        "tstamp7_key".to_string(),
        oracledb::JsonValue::Timestamp(tstamp7_value),
    );
    let tstamp_value = oracledb::OracleTimestamp::new_timestamp(
        2026,
        3,
        2,
        16,
        49,
        25,
        125_000_000,
    );
    map.insert(
        "tstamp_key".to_string(),
        oracledb::JsonValue::Timestamp(tstamp_value),
    );
    let tstamp_tz_value = oracledb::OracleTimestamp::new_timestamp(
        2026,
        3,
        3,
        18,
        31,
        25,
        745_000_000,
    );
    map.insert(
        "tstamp_tz_key".to_string(),
        oracledb::JsonValue::Timestamp(tstamp_tz_value),
    );
    let ds_value = oracledb::OracleIntervalDS::new(8, 5, 15, 0, 0);
    map.insert(
        "interval_ds_key".to_string(),
        oracledb::JsonValue::IntervalDS(ds_value),
    );
    let ym_value = oracledb::OracleIntervalYM::new(1, 9);
    map.insert(
        "interval_ym_key".to_string(),
        oracledb::JsonValue::IntervalYM(ym_value),
    );
    let raw_value = String::from("Raw value").as_bytes().to_vec();
    map.insert("raw_key".to_string(), oracledb::JsonValue::Raw(raw_value));
    let num_str = String::from("12345678901234567890123456789012345");
    let num_value = oracledb::OracleNumber::from_str(&num_str).unwrap();
    map.insert(
        "num_key".to_string(),
        oracledb::JsonValue::Number(num_value),
    );
    let str_u8_value = String::from("A").repeat(64);
    map.insert(
        "string_u8_key".to_string(),
        oracledb::JsonValue::String(str_u8_value),
    );
    let str_u16_value = String::from("A").repeat(500);
    map.insert(
        "string_u16_key".to_string(),
        oracledb::JsonValue::String(str_u16_value),
    );
    let input_json = oracledb::JsonValue::JsonObject(map);
    let row = conn.query_row("select :1 from dual", &[&input_json])?;
    let fetched_json: oracledb::JsonValue = row.get(0)?;
    validate_obj(fetched_json);
    Ok(())
}

#[rstest]
/// test binding and fetching JSON array of objects
fn test_2103(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_json_supported(&conn) {
        return Ok(());
    }
    let mut array: Vec<oracledb::JsonValue> = Vec::new();
    for ix in 1..=3 {
        let mut map: HashMap<String, oracledb::JsonValue> = HashMap::new();
        let num_value = oracledb::OracleNumber::from(ix);
        map.insert(
            "a_key".to_string(),
            oracledb::JsonValue::Number(num_value),
        );
        let str_value = format!("value {ix}");
        map.insert(
            "b_key".to_string(),
            oracledb::JsonValue::String(str_value),
        );
        array.push(oracledb::JsonValue::JsonObject(map));
    }
    let input_json = oracledb::JsonValue::JsonArray(array);
    let row = conn.query_row("select :1 from dual", &[&input_json])?;
    let fetched_json: oracledb::JsonValue = row.get(0)?;
    validate_array(fetched_json);
    Ok(())
}

#[rstest]
/// test fetching JSON object with vector type
fn test_2104(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_json_supported(&conn) {
        return Ok(());
    }
    let expected: Vec<f32> = vec![1.0, 2.0];
    let vec_str = format!(
        "[{}]",
        expected
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    let row = conn.query_row(
        &format!(
            r#"
            select json_object(
                'vector' value vector('{}', {}, float32)
                returning json
            )
            "#,
            vec_str,
            expected.len()
        ),
        &[],
    )?;
    let json: oracledb::JsonValue = row.get(0)?;
    validate_vector(json, expected.as_slice());
    Ok(())
}

#[rstest]
/// test binding vector in JSON
fn test_2105(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_json_supported(&conn) {
        return Ok(());
    }
    let expected = [1.0, 2.0];
    let vector = oracledb::Vector::Dense(oracledb::VectorData::Float32(
        expected.to_vec(),
    ));
    let mut obj = HashMap::new();
    obj.insert("vector".to_string(), oracledb::JsonValue::Vector(vector));
    let json = oracledb::JsonValue::JsonObject(obj);
    let row = conn.query_row("select :1", &[&json])?;
    let returned: oracledb::JsonValue = row.get(0)?;
    validate_vector(returned, &expected);
    Ok(())
}

#[rstest]
/// test fetching null values for JSON
fn test_2106(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_json_supported(&conn) {
        return Ok(());
    }
    let cursor = conn.query("select cast(null as json) from dual", &[])?;
    for row in cursor {
        let row = row?;
        let fetched: Option<oracledb::JsonValue> = row.get(0)?;
        assert!(fetched.is_none());
    }
    Ok(())
}

#[rstest]
/// test JSON metadata for native JSON columns
fn test_2107(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_json_supported(&conn) {
        return Ok(());
    }
    let cursor = conn.query(
        r#"
        select json_object('metadata' value true returning json) as json_col
        from dual
        "#,
        &[],
    )?;
    let columns = cursor.columns();
    assert_eq!(columns[0].name(), "JSON_COL");
    assert_eq!(columns[0].db_type(), &oracledb::DB_TYPE_JSON);
    Ok(())
}

#[rstest]
/// test JSON nested objects and arrays with mixed scalar null values
fn test_2108(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_json_supported(&conn) {
        return Ok(());
    }
    let row = conn.query_row(
        r#"
        select json_object(
            'items' value json_array(
                json_object('id' value 1, 'name' value 'alpha'),
                json_object('id' value 2, 'name' value null,
                            'active' value true)
                returning json
            ),
            'count' value 2,
            'empty' value null
            returning json
        )
        from dual
        "#,
        &[],
    )?;
    let json: oracledb::JsonValue = row.get(0)?;
    let oracledb::JsonValue::JsonObject(obj) = json else {
        panic!("expected JSON object");
    };
    assert!(matches!(obj.get("empty"), Some(oracledb::JsonValue::Null)));
    let Some(oracledb::JsonValue::Number(count)) = obj.get("count") else {
        panic!("expected JSON number");
    };
    assert_eq!(count.to_string(), "2");
    let Some(oracledb::JsonValue::JsonArray(items)) = obj.get("items") else {
        panic!("expected JSON array");
    };
    assert_eq!(items.len(), 2);
    let oracledb::JsonValue::JsonObject(second) = &items[1] else {
        panic!("expected nested JSON object");
    };
    assert!(matches!(
        second.get("name"),
        Some(oracledb::JsonValue::Null)
    ));
    assert!(matches!(
        second.get("active"),
        Some(oracledb::JsonValue::Boolean(true))
    ));
    Ok(())
}
