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
// test_3800_arrow_requested_schema.rs
//
// Tests fetching Arrow query results using a requested schema.
//-----------------------------------------------------------------------------

#![cfg(feature = "arrow")]

mod common;

use std::collections::HashMap;
use std::sync::Arc;

use arrow_array::cast::AsArray;
use arrow_array::types::{
    Int8Type, Int16Type, Int32Type, Int64Type, UInt8Type, UInt16Type,
    UInt32Type, UInt64Type,
};
use arrow_array::{
    Array, BinaryArray, BooleanArray, Decimal128Array, Decimal256Array,
    Float32Array, Float64Array, Int64Array, LargeBinaryArray,
    LargeStringArray, StringArray, TimestampMicrosecondArray,
    TimestampMillisecondArray, TimestampNanosecondArray, TimestampSecondArray,
    UInt64Array,
};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use rstest::*;

use common::conn;

#[rstest]
/// test requested types, field names, and metadata are used in the result
fn test_3800(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let metadata =
        HashMap::from([("description".into(), "identifier".into())]);
    let schema = Arc::new(Schema::new_with_metadata(
        vec![
            Field::new("REQUESTED_ID", DataType::Int64, false)
                .with_metadata(metadata),
            Field::new("REQUESTED_NOTE", DataType::LargeUtf8, true),
        ],
        HashMap::from([("source".into(), "requested".into())]),
    ));
    let batch = conn
        .statement(
            r#"
            select cast(42 as number(4, 0)) as ID,
                   cast('note' as varchar2(10)) as NOTE from dual
            "#,
        )?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::default())?;
    assert_eq!(batch.schema(), schema);
    assert_eq!(batch.num_rows(), 1);
    let ids = batch.column(0).as_any().downcast_ref::<Int64Array>().unwrap();
    assert_eq!(ids.value(0), 42);
    let notes =
        batch.column(1).as_any().downcast_ref::<LargeStringArray>().unwrap();
    assert_eq!(notes.value(0), "note");
    Ok(())
}

#[rstest]
#[case::utf8(DataType::Utf8)]
#[case::large_utf8(DataType::LargeUtf8)]
/// test NUMBER values can be fetched as strings while preserving nulls
fn test_3801(
    conn: oracledb::Connection,
    #[case] data_type: DataType,
) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        data_type.clone(),
        true,
    )]));
    let batch = conn
        .statement(
            r#"
            select value from (
                select 1 as id, cast(123.45 as number(5, 2)) as value
                from dual
                union all
                select 2, cast(null as number(5, 2))
                from dual
            ) order by id
            "#,
        )?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::default())?;
    assert_eq!(batch.schema(), schema);
    assert_eq!(batch.num_rows(), 2);
    match data_type {
        DataType::Utf8 => {
            let values = batch
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            assert_eq!(values.value(0), "123.45");
            assert!(values.is_null(1));
        }
        DataType::LargeUtf8 => {
            let values = batch
                .column(0)
                .as_any()
                .downcast_ref::<LargeStringArray>()
                .unwrap();
            assert_eq!(values.value(0), "123.45");
            assert!(values.is_null(1));
        }
        _ => unreachable!(),
    }
    Ok(())
}

#[rstest]
/// test NUMBER values can use requested unsigned and floating point types
fn test_3802(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("UINT", DataType::UInt64, true),
        Field::new("FLOAT", DataType::Float32, true),
        Field::new("DOUBLE", DataType::Float64, true),
    ]));
    let batch = conn
        .statement("select 42, 1.25, -2.5 from dual")?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::default())?;
    assert_eq!(batch.schema(), schema);
    assert_eq!(
        batch
            .column(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .unwrap()
            .value(0),
        42
    );
    assert_eq!(
        batch
            .column(1)
            .as_any()
            .downcast_ref::<Float32Array>()
            .unwrap()
            .value(0),
        1.25
    );
    assert_eq!(
        batch
            .column(2)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap()
            .value(0),
        -2.5
    );
    Ok(())
}

