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
// json.rs
//
// Defines the structures and enumerations for Oracle JSON values.
//-----------------------------------------------------------------------------

use std::collections::HashMap;
use std::collections::HashSet;

use crate::constants;
use crate::db_type::DB_TYPE_BINARY_DOUBLE;
use crate::db_type::DB_TYPE_BINARY_FLOAT;
use crate::db_type::DB_TYPE_DATE;
use crate::db_type::DB_TYPE_INTERVAL_DS;
use crate::db_type::DB_TYPE_INTERVAL_YM;
use crate::db_type::DB_TYPE_NUMBER;
use crate::db_type::DB_TYPE_TIMESTAMP;
use crate::db_type::DbType;
use crate::error::Error;
use crate::ora_type::OracleIntervalDS;
use crate::ora_type::OracleIntervalYM;
use crate::ora_type::OracleNumber;
use crate::ora_type::OracleTimestamp;
use crate::read_buffer::FromBuf;
use crate::read_buffer::FromBufFallible;
use crate::read_buffer::ReadBuffer;
use crate::vector::Vector;
use crate::write_buffer::ToBuf;
use crate::write_buffer::WriteBuffer;

// define OSON flags
const OSON_FLAG_FNAMES_SEG_UINT32: u16 = 0x0800;
const OSON_FLAG_HASH_ID_UINT8: u16 = 0x0100;
const OSON_FLAG_INLINE_LEAF: u16 = 0x0002;
const OSON_FLAG_IS_SCALAR: u16 = 0x0010;
const OSON_FLAG_NUM_FNAMES_UINT16: u16 = 0x0400;
const OSON_FLAG_NUM_FNAMES_UINT32: u16 = 0x0008;
const OSON_FLAG_REL_OFFSET_MODE: u16 = 0x0001;
const OSON_FLAG_SEC_FNAMES_SEG_UINT16: u16 = 0x0100;
const OSON_FLAG_TINY_NODES_STAT: u16 = 0x2000;
const OSON_FLAG_TREE_SEG_UINT32: u16 = 0x1000;

// define OSON magic bytes
const OSON_MAGIC_BYTE_1: u8 = 0xff;
const OSON_MAGIC_BYTE_2: u8 = 0x4a;
const OSON_MAGIC_BYTE_3: u8 = 0x5a;

// define OSON data types
const OSON_TYPE_NULL: u8 = 0x30;
const OSON_TYPE_TRUE: u8 = 0x31;
const OSON_TYPE_FALSE: u8 = 0x32;
const OSON_TYPE_STRING_LENGTH_UINT8: u8 = 0x33;
const OSON_TYPE_NUMBER_LENGTH_UINT8: u8 = 0x34;
const OSON_TYPE_BINARY_DOUBLE: u8 = 0x36;
const OSON_TYPE_STRING_LENGTH_UINT16: u8 = 0x37;
const OSON_TYPE_STRING_LENGTH_UINT32: u8 = 0x38;
const OSON_TYPE_TIMESTAMP: u8 = 0x39;
const OSON_TYPE_BINARY_LENGTH_UINT16: u8 = 0x3a;
const OSON_TYPE_BINARY_LENGTH_UINT32: u8 = 0x3b;
const OSON_TYPE_DATE: u8 = 0x3c;
const OSON_TYPE_INTERVAL_YM: u8 = 0x3d;
const OSON_TYPE_INTERVAL_DS: u8 = 0x3e;
const OSON_TYPE_TIMESTAMP_TZ: u8 = 0x7c;
const OSON_TYPE_TIMESTAMP7: u8 = 0x7d;
const OSON_TYPE_ID: u8 = 0x7e;
const OSON_TYPE_BINARY_FLOAT: u8 = 0x7f;
const OSON_TYPE_OBJECT: u8 = 0x84;
const OSON_TYPE_ARRAY: u8 = 0xc0;
const OSON_TYPE_VECTOR: u8 = 0x01;
const OSON_TYPE_EXTENDED: u8 = 0x7b;

// define OSON version numbers
const OSON_VERSION_MAX_FNAME_255: u8 = 1;
const OSON_VERSION_MAX_FNAME_65535: u8 = 3;

/// Types of JSON values supported.
#[derive(Debug, Clone)]
pub enum JsonValue {
    BinaryDouble(f64),
    BinaryFloat(f32),
    Boolean(bool),
    IntervalDS(OracleIntervalDS),
    IntervalYM(OracleIntervalYM),
    JsonArray(Vec<JsonValue>),
    JsonId(Vec<u8>),
    JsonObject(HashMap<String, JsonValue>),
    Null,
    Number(OracleNumber),
    Raw(Vec<u8>),
    String(String),
    Timestamp(OracleTimestamp),
    Vector(Vector),
}

