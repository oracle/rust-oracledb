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
// vector.rs
//
// This code implements encoding and decoding for Oracle's VECTOR data
// type,which is used to store vector embeddings and similar numerical arrays
// in Oracle databases.
//-----------------------------------------------------------------------------

use crate::db_type::DbType;
use crate::error::Error;
use crate::read_buffer::FromBuf;
use crate::read_buffer::FromBufFallible;
use crate::read_buffer::ReadBuffer;
use crate::write_buffer::ToBuf;
use crate::write_buffer::WriteBuffer;

// magic byte and the version constants
const VECTOR_MAGIC_BYTE: u8 = 0xDB;
const VECTOR_VERSION_BASE: u8 = 0;
const VECTOR_VERSION_WITH_BINARY: u8 = 1;
const VECTOR_VERSION_WITH_SPARSE: u8 = 2;

// vector flags
const VECTOR_FLAG_NORM: u16 = 0x0002;
const VECTOR_FLAG_NORM_RESERVED: u16 = 0x0010;
const VECTOR_FLAG_SPARSE: u16 = 0x0020;

/// Storage formats possible for vectors.
#[derive(Debug, Clone)]
pub enum VectorStorageFormat {
    Float32 = 0x02,
    Float64 = 0x03,
    Int8 = 0x04,
    Binary = 0x05,
}

impl VectorStorageFormat {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Float32 => "float32",
            Self::Float64 => "float64",
            Self::Int8 => "int8",
            Self::Binary => "binary",
        }
    }
}

impl TryFrom<u8> for VectorStorageFormat {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            v if v == Self::Float32 as u8 => Ok(Self::Float32),
            v if v == Self::Float64 as u8 => Ok(Self::Float64),
            v if v == Self::Int8 as u8 => Ok(Self::Int8),
            v if v == Self::Binary as u8 => Ok(Self::Binary),
            _ => Err(Error::unsupported_vector_format(value)),
        }
    }
}

/// Types of data supported by vectors.
#[derive(Debug, Clone)]
pub enum VectorData {
    Float32(Vec<f32>),
    Float64(Vec<f64>),
    Int8(Vec<i8>),
    Binary(Vec<u8>),
}

impl VectorData {
    /// Decodes the vector data from the buffer.
    fn decode(
        buf: &mut ReadBuffer,
        num_elements: usize,
        vector_format: VectorStorageFormat,
    ) -> Result<Self, Error> {
        match vector_format {
            VectorStorageFormat::Float32 => {
                let mut values = Vec::with_capacity(num_elements);
                for _ in 0..num_elements {
                    values.push(f32::from_buf(buf.read_bytes(4)?));
                }
                Ok(Self::Float32(values))
            }
            VectorStorageFormat::Float64 => {
                let mut values = Vec::with_capacity(num_elements);
                for _ in 0..num_elements {
                    values.push(f64::from_buf(buf.read_bytes(8)?));
                }
                Ok(Self::Float64(values))
            }
            VectorStorageFormat::Int8 => {
                let mut values = Vec::with_capacity(num_elements);
                for _ in 0..num_elements {
                    values.push(buf.read_i8()?);
                }
                Ok(Self::Int8(values))
            }
            VectorStorageFormat::Binary => {
                let byte_count = num_elements / 8;
                let mut values = Vec::with_capacity(byte_count);
                for _ in 0..byte_count {
                    values.push(buf.read_u8()?);
                }
                Ok(Self::Binary(values))
            }
        }
    }

    /// Encodes the values in a VectorData structure into the format required
    /// by the database.
    fn encode(&self, buf: &mut WriteBuffer) {
        match self {
            Self::Int8(values) => {
                for value in values {
                    buf.write_u8(*value as u8);
                }
            }
            Self::Binary(values) => {
                for value in values {
                    buf.write_u8(*value);
                }
            }
            Self::Float32(values) => {
                for value in values {
                    value.to_buf(buf, crate::DB_TYPE_BINARY_FLOAT, false);
                }
            }
            Self::Float64(values) => {
                for value in values {
                    value.to_buf(buf, crate::DB_TYPE_BINARY_DOUBLE, false);
                }
            }
        }
    }

    /// Returns the format of the elements in the vector.
    fn format(&self) -> VectorStorageFormat {
        match self {
            Self::Float32(_) => VectorStorageFormat::Float32,
            Self::Float64(_) => VectorStorageFormat::Float64,
            Self::Int8(_) => VectorStorageFormat::Int8,
            Self::Binary(_) => VectorStorageFormat::Binary,
        }
    }

    /// Returns the number of dimensions in the vector.
    fn num_dimensions(&self) -> usize {
        match self {
            VectorData::Float32(v) => v.len(),
            VectorData::Float64(v) => v.len(),
            VectorData::Int8(v) => v.len(),
            VectorData::Binary(v) => v.len() * 8,
        }
    }
}

/// Represents a sparse vector.
#[derive(Debug, Clone)]
pub struct SparseVector {
    num_dimensions: usize,
    indices: Vec<usize>,
    values: VectorData,
}