#[rstest]
#[case::positive("123.45", 6, 3, Some(123450))]
#[case::negative("-123.45", 6, 3, Some(-123450))]
#[case::zero_negative_scale("0", 4, -2, Some(0))]
#[case::negative_scale("9900", 4, -2, Some(99))]
#[case::no_scale_padding("123.45", 5, 2, Some(12345))]
#[case::maximum_precision(
    "99999999999999999999999999999999999999",
    38,
    0,
    Some(99_999_999_999_999_999_999_999_999_999_999_999_999i128)
)]
#[case::null("cast(null as number)", 6, 3, None)]
/// test requested Decimal128 precision, scale, coefficient, and null values
fn test_3803(
    conn: oracledb::Connection,
    #[case] value: &str,
    #[case] precision: u8,
    #[case] scale: i8,
    #[case] expected: Option<i128>,
) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Decimal128(precision, scale),
        true,
    )]));
    let sql = format!("select {value} from dual");
    let batch = conn
        .statement(&sql)?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::default())?;
    assert_eq!(batch.schema(), schema);
    let values =
        batch.column(0).as_any().downcast_ref::<Decimal128Array>().unwrap();
    match expected {
        Some(value) => assert_eq!(values.value(0), value),
        None => assert!(values.is_null(0)),
    }
    Ok(())
}

#[rstest]
#[case::binary(DataType::Binary)]
#[case::large_binary(DataType::LargeBinary)]
/// test RAW values can use either requested binary offset width
fn test_3804(
    conn: oracledb::Connection,
    #[case] data_type: DataType,
) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        data_type.clone(),
        true,
    )]));
    let batch = conn
        .statement("select hextoraw('00FF8001') from dual")?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::default())?;
    assert_eq!(batch.schema(), schema);
    let expected = &[0x00, 0xFF, 0x80, 0x01];
    match data_type {
        DataType::Binary => {
            let values = batch
                .column(0)
                .as_any()
                .downcast_ref::<BinaryArray>()
                .unwrap();
            assert_eq!(values.value(0), expected);
        }
        DataType::LargeBinary => {
            let values = batch
                .column(0)
                .as_any()
                .downcast_ref::<LargeBinaryArray>()
                .unwrap();
            assert_eq!(values.value(0), expected);
        }
        _ => unreachable!(),
    }
    Ok(())
}

#[rstest]
#[case::seconds(TimeUnit::Second)]
#[case::milliseconds(TimeUnit::Millisecond)]
#[case::microseconds(TimeUnit::Microsecond)]
#[case::nanoseconds(TimeUnit::Nanosecond)]
/// test the requested timestamp unit overrides the Oracle precision default
fn test_3805(
    conn: oracledb::Connection,
    #[case] unit: TimeUnit,
) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Timestamp(unit, None),
        true,
    )]));
    let batch = conn
        .statement(
            "select timestamp '2026-01-02 03:04:05.123456789' from dual",
        )?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::default())?;
    assert_eq!(batch.schema(), schema);
    let expected = chrono::NaiveDate::from_ymd_opt(2026, 1, 2)
        .unwrap()
        .and_hms_nano_opt(3, 4, 5, 123_456_789)
        .unwrap()
        .and_utc();
    match unit {
        TimeUnit::Second => assert_eq!(
            batch
                .column(0)
                .as_any()
                .downcast_ref::<TimestampSecondArray>()
                .unwrap()
                .value(0),
            expected.timestamp()
        ),
        TimeUnit::Millisecond => assert_eq!(
            batch
                .column(0)
                .as_any()
                .downcast_ref::<TimestampMillisecondArray>()
                .unwrap()
                .value(0),
            expected.timestamp_millis()
        ),
        TimeUnit::Microsecond => assert_eq!(
            batch
                .column(0)
                .as_any()
                .downcast_ref::<TimestampMicrosecondArray>()
                .unwrap()
                .value(0),
            expected.timestamp_micros()
        ),
        TimeUnit::Nanosecond => assert_eq!(
            batch
                .column(0)
                .as_any()
                .downcast_ref::<TimestampNanosecondArray>()
                .unwrap()
                .value(0),
            expected.timestamp_nanos_opt().unwrap()
        ),
    }
    Ok(())
}