struct OsonDecoder {
    relative_offsets: bool,
    tree_seg_pos: usize,
    field_id_length: usize,
    field_names: Vec<String>,
}

struct OsonDecoderFieldNamesSeg {
    num_fields: usize,
    offsets_size: usize,
    seg_size: usize,
    lengths_size: usize,
}

struct OsonEncoder {
    field_names: HashSet<String>,
    short_field_names_seg: OsonEncoderFieldNamesSeg,
    long_field_names_seg: OsonEncoderFieldNamesSeg,
    field_id_length: usize,
}

struct OsonEncoderFieldNamesSeg {
    buf: WriteBuffer,
    field_names: Vec<OsonFieldName>,
}

struct OsonEncoderTreeSeg {
    buf: WriteBuffer,
    field_id_map: HashMap<String, usize>,
    field_id_length: usize,
}

struct OsonFieldName {
    name: String,
    hash_id: u32,
    offset: usize,
    field_id: usize,
}

/// Returns information about a container given its node type. Whether the node
/// is shared or not and the number of children in the container are returned.
fn get_container_info(
    buf: &mut ReadBuffer,
    node_type: u8,
) -> Result<(bool, usize), Error> {
    let children_bits: u8 = node_type & 0x18;
    if children_bits == 0 {
        Ok((false, buf.read_u8()? as usize))
    } else if children_bits == 0x08 {
        Ok((false, buf.read_u16be()? as usize))
    } else if children_bits == 0x10 {
        Ok((false, buf.read_u32be()? as usize))
    } else {
        Ok((true, 0))
    }
}

/// Returns an offset given the node type. The offset will be either a 16-bit
/// or 32-bit value depending on the value of the 3rd signficant bit of the
/// node type.
fn get_offset(buf: &mut ReadBuffer, node_type: u8) -> Result<usize, Error> {
    if node_type & 0x20 != 0 {
        Ok(buf.read_u32be()? as usize)
    } else {
        Ok(buf.read_u16be()? as usize)
    }
}

impl JsonValue {
    /// Returns a boolean indicating if the value refers to an array.
    fn is_array(&self) -> bool {
        matches!(self, JsonValue::JsonArray(_))
    }

    /// Returns a boolean indicating if the value refers to an object.
    fn is_object(&self) -> bool {
        matches!(self, JsonValue::JsonObject(_))
    }

    /// Returns a boolean indicating if the value refers to a scalar or not.
    fn is_scalar(&self) -> bool {
        !(self.is_array() || self.is_object())
    }
}

impl OsonDecoderFieldNamesSeg {
    /// Decodes the names in the segment and adds them to the list of field
    /// names.
    fn decode_names(
        &self,
        buf: &mut ReadBuffer,
        names: &mut Vec<String>,
    ) -> Result<(), Error> {
        // skip the hash id array
        let _ = buf.read_bytes(self.num_fields * self.lengths_size)?;

        // skip the field name offsets array
        let mut offsets_pos = buf.get_pos();
        let _ = buf.read_bytes(self.num_fields * self.offsets_size)?;
        let base_data_pos = buf.get_pos();

        // determine the names of the fields; these are encoded as a length
        // followed by the actual string data
        for _ in 0..self.num_fields {
            buf.set_pos(offsets_pos);
            let offset = if self.offsets_size == 2 {
                buf.read_u16be()? as usize
            } else {
                buf.read_u32be()? as usize
            };
            offsets_pos = buf.get_pos();
            buf.set_pos(base_data_pos + offset);
            let len = if self.lengths_size == 1 {
                buf.read_u8()? as usize
            } else {
                buf.read_u16be()? as usize
            };
            let name_data = buf.read_bytes(len)?.to_vec();
            let name = String::from_utf8(name_data).unwrap();
            names.push(name);
        }

        buf.set_pos(base_data_pos + self.seg_size);
        Ok(())
    }

    /// Creates a new field names segment deocder and returns it.
    fn new(lengths_size: usize) -> Self {
        Self {
            num_fields: 0,
            offsets_size: 0,
            seg_size: 0,
            lengths_size,
        }
    }
}

