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
// number.rs
//
// Defines the structure representing Oracle numbers.
//-----------------------------------------------------------------------------

use std::fmt;
use std::fmt::Write;
use std::str::FromStr;

use crate::constants;
use crate::db_type::DbType;
use crate::error::Error;
use crate::read_buffer::FromBuf;
use crate::write_buffer::ToBuf;
use crate::write_buffer::WriteBuffer;

/// Represents the value of NUMBER columns in the database.
#[derive(Clone, Copy)]
pub struct OracleNumber {
    is_positive: bool,
    decimal_point_index: i16,
    num_digits: u8,
    digits: [u8; constants::ORA_NUM_MAX_DIGITS],
}

impl OracleNumber {
    /// Converts an OracleNumber to the i128 value expected for Arrow
    /// Decimal128 that fits within the specified precision and scale.
    #[cfg(feature = "arrow")]
    pub(crate) fn as_decimal128(
        &self,
        precision: u8,
        scale: i8,
    ) -> Result<i128, Error> {
        // the precision of the number cannot exceed the requested precision
        if self.num_digits > precision {
            return Err(Error::out_of_range(format!(
                "NUMBER has precision {} but requested precision is {}",
                self.num_digits, precision
            )));
        }

        // the scale of the number cannot exceed the requested scale
        let num_scale = i16::from(self.num_digits) - self.decimal_point_index;
        let num_zeros_to_add = i16::from(scale) - num_scale;
        if num_zeros_to_add < 0 {
            return Err(Error::out_of_range(format!(
                "NUMBER has scale {} but requested scale is {}",
                num_scale, scale
            )));
        }

        // add each of the digits to the result
        let mut result: i128 = 0;
        for i in 0..self.num_digits {
            result = result.wrapping_mul(10);
            result = result.wrapping_add(self.digits[i as usize].into());
        }

        // add any trailing zeros that are required
        if num_zeros_to_add > 0 {
            result *= 10_i128.pow(num_zeros_to_add.try_into().unwrap());
        }

        if self.is_positive {
            Ok(result)
        } else {
            Ok(-result)
        }
    }
}

impl Default for OracleNumber {
    fn default() -> Self {
        Self {
            is_positive: true,
            decimal_point_index: 0,
            num_digits: 0,
            digits: [0u8; constants::ORA_NUM_MAX_DIGITS],
        }
    }
}

impl fmt::Debug for OracleNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for OracleNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.is_positive {
            f.write_char('-')?;
        }

        // if the decimal point index is 0 or less, add the decimal point and
        // any leading zeroes that are needed; take care of the special case
        // when the value is zero
        if self.decimal_point_index <= 0 {
            f.write_char('0')?;
            if self.num_digits > 0 {
                f.write_char('.')?;
            }
            for _ in self.decimal_point_index..0 {
                f.write_char('0')?;
            }
        }

        // add each of the digits, adding the decimal point where appropriate
        for i in 0..self.num_digits {
            if i > 0 && i16::from(i) == self.decimal_point_index {
                f.write_char('.')?;
            }
            let digit: u32 = self.digits[i as usize] as u32;
            f.write_char(char::from_digit(digit, 10).unwrap())?;
        }

        // if the decimal point index exceeds the number of digits, add any
        // trailing zeroes that are needed
        for _ in i16::from(self.num_digits)..self.decimal_point_index {
            f.write_char('0')?;
        }

        Ok(())
    }
}

impl FromBuf for OracleNumber {
    fn from_buf(buf: &[u8]) -> Self {
        // the first byte is the exponent; positive numbers have the highest
        // order bit set, whereas negative numbers have the highest order bit
        // cleared and the bits inverted
        let is_positive = buf[0] & 0x80 != 0;
        let mut exponent_byte = buf[0];
        if !is_positive {
            exponent_byte = !exponent_byte;
        }
        exponent_byte = exponent_byte.wrapping_sub(193);
        let exponent: i16 = (exponent_byte as i8).into();
        let mut decimal_point_index = exponent * 2 + 2;

        // the remaining bytes are mantissa digits; check for a trailing 102
        // byte for negative numbers and, if present, reduce the number of
        // mantissa digits
        let mut num_mantissa_digits = buf.len() - 1;
        if !is_positive && buf[num_mantissa_digits] == 102 {
            num_mantissa_digits -= 1;
        }

        // a mantissa length of 0 implies a value of 0 (positive) or a value of
        // -1e126 (if negative)
        if num_mantissa_digits == 0 && is_positive {
            decimal_point_index = 0;
        }

        // process each of the mantissa bytes; each mantissa byte is a
        // base-100 digit
        let mut num_digits = 0;
        let mut digits = [0u8; constants::ORA_NUM_MAX_DIGITS];
        for i in 0..num_mantissa_digits {
            // positive numbers have 1 added to them whereas negative numbers
            // are subtracted from the value 101
            let mantissa_byte = if is_positive {
                buf[i + 1] - 1
            } else {
                101 - buf[i + 1]
            };

            // process the first digit; leading zeroes are ignored
            let mut digit = mantissa_byte / 10;
            if digit == 0 && num_digits == 0 {
                decimal_point_index -= 1;
            } else if digit == 10 {
                digits[num_digits] = 1;
                digits[num_digits + 1] = 0;
                num_digits += 2;
            } else if digit != 0 || i > 0 {
                digits[num_digits] = digit;
                num_digits += 1;
            }

            // process the second digit; trailing zeros are ignored
            digit = mantissa_byte % 10;
            if digit != 0 || i < num_mantissa_digits - 1 {
                digits[num_digits] = digit;
                num_digits += 1;
            }
        }

        Self {
            is_positive,
            decimal_point_index,
            num_digits: num_digits.try_into().unwrap(),
            digits,
        }
    }
}