#[rstest]
#[case::too_few(0)]
#[case::too_many(2)]
/// test requested schema column count mismatches return an exact error
fn test_3806(
    conn: oracledb::Connection,
    #[case] num_fields: usize,
) -> Result<(), oracledb::Error> {
    let fields: Vec<_> = (0..num_fields)
        .map(|i| Field::new(format!("VALUE_{i}"), DataType::Int64, true))
        .collect();
    let error = conn
        .statement("select 1 from dual")?
        .with_requested_schema(Arc::new(Schema::new(fields)))
        .build()?
        .query_arrow(oracledb::BindParameters::default())
        .unwrap_err();
    assert_eq!(
        error.kind(),
        &oracledb::ErrorKind::WrongArrowSchemaLength(num_fields, 1)
    );
    Ok(())
}

#[rstest]
#[case::number_to_binary("select 1 from dual", DataType::Binary)]
#[case::text_to_integer("select 'text' from dual", DataType::Int64)]
#[case::raw_to_string("select hextoraw('01') from dual", DataType::Utf8)]
#[case::unsupported_type("select 1 from dual", DataType::Float16)]
#[case::empty_result("select 1 from dual where 1 = 0", DataType::Binary)]
#[case::timestamp_to_binary(
    "select timestamp '2026-01-02 03:04:05' from dual",
    DataType::Binary
)]
#[case::binary_float_to_float64(
    "select cast(1.25 as binary_float) from dual",
    DataType::Float64
)]
#[case::binary_double_to_float32(
    "select cast(1.25 as binary_double) from dual",
    DataType::Float32
)]
#[case::unsupported_oracle_type(
    "select to_yminterval('P1Y') from dual",
    DataType::Utf8
)]
/// test incompatible and unsupported requested types return conversion errors
fn test_3807(
    conn: oracledb::Connection,
    #[case] sql: &str,
    #[case] data_type: DataType,
) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        data_type.clone(),
        true,
    )]));
    let error = conn
        .statement(sql)?
        .with_requested_schema(schema)
        .build()?
        .query_arrow(oracledb::BindParameters::default())
        .unwrap_err();
    match error.kind() {
        oracledb::ErrorKind::UnsupportedConversion(from, to) => {
            assert!(from.ends_with(" column VALUE"));
            assert_eq!(to, &data_type.to_string());
        }
        other => panic!("expected UnsupportedConversion, got {other:?}"),
    }
    Ok(())
}

#[rstest]
#[case::zero_precision(0, 0)]
#[case::excess_precision(39, 0)]
#[case::excess_scale(5, 6)]
/// test invalid requested Decimal128 definitions are rejected before fetching
fn test_3808(
    conn: oracledb::Connection,
    #[case] precision: u8,
    #[case] scale: i8,
) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Decimal128(precision, scale),
        true,
    )]));
    let error = conn
        .statement("select 1 from dual where 1 = 0")?
        .with_requested_schema(schema)
        .build()?
        .query_arrow(oracledb::BindParameters::default())
        .unwrap_err();
    assert_eq!(error.kind(), &oracledb::ErrorKind::ArrowOperation);
    Ok(())
}

#[rstest]
#[case::coefficient_precision(
    "123",
    3,
    2,
    oracledb::ErrorKind::ArrowOperation
)]
#[case::scale_factor_overflow(
    "1e38", 38, 1,
    oracledb::ErrorKind::CannotConvertToArrowDecimal(
        "100000000000000000000000000000000000000".into(), 38, 1
    )
)]
#[case::multiplication_overflow(
    "2e37", 38, 1,
    oracledb::ErrorKind::CannotConvertToArrowDecimal(
        "20000000000000000000000000000000000000".into(), 38, 1
    )
)]
#[case::negative_multiplication_overflow(
    "-2e37", 38, 1,
    oracledb::ErrorKind::CannotConvertToArrowDecimal(
        "-20000000000000000000000000000000000000".into(), 38, 1
    )
)]
#[case::power_overflow(
    "1e100", 38, 0,
    oracledb::ErrorKind::CannotConvertToArrowDecimal(
        "10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".into(), 38, 0
    )
)]
#[case::precision_loss(
    "1234", 3, 0,
    oracledb::ErrorKind::CannotConvertToArrowDecimal("1234".into(), 3, 0)
)]
#[case::scale_loss(
    "1.25", 4, 1,
    oracledb::ErrorKind::CannotConvertToArrowDecimal("1.25".into(), 4, 1)
)]
/// test values that exceed the requested Decimal128 precision return errors
fn test_3809(
    conn: oracledb::Connection,
    #[case] value: &str,
    #[case] precision: u8,
    #[case] scale: i8,
    #[case] expected: oracledb::ErrorKind,
) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Decimal128(precision, scale),
        true,
    )]));
    let sql = format!("select {value} from dual");
    let error = conn
        .statement(&sql)?
        .with_requested_schema(schema)
        .build()?
        .query_arrow(oracledb::BindParameters::default())
        .unwrap_err();
    assert_eq!(error.kind(), &expected);
    Ok(())
}