impl OsonDecoder {
    /// Decodes the OSON encoded data and returns the root level node.
    fn decode(&mut self, buf: &mut ReadBuffer) -> Result<JsonValue, Error> {
        // validate that the first three bytes are the expected "magic" bytes
        // that identify an OSON encoded image
        let magic_bytes = buf.read_bytes(3)?;
        if magic_bytes[0] != OSON_MAGIC_BYTE_1
            || magic_bytes[1] != OSON_MAGIC_BYTE_2
            || magic_bytes[2] != OSON_MAGIC_BYTE_3
        {
            return Err(Error::invalid_oson_encoded_bytes());
        }

        // validate the OSON version is supported
        let version = buf.read_u8()?;
        if version != OSON_VERSION_MAX_FNAME_255
            && version != OSON_VERSION_MAX_FNAME_65535
        {
            return Err(Error::unsupported_oson_version(version));
        }

        // determine whether relative offsets are being used
        let primary_flags = buf.read_u16be()?;
        if primary_flags & OSON_FLAG_REL_OFFSET_MODE != 0 {
            self.relative_offsets = true;
        }

        // if value is a scalar node, the header is much smaller
        if primary_flags & OSON_FLAG_IS_SCALAR != 0 {
            if primary_flags & OSON_FLAG_TREE_SEG_UINT32 != 0 {
                buf.read_u32be()?;
            } else {
                buf.read_u16be()?;
            }
            return self.decode_node(buf);
        }

        // determine the number of short field names and the field id length
        let mut short_names_decoder = OsonDecoderFieldNamesSeg::new(1);
        if primary_flags & OSON_FLAG_NUM_FNAMES_UINT32 != 0 {
            self.field_id_length = 4;
            short_names_decoder.num_fields = buf.read_u32be()? as usize;
        } else if primary_flags & OSON_FLAG_NUM_FNAMES_UINT16 != 0 {
            self.field_id_length = 2;
            short_names_decoder.num_fields = buf.read_u16be()? as usize;
        } else {
            self.field_id_length = 1;
            short_names_decoder.num_fields = buf.read_u8()? as usize;
        };

        // determine the size of the short field names segment
        if primary_flags & OSON_FLAG_FNAMES_SEG_UINT32 != 0 {
            short_names_decoder.offsets_size = 4;
            short_names_decoder.seg_size = buf.read_u32be()? as usize;
        } else {
            short_names_decoder.offsets_size = 2;
            short_names_decoder.seg_size = buf.read_u16be()? as usize;
        }

        // if the version indicates that field names > 255 bytes exist, parse
        // the information about that segment now
        let mut long_names_decoder = OsonDecoderFieldNamesSeg::new(2);
        if version == OSON_VERSION_MAX_FNAME_65535 {
            let secondary_flags = buf.read_u16be()?;
            if secondary_flags & OSON_FLAG_SEC_FNAMES_SEG_UINT16 != 0 {
                long_names_decoder.offsets_size = 2;
            } else {
                long_names_decoder.offsets_size = 4;
            }
            long_names_decoder.num_fields = buf.read_u32be()? as usize;
            long_names_decoder.seg_size = buf.read_u32be()? as usize;
        }

        // determine the size of the tree segment
        let _tree_seg_size = if primary_flags & OSON_FLAG_TREE_SEG_UINT32 != 0
        {
            buf.read_u32be()?
        } else {
            buf.read_u16be()? as u32
        };

        // determine the number of "tiny" nodes (unused)
        let _num_tiny_nodes = buf.read_u16be()?;

        // get the list of field names
        if short_names_decoder.num_fields > 0 {
            short_names_decoder.decode_names(buf, &mut self.field_names)?;
        }
        if long_names_decoder.num_fields > 0 {
            long_names_decoder.decode_names(buf, &mut self.field_names)?;
        }

        // save tree segment position
        self.tree_seg_pos = buf.get_pos();

        // return root node
        self.decode_node(buf)
    }

    /// Decodes a container node (array or object) and returns it.
    fn decode_array_node(
        &mut self,
        buf: &mut ReadBuffer,
        node_type: u8,
    ) -> Result<JsonValue, Error> {
        let (_, num_children) = get_container_info(buf, node_type)?;
        let mut offsets_pos = buf.get_pos();
        let mut array: Vec<JsonValue> = Vec::new();
        for _ in 0..num_children {
            buf.set_pos(offsets_pos);
            let offset = get_offset(buf, node_type)?;
            offsets_pos = buf.get_pos();
            buf.set_pos(self.tree_seg_pos + offset);
            let value = self.decode_node(buf)?;
            array.push(value);
        }
        Ok(JsonValue::JsonArray(array))
    }

