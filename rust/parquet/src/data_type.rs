// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Data types that connect Parquet physical types with their Rust-specific
//! representations.

use std::mem;

use byteorder::{BigEndian, ByteOrder};

use crate::basic::Type;
use crate::util::memory::{ByteBuffer, ByteBufferPtr};

/// Rust representation for logical type INT96, value is backed by an array of `u32`.
/// The type only takes 12 bytes, without extra padding.
#[derive(Clone, Debug)]
pub struct Int96 {
    value: Option<[u32; 3]>,
}

impl Int96 {
    /// Creates new INT96 type struct with no data set.
    pub fn new() -> Self {
        Self { value: None }
    }

    /// Returns underlying data as slice of [`u32`].
    pub fn data(&self) -> &[u32] {
        assert!(self.value.is_some());
        self.value.as_ref().unwrap()
    }

    /// Sets data for this INT96 type.
    pub fn set_data(&mut self, elem0: u32, elem1: u32, elem2: u32) {
        self.value = Some([elem0, elem1, elem2]);
    }
}

impl Default for Int96 {
    fn default() -> Self {
        Self { value: None }
    }
}

impl PartialEq for Int96 {
    fn eq(&self, other: &Int96) -> bool {
        self.data() == other.data()
    }
}

impl From<Vec<u32>> for Int96 {
    fn from(buf: Vec<u32>) -> Self {
        assert_eq!(buf.len(), 3);
        let mut result = Self::new();
        result.set_data(buf[0], buf[1], buf[2]);
        result
    }
}

/// Rust representation for BYTE_ARRAY and FIXED_LEN_BYTE_ARRAY Parquet physical types.
/// Value is backed by a byte buffer.
#[derive(Clone, Debug)]
pub struct ByteArray {
    data: Option<ByteBufferPtr>,
}

impl ByteArray {
    /// Creates new byte array with no data set.
    pub fn new() -> Self {
        ByteArray { data: None }
    }

    /// Gets length of the underlying byte buffer.
    pub fn len(&self) -> usize {
        assert!(self.data.is_some());
        self.data.as_ref().unwrap().len()
    }

    /// Returns slice of data.
    pub fn data(&self) -> &[u8] {
        assert!(self.data.is_some());
        self.data.as_ref().unwrap().as_ref()
    }

    /// Set data from another byte buffer.
    pub fn set_data(&mut self, data: ByteBufferPtr) {
        self.data = Some(data);
    }

    /// Returns `ByteArray` instance with slice of values for a data.
    pub fn slice(&self, start: usize, len: usize) -> Self {
        assert!(self.data.is_some());
        Self::from(self.data.as_ref().unwrap().range(start, len))
    }
}

impl From<Vec<u8>> for ByteArray {
    fn from(buf: Vec<u8>) -> ByteArray {
        Self {
            data: Some(ByteBufferPtr::new(buf)),
        }
    }
}

impl<'a> From<&'a str> for ByteArray {
    fn from(s: &'a str) -> ByteArray {
        let mut v = Vec::new();
        v.extend_from_slice(s.as_bytes());
        Self {
            data: Some(ByteBufferPtr::new(v)),
        }
    }
}

impl From<ByteBufferPtr> for ByteArray {
    fn from(ptr: ByteBufferPtr) -> ByteArray {
        Self { data: Some(ptr) }
    }
}

impl From<ByteBuffer> for ByteArray {
    fn from(mut buf: ByteBuffer) -> ByteArray {
        Self {
            data: Some(buf.consume()),
        }
    }
}

impl Default for ByteArray {
    fn default() -> Self {
        ByteArray { data: None }
    }
}

impl PartialEq for ByteArray {
    fn eq(&self, other: &ByteArray) -> bool {
        self.data() == other.data()
    }
}

/// Rust representation for Decimal values.
///
/// This is not a representation of Parquet physical type, but rather a wrapper for
/// DECIMAL logical type, and serves as container for raw parts of decimal values:
/// unscaled value in bytes, precision and scale.
#[derive(Clone, Debug)]
pub enum Decimal {
    /// Decimal backed by `i32`.
    Int32 {
        value: [u8; 4],
        precision: i32,
        scale: i32,
    },
    /// Decimal backed by `i64`.
    Int64 {
        value: [u8; 8],
        precision: i32,
        scale: i32,
    },
    /// Decimal backed by byte array.
    Bytes {
        value: ByteArray,
        precision: i32,
        scale: i32,
    },
}