#[rstest]
/// test empty results retain the requested schema
fn test_3810(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Int64,
        true,
    )]));
    let batch = conn
        .statement("select cast(1 as number(4, 0)) from dual where 1 = 0")?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::default())?;
    assert_eq!(batch.schema(), schema);
    assert_eq!(batch.num_rows(), 0);
    Ok(())
}

#[rstest]
/// test a requested schema does not change subsequent default fetches
fn test_3811(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let sql = "select cast(42 as number(4, 0)) as VALUE from dual";
    let schema = Arc::new(Schema::new(vec![Field::new(
        "RENAMED",
        DataType::Int64,
        true,
    )]));
    let batch = conn
        .statement(sql)?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::default())?;
    assert_eq!(batch.schema(), schema);
    let default_batch =
        conn.query_arrow(sql, oracledb::BindParameters::default())?;
    assert_eq!(default_batch.schema().field(0).name(), "VALUE");
    assert_eq!(default_batch.schema().field(0).data_type(), &DataType::Int16);
    assert_eq!(
        default_batch
            .schema()
            .field(0)
            .metadata()
            .get("oracle.type-name")
            .unwrap(),
        "NUMBER(4)"
    );
    Ok(())
}

#[rstest]
/// test null results cannot use a non-nullable requested field
fn test_3812(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Int64,
        false,
    )]));
    let error = conn
        .statement("select cast(null as number) from dual")?
        .with_requested_schema(schema)
        .build()?
        .query_arrow(oracledb::BindParameters::default())
        .unwrap_err();
    assert_eq!(error.kind(), &oracledb::ErrorKind::ArrowOperation);
    Ok(())
}

#[rstest]
#[case::integer_overflow("128", DataType::Int8)]
#[case::negative_unsigned("-1", DataType::UInt64)]
#[case::fractional_integer("1.25", DataType::Int64)]
/// test NUMBER values must fit the requested integer type exactly
fn test_3813(
    conn: oracledb::Connection,
    #[case] value: &str,
    #[case] data_type: DataType,
) -> Result<(), oracledb::Error> {
    let schema =
        Arc::new(Schema::new(vec![Field::new("VALUE", data_type, true)]));
    let sql = format!("select {value} from dual");
    let error = conn
        .statement(&sql)?
        .with_requested_schema(schema)
        .build()?
        .query_arrow(oracledb::BindParameters::default())
        .unwrap_err();
    assert!(matches!(
        error.kind(),
        oracledb::ErrorKind::UnsupportedConversion(_, _)
    ));
    Ok(())
}

#[rstest]
#[case::int8(DataType::Int8)]
#[case::int16(DataType::Int16)]
#[case::int32(DataType::Int32)]
#[case::int64(DataType::Int64)]
#[case::uint8(DataType::UInt8)]
#[case::uint16(DataType::UInt16)]
#[case::uint32(DataType::UInt32)]
#[case::uint64(DataType::UInt64)]
/// test every supported requested integer type preserves the NUMBER value
fn test_3814(
    conn: oracledb::Connection,
    #[case] data_type: DataType,
) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        data_type.clone(),
        true,
    )]));
    let batch = conn
        .statement("select 42 from dual")?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::default())?;
    assert_eq!(batch.schema(), schema);
    assert_eq!(batch.num_rows(), 1);
    let values = batch.column(0);
    let value = match data_type {
        DataType::Int8 => {
            i128::from(values.as_primitive::<Int8Type>().value(0))
        }
        DataType::Int16 => {
            i128::from(values.as_primitive::<Int16Type>().value(0))
        }
        DataType::Int32 => {
            i128::from(values.as_primitive::<Int32Type>().value(0))
        }
        DataType::Int64 => {
            i128::from(values.as_primitive::<Int64Type>().value(0))
        }
        DataType::UInt8 => {
            i128::from(values.as_primitive::<UInt8Type>().value(0))
        }
        DataType::UInt16 => {
            i128::from(values.as_primitive::<UInt16Type>().value(0))
        }
        DataType::UInt32 => {
            i128::from(values.as_primitive::<UInt32Type>().value(0))
        }
        DataType::UInt64 => {
            i128::from(values.as_primitive::<UInt64Type>().value(0))
        }
        _ => unreachable!(),
    };
    assert_eq!(value, 42);
    Ok(())
}