    /// Decodes a container node (array or object) and returns it.
    fn decode_object_node(
        &mut self,
        buf: &mut ReadBuffer,
        node_type: u8,
    ) -> Result<JsonValue, Error> {
        let (is_shared, mut num_children) =
            get_container_info(buf, node_type)?;
        let mut obj: HashMap<String, JsonValue> = HashMap::new();
        let mut offsets_pos = if is_shared {
            let offset = get_offset(buf, node_type)?;
            let offsets_pos = buf.get_pos();
            buf.set_pos(self.tree_seg_pos + offset);
            let shared_node_type = buf.read_u8()?;
            let (_, shared_num_children) =
                get_container_info(buf, shared_node_type)?;
            num_children = shared_num_children;
            offsets_pos
        } else {
            buf.get_pos() + num_children * self.field_id_length
        };
        let mut field_ids_pos = buf.get_pos();
        for _ in 0..num_children {
            buf.set_pos(field_ids_pos);
            let field_id = match self.field_id_length {
                1 => buf.read_u8()? as usize,
                2 => buf.read_u16be()? as usize,
                _ => buf.read_u32be()? as usize,
            };
            field_ids_pos = buf.get_pos();
            let name = self.field_names[field_id - 1].to_string();
            buf.set_pos(offsets_pos);
            let offset = get_offset(buf, node_type)?;
            offsets_pos = buf.get_pos();
            buf.set_pos(self.tree_seg_pos + offset);
            let value = self.decode_node(buf)?;
            obj.insert(name, value);
        }
        Ok(JsonValue::JsonObject(obj))
    }

    /// Decodes a single node and returns it.
    fn decode_node(
        &mut self,
        buf: &mut ReadBuffer,
    ) -> Result<JsonValue, Error> {
        let node_type = buf.read_u8()?;
        if node_type & 0x80 != 0 {
            if node_type & 0x40 == 0 {
                return self.decode_object_node(buf, node_type);
            }
            return self.decode_array_node(buf, node_type);
        }
        match node_type {
            OSON_TYPE_NULL => Ok(JsonValue::Null),
            OSON_TYPE_TRUE => Ok(JsonValue::Boolean(true)),
            OSON_TYPE_FALSE => Ok(JsonValue::Boolean(false)),
            OSON_TYPE_DATE | OSON_TYPE_TIMESTAMP7 => {
                let bytes = buf.read_bytes(constants::ORA_TYPE_SIZE_DATE)?;
                let value = OracleTimestamp::from_buf(bytes);
                Ok(JsonValue::Timestamp(value))
            }
            OSON_TYPE_TIMESTAMP => {
                let bytes =
                    buf.read_bytes(constants::ORA_TYPE_SIZE_TIMESTAMP)?;
                let value = OracleTimestamp::from_buf(bytes);
                Ok(JsonValue::Timestamp(value))
            }
            OSON_TYPE_TIMESTAMP_TZ => {
                let bytes =
                    buf.read_bytes(constants::ORA_TYPE_SIZE_TIMESTAMP_TZ)?;
                let value = OracleTimestamp::from_buf(bytes);
                Ok(JsonValue::Timestamp(value))
            }
            OSON_TYPE_BINARY_DOUBLE => {
                let bytes =
                    buf.read_bytes(constants::ORA_TYPE_SIZE_BINARY_DOUBLE)?;
                let value = f64::from_buf(bytes);
                Ok(JsonValue::BinaryDouble(value))
            }
            OSON_TYPE_BINARY_FLOAT => {
                let bytes =
                    buf.read_bytes(constants::ORA_TYPE_SIZE_BINARY_FLOAT)?;
                let value = f32::from_buf(bytes);
                Ok(JsonValue::BinaryFloat(value))
            }
            OSON_TYPE_INTERVAL_DS => {
                let bytes =
                    buf.read_bytes(constants::ORA_TYPE_SIZE_INTERVAL_DS)?;
                let value = OracleIntervalDS::from_buf(bytes);
                Ok(JsonValue::IntervalDS(value))
            }
            OSON_TYPE_INTERVAL_YM => {
                let bytes =
                    buf.read_bytes(constants::ORA_TYPE_SIZE_INTERVAL_YM)?;
                let value = OracleIntervalYM::from_buf(bytes);
                Ok(JsonValue::IntervalYM(value))
            }
            OSON_TYPE_BINARY_LENGTH_UINT16 => {
                let len = buf.read_u16be()? as usize;
                let bytes = buf.read_bytes(len)?;
                Ok(JsonValue::Raw(bytes.to_vec()))
            }
            OSON_TYPE_BINARY_LENGTH_UINT32 => {
                let len = buf.read_u32be()? as usize;
                let bytes = buf.read_bytes(len)?;
                Ok(JsonValue::Raw(bytes.to_vec()))
            }
            OSON_TYPE_NUMBER_LENGTH_UINT8 => {
                let len = buf.read_u8()? as usize;
                let bytes = buf.read_bytes(len)?;
                let value = OracleNumber::from_buf(bytes);
                Ok(JsonValue::Number(value))
            }
            OSON_TYPE_STRING_LENGTH_UINT8 => {
                let len = buf.read_u8()? as usize;
                let bytes = buf.read_bytes(len)?;
                let value = String::from_utf8(bytes.to_vec()).unwrap();
                Ok(JsonValue::String(value))
            }
            OSON_TYPE_STRING_LENGTH_UINT16 => {
                let len = buf.read_u16be()? as usize;
                let bytes = buf.read_bytes(len)?;
                let value = String::from_utf8(bytes.to_vec()).unwrap();
                Ok(JsonValue::String(value))
            }
            OSON_TYPE_STRING_LENGTH_UINT32 => {
                let len = buf.read_u32be()? as usize;
                let bytes = buf.read_bytes(len)?;
                let value = String::from_utf8(bytes.to_vec()).unwrap();
                Ok(JsonValue::String(value))
            }
            OSON_TYPE_EXTENDED => {
                let subtype = buf.read_u8()?;
                match subtype {
                    OSON_TYPE_VECTOR => {
                        let len = buf.read_u32be()? as usize;
                        let raw = buf.read_bytes(len)?;
                        let mut sub = ReadBuffer::from_bytes(raw);
                        let vector = Vector::from_buf_fallible(&mut sub)?;
                        Ok(JsonValue::Vector(vector))
                    }
                    _ => Err(Error::unsupported_oson_node_type(subtype)),
                }
            }
            0x00 => Ok(JsonValue::String(String::new())),
            0x01..=0x1f => {
                let bytes = buf.read_bytes(node_type.into())?;
                let value = String::from_utf8(bytes.to_vec()).unwrap();
                Ok(JsonValue::String(value))
            }
            0x20..=0x2f | 0x60..=0x6f => {
                let num_bytes = ((node_type & 0x0f) + 1) as usize;
                let bytes = buf.read_bytes(num_bytes)?;
                let value = OracleNumber::from_buf(bytes);
                Ok(JsonValue::Number(value))
            }
            0x40..=0x5f => {
                let num_bytes = (node_type & 0x0f) as usize;
                let bytes = buf.read_bytes(num_bytes)?;
                let value = OracleNumber::from_buf(bytes);
                Ok(JsonValue::Number(value))
            }
            _ => Err(Error::unsupported_oson_node_type(node_type)),
        }
    }

