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
// interval_ym.rs
//
// Defines the structure representing Oracle years to months intervals.
//-----------------------------------------------------------------------------

use std::fmt;

use crate::constants;
use crate::db_type::DbType;
use crate::read_buffer::FromBuf;
use crate::write_buffer::ToBuf;
use crate::write_buffer::WriteBuffer;

/// Represents the value of INTERVAL YEAR TO MONTH columns in the database.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OracleIntervalYM {
    years: i32,
    months: i8,
}

impl OracleIntervalYM {
    /// Returns the number of months in the interval.
    pub fn months(&self) -> i8 {
        self.months
    }

    /// Creates a new interval and returns it.
    pub fn new(years: i32, months: i8) -> Self {
        Self { years, months }
    }

    /// Returns the number of years in the interval.
    pub fn years(&self) -> i32 {
        self.years
    }
}

impl fmt::Display for OracleIntervalYM {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "P{}Y{}M", self.years, self.months)
    }
}

impl FromBuf for OracleIntervalYM {
    fn from_buf(buf: &[u8]) -> Self {
        let years = u32::from_be_bytes(buf[0..4].try_into().unwrap());
        Self {
            years: years.wrapping_sub(constants::DURATION_MID) as i32,
            months: buf[4].wrapping_sub(constants::DURATION_OFFSET) as i8,
        }
    }
}

impl ToBuf for OracleIntervalYM {
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
            (self.years() as u32).wrapping_add(constants::DURATION_MID),
        );
        buf.write_u8(
            (self.months() as u8).wrapping_add(constants::DURATION_OFFSET),
        );
    }
}