#[rstest]
#[case::utf8(DataType::Utf8)]
#[case::large_utf8(DataType::LargeUtf8)]
/// test character columns support both requested string offset widths
fn test_3815(
    conn: oracledb::Connection,
    #[case] data_type: DataType,
) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        data_type.clone(),
        true,
    )]));
    let batch = conn
        .statement("select 'Arrow text' from dual")?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::default())?;
    assert_eq!(batch.schema(), schema);
    let value = match data_type {
        DataType::Utf8 => batch.column(0).as_string::<i32>().value(0),
        DataType::LargeUtf8 => batch.column(0).as_string::<i64>().value(0),
        _ => unreachable!(),
    };
    assert_eq!(value, "Arrow text");
    Ok(())
}

#[rstest]
/// test native Oracle floating point columns support their requested Arrow types
fn test_3816(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("FLOAT", DataType::Float32, true),
        Field::new("DOUBLE", DataType::Float64, true),
    ]));
    let batch = conn
        .statement(
            r#"
            select cast(1.25 as binary_float),
                   cast(-2.5 as binary_double) from dual
            "#,
        )?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::default())?;
    assert_eq!(batch.schema(), schema);
    assert_eq!(
        batch
            .column(0)
            .as_any()
            .downcast_ref::<Float32Array>()
            .unwrap()
            .value(0),
        1.25
    );
    assert_eq!(
        batch
            .column(1)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap()
            .value(0),
        -2.5
    );
    Ok(())
}

#[rstest]
#[case::boolean(DataType::Boolean)]
#[case::incompatible_integer(DataType::Int64)]
/// test native BOOLEAN columns accept Boolean and reject incompatible types
fn test_3817(
    conn: oracledb::Connection,
    #[case] data_type: DataType,
) -> Result<(), oracledb::Error> {
    if common::skip_unless_native_boolean_supported(&conn) {
        return Ok(());
    }
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        data_type.clone(),
        true,
    )]));
    let result = conn
        .statement("select true from dual")?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::default());
    if data_type == DataType::Boolean {
        let batch = result?;
        assert_eq!(batch.schema(), schema);
        let values =
            batch.column(0).as_any().downcast_ref::<BooleanArray>().unwrap();
        assert!(values.value(0));
    } else {
        let error = result.unwrap_err();
        assert_eq!(
            error.kind(),
            &oracledb::ErrorKind::UnsupportedConversion(
                "BOOLEAN column VALUE".into(),
                "Int64".into(),
            )
        );
    }
    Ok(())
}

#[rstest]
/// test NUMBER to String conversion supports borrowed and owned row values
fn test_3818(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let mut row = conn.query_row("select 123.45 from dual", &[])?;
    assert_eq!(row.get::<String>(0)?, "123.45");
    assert_eq!(row.take::<String>(0)?, "123.45");
    Ok(())
}

#[rstest]
/// test the requested schema applies to every set of query bind parameters
fn test_3819(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Int64,
        true,
    )]));
    let batch = conn
        .statement("select :1 from dual")?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::Slice(&[
            &[&1_i32],
            &[&2_i32],
        ]))?;
    assert_eq!(batch.schema(), schema);
    assert_eq!(batch.num_rows(), 2);
    let values =
        batch.column(0).as_any().downcast_ref::<Int64Array>().unwrap();
    assert_eq!(values.value(0), 1);
    assert_eq!(values.value(1), 2);
    Ok(())
}