    /// Returns a new OsonDecoder instance.
    fn new() -> Self {
        Self {
            tree_seg_pos: 0,
            relative_offsets: false,
            field_id_length: 0,
            field_names: Vec::new(),
        }
    }
}

impl OsonEncoder {
    /// Adds a field name to the list of field names and places it in the
    /// correct field names segment.
    fn add_field_name(&mut self, name: &str) {
        let field_name = OsonFieldName::new(name);
        self.field_names.insert(name.to_string());
        if name.len() <= 255 {
            self.short_field_names_seg.add_name(field_name);
        } else {
            self.long_field_names_seg.add_name(field_name);
        }
    }

    /// Determines the set of flags to use for the OSON header.
    fn determine_flags(&mut self, value: &JsonValue) -> u16 {
        // if value is a simple scalar, nothing more needs to be done
        let mut flags: u16 = OSON_FLAG_INLINE_LEAF;
        if value.is_scalar() {
            return flags | OSON_FLAG_IS_SCALAR;
        }

        // examine all values recursively to determine the unique set of field
        // names and whether they need to be added to the short field names
        // segment (names <= 255 bytes) or the long field names segment
        // (greater than 255 bytes)
        self.examine_node(value);
        self.short_field_names_seg.process_field_names(0);
        let num_short_field_names =
            self.short_field_names_seg.field_names.len();
        self.long_field_names_seg
            .process_field_names(num_short_field_names);

        // determine remaining flags and field id size
        flags |= OSON_FLAG_HASH_ID_UINT8 | OSON_FLAG_TINY_NODES_STAT;
        match self.field_names.len() {
            0..=255 => {
                self.field_id_length = 1;
            }
            256..=65535 => {
                self.field_id_length = 2;
                flags |= OSON_FLAG_NUM_FNAMES_UINT16;
            }
            _ => {
                self.field_id_length = 4;
                flags |= OSON_FLAG_NUM_FNAMES_UINT32;
            }
        }
        flags
    }