impl FromStr for OracleNumber {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut digits = [0u8; constants::ORA_NUM_MAX_DIGITS];
        let mut is_positive = true;
        let mut num_digits = 0;
        let mut decimal_point_index: i16 = 0;
        let mut first_char = true;
        let mut num_zeros = 0;
        let mut decimal_point_detected = false;
        for ch in s.chars() {
            if first_char {
                first_char = false;
                if ch == '-' {
                    is_positive = false;
                    continue;
                } else if ch == '+' {
                    continue;
                }
            }
            if ch == '.' && !decimal_point_detected {
                decimal_point_detected = true;
                if num_digits > 0 {
                    num_digits += num_zeros;
                    decimal_point_index = num_digits.try_into().unwrap();
                }
                num_zeros = 0;
            } else if let Some(digit) = ch.to_digit(10) {
                if digit == 0 {
                    num_zeros += 1;
                } else {
                    if num_digits > 0 {
                        if num_digits + num_zeros + 1 > digits.len() {
                            return Err(Error::invalid_oracle_number(
                                s.into(),
                            ));
                        }
                        num_digits += num_zeros;
                    } else if decimal_point_detected {
                        decimal_point_index =
                            (0 - (num_zeros as isize)).try_into().unwrap();
                    }
                    num_zeros = 0;
                    digits[num_digits] = digit.try_into().unwrap();
                    num_digits += 1;
                }
            } else {
                return Err(Error::invalid_oracle_number(s.into()));
            }
        }
        if num_digits == 0 && num_zeros == 0 {
            Err(Error::invalid_oracle_number(s.into()))
        } else {
            if num_digits == 0 {
                decimal_point_index = 0;
            } else if !decimal_point_detected {
                num_digits += num_zeros;
                decimal_point_index = num_digits.try_into().unwrap();
            }
            Ok(Self {
                is_positive,
                decimal_point_index,
                num_digits: num_digits.try_into().unwrap(),
                digits,
            })
        }
    }
}

impl ToBuf for OracleNumber {
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        _db_type: &'static DbType,
        write_length: bool,
    ) {
        // when the number of digits is zero, the value is itself zero since
        // all leading and trailing zeros are removed; this is a special case
        if self.num_digits == 0 {
            buf.write_u8(1);
            buf.write_u8(128);
            return;
        }

        // each pair of digits is encoded as a base-100 digit; so if the number
        // of digits is not even or the decimal point index is not even, a
        // zero needs to be prepended or appended to the digits
        let mut num_digits: usize = self.num_digits.into();
        let mut prepend_zero = false;
        let mut decimal_point_index = self.decimal_point_index;
        if decimal_point_index % 2 == 1 {
            prepend_zero = true;
            num_digits += 1;
            decimal_point_index += 1;
        }
        if num_digits % 2 == 1 {
            num_digits += 1;
        }

        // write length, if applicable
        let num_pairs = num_digits / 2;
        let append_sentinel =
            !self.is_positive && num_pairs < constants::ORA_NUM_MAX_DIGITS / 2;
        if write_length {
            let mut num_len = num_pairs + 1;
            if append_sentinel {
                num_len += 1;
            }
            buf.write_u8(num_len.try_into().unwrap());
        }

        // write exponent
        let mut exponent: u8 =
            ((decimal_point_index / 2) + 192).try_into().unwrap();
        if !self.is_positive {
            exponent = !exponent;
        }
        buf.write_u8(exponent);

        // write digit pairs
        let mut digits_pos = 0;
        for i in 0..num_pairs {
            let mut digit = self.digits[digits_pos];
            if i == 0 && prepend_zero {
                digits_pos += 1;
            } else {
                digit = digit * 10 + self.digits[digits_pos + 1];
                digits_pos += 2;
            }
            if self.is_positive {
                buf.write_u8(digit + 1);
            } else {
                buf.write_u8(101 - digit);
            }
        }
        if append_sentinel {
            buf.write_u8(102);
        }
    }
}