impl Decimal {
    /// Creates new decimal value from `i32`.
    pub fn from_i32(value: i32, precision: i32, scale: i32) -> Self {
        let mut bytes = [0; 4];
        BigEndian::write_i32(&mut bytes, value);
        Decimal::Int32 {
            value: bytes,
            precision,
            scale,
        }
    }

    /// Creates new decimal value from `i64`.
    pub fn from_i64(value: i64, precision: i32, scale: i32) -> Self {
        let mut bytes = [0; 8];
        BigEndian::write_i64(&mut bytes, value);
        Decimal::Int64 {
            value: bytes,
            precision,
            scale,
        }
    }

    /// Creates new decimal value from `ByteArray`.
    pub fn from_bytes(value: ByteArray, precision: i32, scale: i32) -> Self {
        Decimal::Bytes {
            value,
            precision,
            scale,
        }
    }

    /// Returns bytes of unscaled value.
    pub fn data(&self) -> &[u8] {
        match *self {
            Decimal::Int32 { ref value, .. } => value,
            Decimal::Int64 { ref value, .. } => value,
            Decimal::Bytes { ref value, .. } => value.data(),
        }
    }

    /// Returns decimal precision.
    pub fn precision(&self) -> i32 {
        match *self {
            Decimal::Int32 { precision, .. } => precision,
            Decimal::Int64 { precision, .. } => precision,
            Decimal::Bytes { precision, .. } => precision,
        }
    }

    /// Returns decimal scale.
    pub fn scale(&self) -> i32 {
        match *self {
            Decimal::Int32 { scale, .. } => scale,
            Decimal::Int64 { scale, .. } => scale,
            Decimal::Bytes { scale, .. } => scale,
        }
    }
}

impl Default for Decimal {
    fn default() -> Self {
        Self::from_i32(0, 0, 0)
    }
}

impl PartialEq for Decimal {
    fn eq(&self, other: &Decimal) -> bool {
        self.precision() == other.precision()
            && self.scale() == other.scale()
            && self.data() == other.data()
    }
}

/// Converts an instance of data type to a slice of bytes as `u8`.
pub trait AsBytes {
    /// Returns slice of bytes for this data type.
    fn as_bytes(&self) -> &[u8];
}

macro_rules! gen_as_bytes {
    ($source_ty:ident) => {
        impl AsBytes for $source_ty {
            fn as_bytes(&self) -> &[u8] {
                unsafe {
                    ::std::slice::from_raw_parts(
                        self as *const $source_ty as *const u8,
                        ::std::mem::size_of::<$source_ty>(),
                    )
                }
            }
        }
    };
}

gen_as_bytes!(bool);
gen_as_bytes!(u8);
gen_as_bytes!(i32);
gen_as_bytes!(u32);
gen_as_bytes!(i64);
gen_as_bytes!(f32);
gen_as_bytes!(f64);

impl AsBytes for Int96 {
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            ::std::slice::from_raw_parts(self.data() as *const [u32] as *const u8, 12)
        }
    }
}

impl AsBytes for ByteArray {
    fn as_bytes(&self) -> &[u8] {
        self.data()
    }
}

impl AsBytes for Decimal {
    fn as_bytes(&self) -> &[u8] {
        self.data()
    }
}

impl AsBytes for Vec<u8> {
    fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<'a> AsBytes for &'a str {
    fn as_bytes(&self) -> &[u8] {
        (self as &str).as_bytes()
    }
}

impl AsBytes for str {
    fn as_bytes(&self) -> &[u8] {
        (self as &str).as_bytes()
    }
}

/// Contains the Parquet physical type information as well as the Rust primitive type
/// presentation.
pub trait DataType: 'static {
    type T: ::std::cmp::PartialEq
        + ::std::fmt::Debug
        + ::std::default::Default
        + ::std::clone::Clone
        + AsBytes;

    /// Returns Parquet physical type.
    fn get_physical_type() -> Type;

    /// Returns size in bytes for Rust representation of the physical type.
    fn get_type_size() -> usize;
}

macro_rules! make_type {
    ($name:ident, $physical_ty:path, $native_ty:ty, $size:expr) => {
        pub struct $name {}

        impl DataType for $name {
            type T = $native_ty;

            fn get_physical_type() -> Type {
                $physical_ty
            }

            fn get_type_size() -> usize {
                $size
            }
        }
    };
}

/// Generate struct definitions for all physical types