    /// Encodes the value as OSON encoded bytes into a buffer and returns it.
    fn encode(&mut self, value: &JsonValue) -> WriteBuffer {
        let mut buf = WriteBuffer::new();
        let mut flags = self.determine_flags(value);

        // encode the values in the tree segment
        let segs = [&self.short_field_names_seg, &self.long_field_names_seg];
        let mut tree_seg =
            OsonEncoderTreeSeg::new(&segs, self.field_id_length);
        tree_seg.encode_node(value);
        let tree_seg_length = tree_seg.buf.len();
        if tree_seg_length > 65535 {
            flags |= OSON_FLAG_TREE_SEG_UINT32;
        }

        // write the initial header
        buf.write_u8(OSON_MAGIC_BYTE_1);
        buf.write_u8(OSON_MAGIC_BYTE_2);
        buf.write_u8(OSON_MAGIC_BYTE_3);
        if self.long_field_names_seg.field_names.is_empty() {
            buf.write_u8(OSON_VERSION_MAX_FNAME_255);
        } else {
            buf.write_u8(OSON_VERSION_MAX_FNAME_65535);
        }
        buf.write_u16be(flags);

        // write extended header, if applicable
        if !value.is_scalar() {
            self.write_extended_header(&mut buf);
        }

        // write size of tree segment
        if tree_seg_length < 65536 {
            buf.write_u16be(tree_seg_length.try_into().unwrap());
        } else {
            buf.write_u32be(tree_seg_length.try_into().unwrap());
        }

        // write remainder of header and any field name segments, if applicable
        if !value.is_scalar() {
            buf.write_u16be(0); // number of "tiny" nodes (always zero)
            for seg in segs {
                seg.write_to_buf(&mut buf);
            }
        }

        // write tree segment data
        buf.write_bytes(tree_seg.buf.get_buf());

        // return buffer containing encoded data
        buf
    }

    /// Examines the value. If is an object, all keys are extracted and unique
    /// names retained. Elements in arrays and the values in objects are then
    /// examined to determine if they contain objects as well.
    fn examine_node(&mut self, value: &JsonValue) {
        if let JsonValue::JsonArray(array) = value {
            for child_value in array {
                self.examine_node(child_value);
            }
        } else if let JsonValue::JsonObject(map) = value {
            for (key, child_value) in map {
                if !self.field_names.contains(key) {
                    self.add_field_name(key);
                }
                self.examine_node(child_value);
            }
        }
    }

    /// Creates a new OSON encoder.
    fn new() -> Self {
        Self {
            field_names: HashSet::new(),
            short_field_names_seg: OsonEncoderFieldNamesSeg::new(),
            long_field_names_seg: OsonEncoderFieldNamesSeg::new(),
            field_id_length: 0,
        }
    }

    /// Writes the extended header containing informationg about the short and
    /// long field name segments.
    fn write_extended_header(&self, buf: &mut WriteBuffer) {
        // write the number of short field names
        let num_short_field_names =
            self.short_field_names_seg.field_names.len();
        match self.field_id_length {
            1 => buf.write_u8(num_short_field_names.try_into().unwrap()),
            2 => buf.write_u16be(num_short_field_names.try_into().unwrap()),
            _ => buf.write_u32be(num_short_field_names.try_into().unwrap()),
        }

        // write the size of the short field names segment
        let short_names_seg_length = self.short_field_names_seg.buf.len();
        if short_names_seg_length < 65536 {
            buf.write_u16be(short_names_seg_length.try_into().unwrap());
        } else {
            buf.write_u32be(short_names_seg_length.try_into().unwrap());
        }

        // write information about the long field names segment, if applicable
        if !self.long_field_names_seg.field_names.is_empty() {
            let seg_length = self.long_field_names_seg.buf.len();
            let num_field_names = self.long_field_names_seg.field_names.len();
            let mut secondary_flags: u16 = 0;
            if seg_length < 65536 {
                secondary_flags = OSON_FLAG_SEC_FNAMES_SEG_UINT16;
            }
            buf.write_u16be(secondary_flags);
            buf.write_u32be(num_field_names.try_into().unwrap());
            buf.write_u32be(seg_length.try_into().unwrap());
        }
    }
}

impl OsonEncoderFieldNamesSeg {
    /// Adds a name to the field names segment.
    fn add_name(&mut self, mut field_name: OsonFieldName) {
        field_name.offset = self.buf.len();
        if field_name.name.len() <= 255 {
            self.buf.write_u8(field_name.name.len().try_into().unwrap());
        } else {
            self.buf
                .write_u16be(field_name.name.len().try_into().unwrap());
        }
        self.buf.write_bytes(field_name.name.as_bytes());
        self.field_names.push(field_name);
    }

    /// Creates a new OSON encoder field names segment.
    fn new() -> Self {
        Self {
            buf: WriteBuffer::new(),
            field_names: Vec::new(),
        }
    }