#[rstest]
#[case::number("test_3820_number", "number")]
#[case::float("test_3820_float", "float(126)")]
/// test requesting Utf8 preserves exact NUMBER values from GitHub issue 53
fn test_3820(
    conn: oracledb::Connection,
    #[case] table_name: &str,
    #[case] column_type: &str,
) -> Result<(), oracledb::Error> {
    let _guard = common::create_table(
        &conn,
        table_name,
        &format!("id number(4) primary key, value {column_type}"),
    )?;
    let inputs = [
        "9007199254740993",
        "12345678901234567890123456789012345678",
        "-1e-36",
        "null",
    ];
    for (index, value) in inputs.iter().enumerate() {
        conn.execute(
            &format!("insert into {table_name} values ({index}, {value})"),
            &[],
        )?;
    }
    let schema =
        Arc::new(Schema::new(vec![Field::new("VALUE", DataType::Utf8, true)]));
    let sql = format!("select value from {table_name} order by id");
    let batch = conn
        .statement(&sql)?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::default())?;
    assert_eq!(batch.schema(), schema);
    assert_eq!(batch.num_rows(), inputs.len());
    let values =
        batch.column(0).as_any().downcast_ref::<StringArray>().unwrap();
    assert_eq!(values.value(0), "9007199254740993");
    assert_eq!(values.value(1), "12345678901234567890123456789012345678");
    assert_eq!(values.value(2), format!("-0.{}1", "0".repeat(35)));
    assert!(values.is_null(3));
    Ok(())
}

#[rstest]
#[case::small_decimal("12.25", 4, 2, "1225".into())]
#[case::integer_beyond_float(
    "9007199254740993", 76, 38, format!("9007199254740993{}", "0".repeat(38))
)]
#[case::large_integer(
    "22222222222222222222222222222222222222", 76, 38,
    format!("22222222222222222222222222222222222222{}", "0".repeat(38))
)]
#[case::positive_fraction("123.45", 76, 38, format!("12345{}", "0".repeat(36)))]
#[case::large_fraction(
    "88888888888888888888.888888888888888888", 76, 38,
    format!("88888888888888888888888888888888888888{}", "0".repeat(20))
)]
#[case::fraction_below_one(
    "0.99999999999999999999999999999999999999", 76, 38,
    "99999999999999999999999999999999999999".into()
)]
#[case::fraction_above_one(
    "1.9876543210987654321098765432109876543", 76, 38,
    "198765432109876543210987654321098765430".into()
)]
#[case::large_integer_with_fraction(
    "9876543210987654321098765432109876543.7", 76, 38,
    format!("98765432109876543210987654321098765437{}", "0".repeat(37))
)]
#[case::negative_fraction("-0.0000000000000000000000000000000000001", 76, 38, "-10".into())]
#[case::smallest_positive_fraction("1e-76", 76, 76, "1".into())]
#[case::smallest_negative_fraction("-1e-76", 76, 76, "-1".into())]
#[case::negative_scale("9900", 4, -2, "99".into())]
#[case::negative_value_and_scale("-9900", 4, -2, "-99".into())]
#[case::large_negative_scale("1e76", 76, -76, "1".into())]
#[case::large_coefficient("1e75", 76, 0, format!("1{}", "0".repeat(75)))]
#[case::zero_negative_scale("0", 1, -128, "0".into())]
#[case::zero_positive_scale("0", 76, 76, "0".into())]
/// test requested Decimal256 fetches preserve the exact coefficient, scale, and nulls
fn test_3821(
    conn: oracledb::Connection,
    #[case] value: &str,
    #[case] precision: u8,
    #[case] scale: i8,
    #[case] expected: String,
) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Decimal256(precision, scale),
        true,
    )]));
    let sql = r#"
        select value from (
            select 1 as id, to_number(:1) as value from dual
            union all
            select 2, cast(null as number) from dual
        ) order by id
        "#;
    let batch = conn
        .statement(sql)?
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::Slice(&[&[&value]]))?;
    assert_eq!(batch.schema(), schema);
    assert_eq!(batch.num_rows(), 2);
    let values =
        batch.column(0).as_any().downcast_ref::<Decimal256Array>().unwrap();
    assert_eq!(values.value(0).to_string(), expected);
    assert!(values.is_null(1));
    Ok(())
}