make_type!(BoolType, Type::BOOLEAN, bool, 1);
make_type!(Int32Type, Type::INT32, i32, 4);
make_type!(Int64Type, Type::INT64, i64, 8);
make_type!(Int96Type, Type::INT96, Int96, mem::size_of::<Int96>());
make_type!(FloatType, Type::FLOAT, f32, 4);
make_type!(DoubleType, Type::DOUBLE, f64, 8);
make_type!(
    ByteArrayType,
    Type::BYTE_ARRAY,
    ByteArray,
    mem::size_of::<ByteArray>()
);
make_type!(
    FixedLenByteArrayType,
    Type::FIXED_LEN_BYTE_ARRAY,
    ByteArray,
    mem::size_of::<ByteArray>()
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_bytes() {
        assert_eq!(false.as_bytes(), &[0]);
        assert_eq!(true.as_bytes(), &[1]);
        assert_eq!((7 as i32).as_bytes(), &[7, 0, 0, 0]);
        assert_eq!((555 as i32).as_bytes(), &[43, 2, 0, 0]);
        assert_eq!((555 as u32).as_bytes(), &[43, 2, 0, 0]);
        assert_eq!(i32::max_value().as_bytes(), &[255, 255, 255, 127]);
        assert_eq!(i32::min_value().as_bytes(), &[0, 0, 0, 128]);
        assert_eq!((7 as i64).as_bytes(), &[7, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!((555 as i64).as_bytes(), &[43, 2, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            (i64::max_value()).as_bytes(),
            &[255, 255, 255, 255, 255, 255, 255, 127]
        );
        assert_eq!((i64::min_value()).as_bytes(), &[0, 0, 0, 0, 0, 0, 0, 128]);
        assert_eq!((3.14 as f32).as_bytes(), &[195, 245, 72, 64]);
        assert_eq!(
            (3.14 as f64).as_bytes(),
            &[31, 133, 235, 81, 184, 30, 9, 64]
        );
        assert_eq!("hello".as_bytes(), &[b'h', b'e', b'l', b'l', b'o']);
        assert_eq!(
            Vec::from("hello".as_bytes()).as_bytes(),
            &[b'h', b'e', b'l', b'l', b'o']
        );

        // Test Int96
        let i96 = Int96::from(vec![1, 2, 3]);
        assert_eq!(i96.as_bytes(), &[1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]);

        // Test ByteArray
        let ba = ByteArray::from(vec![1, 2, 3]);
        assert_eq!(ba.as_bytes(), &[1, 2, 3]);

        // Test Decimal
        let decimal = Decimal::from_i32(123, 5, 2);
        assert_eq!(decimal.as_bytes(), &[0, 0, 0, 123]);
        let decimal = Decimal::from_i64(123, 5, 2);
        assert_eq!(decimal.as_bytes(), &[0, 0, 0, 0, 0, 0, 0, 123]);
        let decimal = Decimal::from_bytes(ByteArray::from(vec![1, 2, 3]), 5, 2);
        assert_eq!(decimal.as_bytes(), &[1, 2, 3]);
    }

    #[test]
    fn test_int96_from() {
        assert_eq!(
            Int96::from(vec![1, 12345, 1234567890]).data(),
            &[1, 12345, 1234567890]
        );
    }

    #[test]
    fn test_byte_array_from() {
        assert_eq!(
            ByteArray::from(vec![b'A', b'B', b'C']).data(),
            &[b'A', b'B', b'C']
        );
        assert_eq!(ByteArray::from("ABC").data(), &[b'A', b'B', b'C']);
        assert_eq!(
            ByteArray::from(ByteBufferPtr::new(vec![1u8, 2u8, 3u8, 4u8, 5u8])).data(),
            &[1u8, 2u8, 3u8, 4u8, 5u8]
        );
        let mut buf = ByteBuffer::new();
        buf.set_data(vec![6u8, 7u8, 8u8, 9u8, 10u8]);
        assert_eq!(ByteArray::from(buf).data(), &[6u8, 7u8, 8u8, 9u8, 10u8]);
    }

    #[test]
    fn test_decimal_partial_eq() {
        assert_eq!(Decimal::default(), Decimal::from_i32(0, 0, 0));
        assert_eq!(Decimal::from_i32(222, 5, 2), Decimal::from_i32(222, 5, 2));
        assert_eq!(
            Decimal::from_bytes(ByteArray::from(vec![0, 0, 0, 3]), 5, 2),
            Decimal::from_i32(3, 5, 2)
        );

        assert!(Decimal::from_i32(222, 5, 2) != Decimal::from_i32(111, 5, 2));
        assert!(Decimal::from_i32(222, 5, 2) != Decimal::from_i32(222, 6, 2));
        assert!(Decimal::from_i32(222, 5, 2) != Decimal::from_i32(222, 5, 3));

        assert!(Decimal::from_i64(222, 5, 2) != Decimal::from_i32(222, 5, 2));
    }
}
