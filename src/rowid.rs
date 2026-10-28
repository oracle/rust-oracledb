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
// rowid.rs
//
// Defines the structure representing a rowid.
//-----------------------------------------------------------------------------

use std::fmt;

use crate::constants;
use crate::error::Error;
use crate::read_buffer::ReadBuffer;
use crate::response::Response;

#[derive(Clone, Debug)]
pub struct Rowid {
    rba: u32,
    partition_id: u16,
    block_num: u32,
    slot_num: u16,
}

impl Rowid {
    /// Deserializes a rowid from a database response.
    pub(crate) fn deserialize(resp: &mut Response) -> Result<Self, Error> {
        let rba = resp.read_ub4()?;
        let partition_id = resp.read_ub2()?;
        resp.advance(1)?;
        let block_num = resp.read_ub4()?;
        let slot_num = resp.read_ub2()?;
        Ok(Self { rba, partition_id, block_num, slot_num })
    }

    /// Returns a rowid from an encoded buffer.
    pub(crate) fn from_buf(buf: &mut ReadBuffer) -> Result<Self, Error> {
        let rba = buf.read_u32be()?;
        let partition_id = buf.read_u16be()?;
        let block_num = buf.read_u32be()?;
        let slot_num = buf.read_u16be()?;
        Ok(Self { rba, partition_id, block_num, slot_num })
    }
}

impl fmt::Display for Rowid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut output = [0u8; constants::ORA_TYPE_SIZE_ROWID];
        convert_base64(&mut output[0..6], self.rba.try_into().unwrap());
        convert_base64(&mut output[6..9], self.partition_id.into());
        convert_base64(&mut output[9..15], self.block_num.try_into().unwrap());
        convert_base64(&mut output[15..18], self.slot_num.into());
        f.write_str(std::str::from_utf8(&output).unwrap())
    }
}

// base64 alphabet used for rowid conversions
const BASE64_CHARS: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Converts every six bits into a character from left to right. This is
/// similar to ordinary base64 encoding with a few differences.
fn convert_base64(output: &mut [u8], value: usize) {
    let mut value = value;
    for i in (0..output.len()).rev() {
        let char_index: usize = value & 0x3f_usize;
        output[i] = BASE64_CHARS[char_index];
        value >>= 6;
    }
}

/// Converts a series of bytes making up a logical rowid into a base64 string.
pub(crate) fn convert_logical_rowid(input: &[u8]) -> String {
    let mut output_len: usize = (input.len() / 3) * 4 + 1;
    match input.len() % 3 {
        1 => output_len += 2,
        2 => output_len += 3,
        _ => {}
    };
    let mut output: Vec<u8> = vec![0u8; output_len];
    output[0] = b'*';
    let mut input_offset: usize = 0;
    let mut output_offset: usize = 1;
    while input_offset < input.len() {
        let (num_input_bytes, output_buf) = match input.len() - input_offset {
            1 => (1, &mut output[output_offset..output_offset + 2]),
            2 => (2, &mut output[output_offset..output_offset + 3]),
            _ => (3, &mut output[output_offset..output_offset + 4]),
        };
        let mut value: usize = 0;
        for i in 0..num_input_bytes {
            value <<= 8;
            value |= input[input_offset + i] as usize;
        }
        match output_buf.len() {
            2 => value <<= 4,
            3 => value <<= 2,
            _ => {}
        };
        convert_base64(output_buf, value);
        input_offset += num_input_bytes;
        output_offset += output_buf.len();
    }
    String::from_utf8(output).unwrap()
}