    /// Processes the field names in preparation for encoding within OSON. The
    /// names must be sorted as required by the format.
    fn process_field_names(&mut self, field_id_offset: usize) {
        self.field_names
            .sort_by_key(|n| (n.hash_id, n.name.len(), n.name.clone()));
        let mut field_id = field_id_offset + 1;
        for field_name in &mut self.field_names {
            field_name.field_id = field_id;
            field_id += 1;
        }
    }

    /// Writes the contents of the field names segment to the buffer.
    fn write_to_buf(&self, buf: &mut WriteBuffer) {
        // write array of hash ids
        for field_name in &self.field_names {
            if field_name.name.len() <= 255 {
                buf.write_u8(field_name.hash_id.try_into().unwrap());
            } else {
                buf.write_u16be(field_name.hash_id.try_into().unwrap());
            }
        }

        // write array of field name offsets
        let seg_length = self.buf.len();
        for field_name in &self.field_names {
            if seg_length < 65536 {
                buf.write_u16be(field_name.offset.try_into().unwrap());
            } else {
                buf.write_u32be(field_name.offset.try_into().unwrap());
            }
        }

        // write the field name data
        buf.write_bytes(self.buf.get_buf());
    }
}

impl OsonEncoderTreeSeg {
    /// Encodes a JSON array node in the tree segment.
    fn encode_array(&mut self, array: &Vec<JsonValue>) {
        self.encode_container_header(OSON_TYPE_ARRAY, array.len());
        let mut offsets_pos = self.buf.reserve_bytes(array.len() * 4);
        let mut offsets: Vec<usize> = Vec::new();
        for value in array {
            offsets.push(self.buf.len());
            self.encode_node(value);
        }
        for offset in offsets {
            self.buf
                .write_u32be_at(offset.try_into().unwrap(), offsets_pos);
            offsets_pos += 4;
        }
    }

    /// Encodes the first part of a container node (array/object).
    fn encode_container_header(&mut self, node_type: u8, num_children: usize) {
        let mut final_node_type = node_type;
        final_node_type |= 0x20; // use u32 for offsets
        if num_children > 65535 {
            final_node_type |= 0x10; // use u32 for num children
        } else if num_children > 255 {
            final_node_type |= 0x08; // use u16 for num children
        }
        self.buf.write_u8(final_node_type);
        match num_children {
            0..=255 => self.buf.write_u8(num_children.try_into().unwrap()),
            256..=65535 => {
                self.buf.write_u16be(num_children.try_into().unwrap())
            }
            _ => self.buf.write_u32be(num_children.try_into().unwrap()),
        }
    }

    /// Encodes a node in the tree segment.
    fn encode_node(&mut self, value: &JsonValue) {
        match value {
            JsonValue::JsonArray(array) => self.encode_array(array),
            JsonValue::JsonObject(map) => self.encode_object(map),
            JsonValue::Null => self.buf.write_u8(OSON_TYPE_NULL),
            JsonValue::BinaryDouble(v) => {
                self.buf.write_u8(OSON_TYPE_BINARY_DOUBLE);
                v.to_buf(&mut self.buf, &DB_TYPE_BINARY_DOUBLE, false);
            }
            JsonValue::BinaryFloat(v) => {
                self.buf.write_u8(OSON_TYPE_BINARY_FLOAT);
                v.to_buf(&mut self.buf, &DB_TYPE_BINARY_FLOAT, false);
            }
            JsonValue::Boolean(v) => match *v {
                true => self.buf.write_u8(OSON_TYPE_TRUE),
                false => self.buf.write_u8(OSON_TYPE_FALSE),
            },
            JsonValue::IntervalDS(v) => {
                self.buf.write_u8(OSON_TYPE_INTERVAL_DS);
                v.to_buf(&mut self.buf, &DB_TYPE_INTERVAL_DS, false);
            }
            JsonValue::IntervalYM(v) => {
                self.buf.write_u8(OSON_TYPE_INTERVAL_YM);
                v.to_buf(&mut self.buf, &DB_TYPE_INTERVAL_YM, false);
            }
            JsonValue::JsonId(v) => {
                self.buf.write_u8(OSON_TYPE_ID);
                self.buf.write_bytes_with_length(v);
            }
            JsonValue::Number(v) => {
                self.buf.write_u8(OSON_TYPE_NUMBER_LENGTH_UINT8);
                v.to_buf(&mut self.buf, &DB_TYPE_NUMBER, true);
            }
            JsonValue::Raw(v) => {
                if v.len() < 65536 {
                    self.buf.write_u8(OSON_TYPE_BINARY_LENGTH_UINT16);
                    self.buf.write_u16be(v.len().try_into().unwrap());
                } else {
                    self.buf.write_u8(OSON_TYPE_BINARY_LENGTH_UINT32);
                    self.buf.write_u32be(v.len().try_into().unwrap());
                }
                self.buf.write_bytes(v);
            }
            JsonValue::String(v) => {
                match v.len() {
                    0..=255 => {
                        self.buf.write_u8(OSON_TYPE_STRING_LENGTH_UINT8);
                        self.buf.write_u8(v.len().try_into().unwrap());
                    }
                    256..=65535 => {
                        self.buf.write_u8(OSON_TYPE_STRING_LENGTH_UINT16);
                        self.buf.write_u16be(v.len().try_into().unwrap());
                    }
                    _ => {
                        self.buf.write_u8(OSON_TYPE_STRING_LENGTH_UINT32);
                        self.buf.write_u32be(v.len().try_into().unwrap());
                    }
                }
                if !v.is_empty() {
                    self.buf.write_bytes(v.as_bytes());
                }
            }
            JsonValue::Timestamp(v) => {
                if v.nanoseconds() == 0 {
                    self.buf.write_u8(OSON_TYPE_TIMESTAMP7);
                    v.to_buf(&mut self.buf, &DB_TYPE_DATE, false);
                } else {
                    self.buf.write_u8(OSON_TYPE_TIMESTAMP);
                    v.to_buf(&mut self.buf, &DB_TYPE_TIMESTAMP, false);
                }
            }
            JsonValue::Vector(v) => {
                let mut encoded = WriteBuffer::new();
                v.encode(&mut encoded);
                self.buf.write_u8(OSON_TYPE_EXTENDED);
                self.buf.write_u8(OSON_TYPE_VECTOR);
                self.buf.write_u32be(encoded.len().try_into().unwrap());
                self.buf.write_bytes(encoded.get_buf());
            }
        }
    }