#[rstest]
#[case::precision_loss("1234", 3, 0)]
#[case::scale_loss("1.25", 4, 1)]
#[case::power_overflow("1e100", 76, 0)]
#[case::multiplication_overflow("9e75", 76, 1)]
#[case::negative_multiplication_overflow("-9e75", 76, 1)]
/// test Decimal256 fetch conversion failures report the source value and target type
fn test_3822(
    conn: oracledb::Connection,
    #[case] value: &str,
    #[case] precision: u8,
    #[case] scale: i8,
) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Decimal256(precision, scale),
        true,
    )]));
    let sql = format!("select {value} from dual");
    let expected = conn.query_row(&sql, &[])?.get::<String>(0)?;
    let error = conn
        .statement(&sql)?
        .with_requested_schema(schema)
        .build()?
        .query_arrow(oracledb::BindParameters::default())
        .unwrap_err();
    assert_eq!(
        error.kind(),
        &oracledb::ErrorKind::CannotConvertToArrowDecimal(
            expected.clone(),
            precision,
            scale,
        )
    );
    assert_eq!(
        error.to_string(),
        format!(
            "{expected} cannot be converted to an Apache Arrow decimal({precision}, {scale})"
        )
    );
    Ok(())
}

#[rstest]
#[case::zero_precision(0, 0)]
#[case::excess_precision(77, 0)]
#[case::excess_scale(76, 77)]
#[case::scale_exceeds_precision(4, 5)]
/// test invalid Decimal256 schema definitions are rejected even for empty results
fn test_3823(
    conn: oracledb::Connection,
    #[case] precision: u8,
    #[case] scale: i8,
) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Decimal256(precision, scale),
        true,
    )]));
    let error = conn
        .statement("select 1 from dual where 1 = 0")?
        .with_requested_schema(schema)
        .build()?
        .query_arrow(oracledb::BindParameters::default())
        .unwrap_err();
    assert_eq!(error.kind(), &oracledb::ErrorKind::ArrowOperation);
    Ok(())
}

#[rstest]
#[case::padded_coefficient("123", 3, 2)]
#[case::coefficient_exceeds_76_digits("1e76", 76, 0)]
/// test a fetched Decimal256 coefficient must fit the requested precision
fn test_3824(
    conn: oracledb::Connection,
    #[case] value: &str,
    #[case] precision: u8,
    #[case] scale: i8,
) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Decimal256(precision, scale),
        true,
    )]));
    let sql = format!("select {value} from dual");
    let error = conn
        .statement(&sql)?
        .with_requested_schema(schema)
        .build()?
        .query_arrow(oracledb::BindParameters::default())
        .unwrap_err();
    assert_eq!(error.kind(), &oracledb::ErrorKind::ArrowOperation);
    Ok(())
}

#[rstest]
/// test a character column cannot be fetched as Decimal256
fn test_3825(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Decimal256(76, 38),
        true,
    )]));
    let error = conn
        .statement("select 'text' from dual")?
        .with_requested_schema(schema)
        .build()?
        .query_arrow(oracledb::BindParameters::default())
        .unwrap_err();
    assert!(matches!(
        error.kind(),
        oracledb::ErrorKind::UnsupportedConversion(_, _)
    ));
    Ok(())
}

#[rstest]
/// test duplicate Decimal256 values are preserved across database fetches
fn test_3826(conn: oracledb::Connection) -> Result<(), oracledb::Error> {
    let schema = Arc::new(Schema::new(vec![Field::new(
        "VALUE",
        DataType::Decimal256(4, 2),
        true,
    )]));
    let value = "12.25";
    let batch = conn
        .statement(
            r#"
            select to_number(:1) as value from dual
            connect by level <= 6
            "#,
        )?
        .fetch_array_size(3)
        .prefetch_rows(0)
        .with_requested_schema(schema.clone())
        .build()?
        .query_arrow(oracledb::BindParameters::Slice(&[&[&value]]))?;
    assert_eq!(batch.schema(), schema);
    assert_eq!(batch.num_rows(), 6);
    let values =
        batch.column(0).as_any().downcast_ref::<Decimal256Array>().unwrap();
    for row in 0..values.len() {
        assert!(!values.is_null(row));
        assert_eq!(values.value(row).to_string(), "1225");
    }
    Ok(())
}
