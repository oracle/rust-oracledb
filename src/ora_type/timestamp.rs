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
// timestamp.rs
//
// Defines the structure representing Oracle timestamps.
//-----------------------------------------------------------------------------

use chrono::Datelike;
use chrono::Timelike;
use std::fmt;
use std::fmt::Write;

#[cfg(feature = "arrow")]
use arrow_schema::TimeUnit;

use crate::constants;
use crate::db_type::DbType;
use crate::error::Error;
use crate::read_buffer::FromBuf;
use crate::write_buffer::ToBuf;
use crate::write_buffer::WriteBuffer;

/// Represents the value of DATE, TIMESTAMP, TIMESTAMP WITH LOCAL TIME ZONE and
/// TIMESTAMP WITH TIME ZONE columns in the database.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OracleTimestamp {
    year: i16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    nanoseconds: u32,
    is_utc: bool,
}

impl Default for OracleTimestamp {
    fn default() -> Self {
        Self {
            year: 1970,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            nanoseconds: 0,
            is_utc: true,
        }
    }
}

impl OracleTimestamp {
    #[cfg(feature = "arrow")]
    /// Returns the timestamp in the specified Arrow time unit.
    pub(crate) fn to_timestamp(
        self,
        time_unit: TimeUnit,
    ) -> Result<i64, Error> {
        let timestamp = chrono::NaiveDateTime::try_from(self)?.and_utc();
        match time_unit {
            TimeUnit::Second => Ok(timestamp.timestamp()),
            TimeUnit::Millisecond => Ok(timestamp.timestamp_millis()),
            TimeUnit::Microsecond => Ok(timestamp.timestamp_micros()),
            TimeUnit::Nanosecond => {
                timestamp.timestamp_nanos_opt().ok_or_else(|| {
                    Error::out_of_range("timestamp is outside Arrow range")
                })
            }
        }
    }

    /// Returns the day of the timestamp.
    pub fn day(&self) -> u8 {
        self.day
    }

    /// Returns the hour of the timestamp.
    pub fn hour(&self) -> u8 {
        self.hour
    }

    /// Returns whether the timestamp is a UTC timestamp or not.
    pub fn is_utc(&self) -> bool {
        self.is_utc
    }

    /// Returns the minute of the timestamp.
    pub fn minute(&self) -> u8 {
        self.minute
    }

    /// Returns the month of the timestamp.
    pub fn month(&self) -> u8 {
        self.month
    }

    /// Returns the nanoseconds of the timestamp.
    pub fn nanoseconds(&self) -> u32 {
        self.nanoseconds
    }

    /// Creates a new timestamp given only the date. The time portion is set
    /// to be midnight.
    pub fn new_date(year: i16, month: u8, day: u8) -> Self {
        Self {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
            second: 0,
            nanoseconds: 0,
            is_utc: false,
        }
    }

    /// Creates a new naive timestamp given the date and time.
    pub fn new_timestamp(
        year: i16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        nanoseconds: u32,
    ) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanoseconds,
            is_utc: false,
        }
    }

    /// Creates a new UTC timestamp given the date and time.
    pub fn new_utc_timestamp(
        year: i16,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        nanoseconds: u32,
    ) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            nanoseconds,
            is_utc: true,
        }
    }

    /// Returns the second of the timestamp.
    pub fn second(&self) -> u8 {
        self.second
    }

    /// Returns the year of the timestamp.
    pub fn year(&self) -> i16 {
        self.year
    }
}

impl fmt::Display for OracleTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}",
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
            self.nanoseconds
        )?;
        if self.is_utc {
            f.write_char('Z')?;
        }
        Ok(())
    }
}

impl FromBuf for OracleTimestamp {
    fn from_buf(buf: &[u8]) -> Self {
        let mut nanoseconds: u32 = 0;
        if buf.len() >= constants::ORA_TYPE_SIZE_TIMESTAMP {
            nanoseconds = u32::from_be_bytes(buf[7..11].try_into().unwrap());
        }
        let is_utc = buf.len() == constants::ORA_TYPE_SIZE_TIMESTAMP_TZ;
        Self {
            year: ((buf[0] as i16 - 100) * 100 + buf[1] as i16 - 100),
            month: buf[2],
            day: buf[3],
            hour: buf[4] - 1,
            minute: buf[5] - 1,
            second: buf[6] - 1,
            nanoseconds,
            is_utc,
        }
    }
}

impl ToBuf for OracleTimestamp {
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        db_type: &'static DbType,
        write_length: bool,
    ) {
        let mut actual_db_type = db_type;
        if db_type == crate::DB_TYPE_TIMESTAMP && self.nanoseconds() == 0 {
            actual_db_type = crate::DB_TYPE_DATE;
        }
        if write_length {
            buf.write_u8(
                actual_db_type.buffer_size_factor.try_into().unwrap(),
            );
        }
        buf.write_u8((self.year() / 100 + 100).try_into().unwrap());
        buf.write_u8((self.year() % 100 + 100).try_into().unwrap());
        buf.write_u8(self.month());
        buf.write_u8(self.day());
        buf.write_u8(self.hour() + 1);
        buf.write_u8(self.minute() + 1);
        buf.write_u8(self.second() + 1);
        if actual_db_type != crate::DB_TYPE_DATE {
            buf.write_u32be(self.nanoseconds());
        } else if actual_db_type == crate::DB_TYPE_TIMESTAMP_TZ {
            buf.write_u8(constants::TZ_HOUR_OFFSET);
            buf.write_u8(constants::TZ_MINUTE_OFFSET);
        }
    }
}

impl From<chrono::NaiveDateTime> for OracleTimestamp {
    fn from(value: chrono::NaiveDateTime) -> Self {
        Self {
            year: value.year() as i16,
            month: value.month() as u8,
            day: value.day() as u8,
            hour: value.hour() as u8,
            minute: value.minute() as u8,
            second: value.second() as u8,
            nanoseconds: value.nanosecond(),
            is_utc: false,
        }
    }
}

impl TryFrom<OracleTimestamp> for chrono::NaiveDateTime {
    type Error = Error;
    fn try_from(ts: OracleTimestamp) -> Result<Self, Self::Error> {
        chrono::NaiveDate::from_ymd_opt(
            ts.year().into(),
            ts.month().into(),
            ts.day().into(),
        )
        .ok_or_else(|| {
            Error::out_of_range(format!(
                "invalid year-month-day: {}-{}-{}",
                ts.year(),
                ts.month(),
                ts.day()
            ))
        })?
        .and_hms_nano_opt(
            ts.hour().into(),
            ts.minute().into(),
            ts.second().into(),
            ts.nanoseconds(),
        )
        .ok_or_else(|| {
            Error::out_of_range(format!(
                "invalid timestamp: {}-{}-{} {}:{}:{}.{:09}",
                ts.year(),
                ts.month(),
                ts.day(),
                ts.hour(),
                ts.minute(),
                ts.second(),
                ts.nanoseconds()
            ))
        })
    }
}