impl SparseVector {
    /// Decodes the sparse vector data from the buffer.
    fn decode(
        buf: &mut ReadBuffer,
        num_dimensions: usize,
        vector_format: VectorStorageFormat,
    ) -> Result<Self, Error> {
        let num_sparse_elements = buf.read_u16be()? as usize;
        let mut indices = Vec::with_capacity(num_sparse_elements);
        for _ in 0..num_sparse_elements {
            indices.push(buf.read_u32be()? as usize);
        }
        let values =
            VectorData::decode(buf, num_sparse_elements, vector_format)?;
        Ok(Self {
            num_dimensions,
            indices,
            values,
        })
    }

    /// Encodes the sparse vector in the format required by the database.
    fn encode(&self, buf: &mut WriteBuffer) {
        buf.write_u16be(self.indices.len().try_into().unwrap());
        for ix in &self.indices {
            buf.write_u32be((*ix).try_into().unwrap());
        }
        self.values.encode(buf);
    }

    /// Creates a new sparse vector and returns it.
    pub fn new(
        num_dimensions: usize,
        indices: Vec<usize>,
        values: VectorData,
    ) -> Self {
        Self {
            num_dimensions,
            indices,
            values,
        }
    }

    /// Returns the number of dimensions in the sparse vector.
    pub fn num_dimensions(&self) -> usize {
        self.num_dimensions
    }

    /// Returns the indices of the sparse vector that are non-zero.
    pub fn indices(&self) -> &[usize] {
        &self.indices
    }

    /// Returns the values of sparse vector that are non-zero.
    pub fn values(&self) -> &VectorData {
        &self.values
    }
}

/// Tyeps of vectors supported by the library.
#[derive(Debug, Clone)]
pub enum Vector {
    Dense(VectorData),
    Sparse(SparseVector),
}

impl Vector {
    /// Decodes a vector from the read buffer and returns it.
    fn decode(buf: &mut ReadBuffer) -> Result<Self, Error> {
        // read header
        let magic_byte = buf.read_u8()?;
        if magic_byte != VECTOR_MAGIC_BYTE {
            return Err(Error::invalid_encoded_vector());
        }
        let version = buf.read_u8()?;
        if version > VECTOR_VERSION_WITH_SPARSE {
            return Err(Error::unsupported_vector_version(version));
        }
        let flags = buf.read_u16be()?;
        let vector_format = VectorStorageFormat::try_from(buf.read_u8()?)?;
        let num_elements = buf.read_u32be()? as usize;

        // skip norm data if present (unused)
        if (flags & VECTOR_FLAG_NORM_RESERVED) != 0
            || (flags & VECTOR_FLAG_NORM) != 0
        {
            buf.read_bytes(8)?;
        }

        // handle sparse vectors
        if flags & VECTOR_FLAG_SPARSE != 0 {
            let sparse =
                SparseVector::decode(buf, num_elements, vector_format)?;
            Ok(Self::Sparse(sparse))
        } else {
            let values = VectorData::decode(buf, num_elements, vector_format)?;
            Ok(Self::Dense(values))
        }
    }

    /// Returns the flags to use when encoding the vector.
    fn flags(&self) -> u16 {
        let base_flags = VECTOR_FLAG_NORM_RESERVED | VECTOR_FLAG_NORM;
        match self {
            Self::Dense(_) => base_flags,
            Self::Sparse(_) => base_flags | VECTOR_FLAG_SPARSE,
        }
    }

    /// Returns the format to use when encoding the vector.
    fn format(&self) -> VectorStorageFormat {
        match self {
            Self::Dense(data) => data.format(),
            Self::Sparse(sparse) => sparse.values.format(),
        }
    }

    /// Returns the total number of dimensions in the vector.
    fn num_dimensions(&self) -> usize {
        match self {
            Self::Dense(data) => data.num_dimensions(),
            Self::Sparse(sparse) => sparse.num_dimensions,
        }
    }

    /// Returns the version to use when encoding the vector.
    fn version(&self) -> u8 {
        match self {
            Self::Dense(data) => match data {
                VectorData::Binary(_) => VECTOR_VERSION_WITH_BINARY,
                _ => VECTOR_VERSION_BASE,
            },
            Self::Sparse(_) => VECTOR_VERSION_WITH_SPARSE,
        }
    }

    /// Encode the vector in the format required by the database.
    pub(crate) fn encode(&self, buf: &mut WriteBuffer) {
        // write header
        buf.write_u8(VECTOR_MAGIC_BYTE);
        buf.write_u8(self.version());
        buf.write_u16be(self.flags());
        buf.write_u8(self.format() as u8);
        buf.write_u32be(self.num_dimensions().try_into().unwrap());
        buf.write_bytes(&[0u8; 8]); // norm (unused)

        // write data
        match self {
            Vector::Dense(data) => {
                data.encode(buf);
            }
            Vector::Sparse(sparse) => {
                sparse.encode(buf);
            }
        }
    }
}

impl FromBufFallible for Vector {
    fn from_buf_fallible(buf: &mut ReadBuffer) -> Result<Self, Error> {
        Vector::decode(buf)
    }
}

impl ToBuf for Vector {
    fn to_buf(
        &self,
        buf: &mut WriteBuffer,
        _db_type: &'static DbType,
        _write_length: bool,
    ) {
        let mut encode_buf = WriteBuffer::new();
        self.encode(&mut encode_buf);
        buf.write_qlocator(&encode_buf);
    }
}