    /// Encodes a JSON object node in the tree segment.
    fn encode_object(&mut self, map: &HashMap<String, JsonValue>) {
        self.encode_container_header(OSON_TYPE_OBJECT, map.len());
        for key in map.keys() {
            let field_id = *self.field_id_map.get(key).unwrap();
            match self.field_id_length {
                1 => self.buf.write_u8(field_id.try_into().unwrap()),
                2 => self.buf.write_u16be(field_id.try_into().unwrap()),
                _ => self.buf.write_u32be(field_id.try_into().unwrap()),
            }
        }
        let mut offsets_pos = self.buf.reserve_bytes(map.len() * 4);
        let mut offsets: Vec<usize> = Vec::new();
        for value in map.values() {
            offsets.push(self.buf.len());
            self.encode_node(value);
        }
        for offset in offsets {
            self.buf
                .write_u32be_at(offset.try_into().unwrap(), offsets_pos);
            offsets_pos += 4;
        }
    }

    /// Creates a new OSON encoder tree segment
    fn new(
        field_name_segs: &[&OsonEncoderFieldNamesSeg],
        field_id_length: usize,
    ) -> Self {
        let mut field_id_map: HashMap<String, usize> = HashMap::new();
        for seg in field_name_segs {
            for field_name in &seg.field_names {
                field_id_map
                    .insert(field_name.name.clone(), field_name.field_id);
            }
        }
        Self {
            buf: WriteBuffer::new(),
            field_id_length,
            field_id_map,
        }
    }
}

impl OsonFieldName {
    /// Creates a new OSON field name.
    fn new(name: &str) -> Self {
        // calculate the hash id to use for the field name. This is based on
        // Bernstein's hash function.
        let mut hash_id: u32 = 0x811C9DC5;
        for byte in name.as_bytes() {
            hash_id = (hash_id ^ (*byte as u32)).wrapping_mul(16777619);
        }
        if name.len() <= 255 {
            hash_id &= 0xff;
        } else {
            hash_id &= 0xffff;
        }
        Self {
            name: name.to_string(),
            hash_id,
            offset: 0,
            field_id: 0,
        }
    }
}

impl FromBufFallible for JsonValue {
    /// Decodes a JsonValue from a ReadBuffer instance containing OSON-encoded
    /// bytes.
    fn from_buf_fallible(buf: &mut ReadBuffer) -> Result<Self, Error> {
        OsonDecoder::new().decode(buf)
    }
}

impl ToBuf for JsonValue {
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        _db_type: &'static DbType,
        _write_length: bool,
    ) {
        buf.write_qlocator(&OsonEncoder::new().encode(self))
    }
}

/// Encodes a JSON value into raw OSON bytes.
pub(crate) fn encode_oson(value: &JsonValue) -> Vec<u8> {
    OsonEncoder::new().encode(value).get_buf().to_vec()
}
