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
// interval_ds.rs
//
// Defines the structure representing Oracle days to seconds intervals.
//-----------------------------------------------------------------------------

use std::fmt;

use crate::constants;
use crate::db_type::DbType;
use crate::read_buffer::FromBuf;
use crate::write_buffer::ToBuf;
use crate::write_buffer::WriteBuffer;

/// Represents the value of INTERVAL DAY TO SECOND columns in the database.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OracleIntervalDS {
    days: i32,
    hours: i8,
    minutes: i8,
    seconds: i8,
    nanoseconds: i32,
}

impl OracleIntervalDS {
    /// Returns the number of days in the interval.
    pub fn days(&self) -> i32 {
        self.days
    }

    /// Returns the number of hours in the interval.
    pub fn hours(&self) -> i8 {
        self.hours
    }

    /// Returns the number of minutes in the interval.
    pub fn minutes(&self) -> i8 {
        self.minutes
    }

    /// Returns the number of nanoseconds in the interval.
    pub fn nanoseconds(&self) -> i32 {
        self.nanoseconds
    }

    /// Creates a new interval and returns it.
    pub fn new(
        days: i32,
        hours: i8,
        minutes: i8,
        seconds: i8,
        nanoseconds: i32,
    ) -> Self {
        Self {
            days,
            hours,
            minutes,
            seconds,
            nanoseconds,
        }
    }

    /// Returns the number of seconds in the interval.
    pub fn seconds(&self) -> i8 {
        self.seconds
    }
}

impl fmt::Display for OracleIntervalDS {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "P{}DT{}H{}M{}.{:09}S",
            self.days,
            self.hours,
            self.minutes,
            self.seconds,
            self.nanoseconds
        )
    }
}

impl FromBuf for OracleIntervalDS {
    fn from_buf(buf: &[u8]) -> Self {
        let days = u32::from_be_bytes(buf[0..4].try_into().unwrap());
        let nanoseconds = u32::from_be_bytes(buf[7..11].try_into().unwrap());
        Self {
            days: days.wrapping_sub(constants::DURATION_MID) as i32,
            hours: buf[4].wrapping_sub(constants::DURATION_OFFSET) as i8,
            minutes: buf[5].wrapping_sub(constants::DURATION_OFFSET) as i8,
            seconds: buf[6].wrapping_sub(constants::DURATION_OFFSET) as i8,
            nanoseconds: nanoseconds.wrapping_sub(constants::DURATION_MID)
                as i32,
        }
    }
}

impl ToBuf for OracleIntervalDS {
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        db_type: &'static DbType,
        write_length: bool,
    ) {
        if write_length {
            buf.write_u8(db_type.buffer_size_factor.try_into().unwrap());
        }
        buf.write_u32be(
            (self.days() as u32).wrapping_add(constants::DURATION_MID),
        );
        buf.write_u8(
            (self.hours() as u8).wrapping_add(constants::DURATION_OFFSET),
        );
        buf.write_u8(
            (self.minutes() as u8).wrapping_add(constants::DURATION_OFFSET),
        );
        buf.write_u8(
            (self.seconds() as u8).wrapping_add(constants::DURATION_OFFSET),
        );
        buf.write_u32be(
            (self.nanoseconds() as u32).wrapping_add(constants::DURATION_MID),
        );
    }
}
