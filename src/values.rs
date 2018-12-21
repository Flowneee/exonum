// Copyright 2018 The Exonum Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A definition of `BinaryForm` trait and implementations for common types.

use std::{borrow::Cow, io::Read};

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use chrono::{DateTime, NaiveDateTime, Utc};
use failure::{self, format_err};
use rust_decimal::Decimal;
use uuid::Uuid;

use super::UniqueHash;
use exonum_crypto::{Hash, PublicKey};

/// A type that can be (de)serialized as a value in the blockchain storage.
///
/// If you need to implement `BinaryForm` for your types, use little-endian encoding
/// for integer types for compatibility with modern architectures.
///
/// # Examples
///
/// Implementing `BinaryForm` for the type:
///
/// ```
/// use std::{borrow::Cow, io::{Read, Write}};
/// use byteorder::{LittleEndian, ReadBytesExt, ByteOrder};
/// use failure;
/// use exonum_merkledb::BinaryForm;
///
/// #[derive(Clone)]
/// struct Data {
///     a: i16,
///     b: u32,
/// }
///
/// impl BinaryForm for Data {
///     fn to_bytes(&self) -> Vec<u8> {
///         let mut buf = vec![0_u8; 6];
///         LittleEndian::write_i16(&mut buf[0..2], self.a);
///         LittleEndian::write_u32(&mut buf[2..6], self.b);
///         buf
///     }
///
///     fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, failure::Error> {
///         let mut buf = bytes.as_ref();
///         let a = buf.read_i16::<LittleEndian>()?;
///         let b = buf.read_u32::<LittleEndian>()?;
///         Ok(Self { a, b })
///     }
/// }
/// # fn main() {}
/// ```
pub trait BinaryForm: Sized {
    /// Serializes the given value to the vector of bytes.
    fn to_bytes(&self) -> Vec<u8>;
    /// TODO
    fn into_bytes(self) -> Vec<u8> {
        self.to_bytes()
    }
    /// Deserializes the value from the given bytes array.
    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, failure::Error>;
}

macro_rules! impl_binary_form_scalar {
    ($type:tt, $read:ident) => {
        impl BinaryForm for $type {
            fn to_bytes(&self) -> Vec<u8> {
                vec![*self as u8]
            }

            fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, failure::Error> {
                use byteorder::ReadBytesExt;
                bytes.as_ref().$read().map_err(From::from)
            }
        }

        impl UniqueHash for $type {}
    };
    ($type:tt, $write:ident, $read:ident, $len:expr) => {
        impl BinaryForm for $type {
            fn to_bytes(&self) -> Vec<u8> {
                let mut v = vec![0; $len];
                LittleEndian::$write(&mut v, *self);
                v
            }

            fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, failure::Error> {
                use byteorder::ReadBytesExt;
                bytes.as_ref().$read::<LittleEndian>().map_err(From::from)
            }
        }

        impl UniqueHash for $type {}
    };
}

// Unsigned scalar types
impl_binary_form_scalar! { u8,  read_u8 }
impl_binary_form_scalar! { u16, write_u16, read_u16, 2 }
impl_binary_form_scalar! { u32, write_u32, read_u32, 4 }
impl_binary_form_scalar! { u64, write_u64, read_u64, 8 }
// Signed scalar types
impl_binary_form_scalar! { i8,  read_i8 }
impl_binary_form_scalar! { i16, write_i16, read_i16, 2 }
impl_binary_form_scalar! { i32, write_i32, read_i32, 4 }
impl_binary_form_scalar! { i64, write_i64, read_i64, 8 }

/// No-op implementation.
impl BinaryForm for () {
    fn to_bytes(&self) -> Vec<u8> {
        Vec::default()
    }

    fn from_bytes(_bytes: Cow<[u8]>) -> Result<Self, failure::Error> {
        Ok(())
    }
}

impl UniqueHash for () {}

impl BinaryForm for bool {
    fn to_bytes(&self) -> Vec<u8> {
        vec![*self as u8]
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, failure::Error> {
        let value = bytes.as_ref();
        assert_eq!(value.len(), 1);

        match value[0] {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(format_err!("Invalid value for bool: {}", value)),
        }
    }
}

impl UniqueHash for bool {}

impl BinaryForm for Vec<u8> {
    fn to_bytes(&self) -> Vec<u8> {
        self.clone()
    }

    fn into_bytes(self) -> Vec<u8> {
        self    
    }    

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, failure::Error> {
        Ok(bytes.as_ref().to_owned())
    }
}

impl UniqueHash for Vec<u8> {}

impl BinaryForm for String {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_owned()
    }

    fn into_bytes(self) -> Vec<u8> {
        Self::into_bytes(self)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, failure::Error> {
        Self::from_utf8(bytes.as_ref().to_owned()).map_err(From::from)
    }
}

impl UniqueHash for String {}

impl BinaryForm for Hash {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_ref().to_vec()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, failure::Error> {
        Self::from_slice(bytes.as_ref()).ok_or_else(|| format_err!("Unable to decode value"))
    }
}

impl BinaryForm for PublicKey {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_ref().to_vec()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, failure::Error> {
        Self::from_slice(bytes.as_ref()).ok_or_else(|| format_err!("Unable to decode value"))
    }
}

impl UniqueHash for PublicKey {}

// FIXME Maybe we should remove this implementations

impl BinaryForm for DateTime<Utc> {
    fn to_bytes(&self) -> Vec<u8> {
        let secs = self.timestamp();
        let nanos = self.timestamp_subsec_nanos();

        let mut buffer = vec![0; 12];
        LittleEndian::write_i64(&mut buffer[0..8], secs);
        LittleEndian::write_u32(&mut buffer[8..12], nanos);
        buffer
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, failure::Error> {
        let mut value = bytes.as_ref();
        let secs = value.read_i64::<LittleEndian>()?;
        let nanos = value.read_u32::<LittleEndian>()?;
        Ok(Self::from_utc(
            NaiveDateTime::from_timestamp(secs, nanos),
            Utc,
        ))
    }
}

impl UniqueHash for DateTime<Utc> {}

impl BinaryForm for Uuid {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, failure::Error> {
        Self::from_slice(bytes.as_ref()).map_err(From::from)
    }
}

impl UniqueHash for Uuid {}

impl BinaryForm for Decimal {
    fn to_bytes(&self) -> Vec<u8> {
        self.serialize().to_vec()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, failure::Error> {
        let mut value = bytes.as_ref();
        let mut buf: [u8; 16] = [0; 16];
        value.read_exact(&mut buf)?;
        Ok(Self::deserialize(buf))
    }
}

impl UniqueHash for Decimal {}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::str::FromStr;

    use chrono::Duration;

    use super::*;

    fn assert_round_trip_eq<T: BinaryForm + PartialEq + Debug>(values: &[T]) {
        for value in values {
            let bytes = value.to_bytes();
            assert_eq!(*value, <T as BinaryForm>::from_bytes(bytes.into()).unwrap());
        }
    }

    macro_rules! impl_test_binary_form_scalar_unsigned {
        ($name:ident, $type:tt) => {
            #[test]
            fn $name() {
                let values = [$type::min_value(), 1, $type::max_value()];
                assert_round_trip_eq(&values);
            }
        };
    }

    macro_rules! impl_test_binary_form_scalar_signed {
        ($name:ident, $type:tt) => {
            #[test]
            fn $name() {
                let values = [$type::min_value(), -1, 0, 1, $type::max_value()];
                assert_round_trip_eq(&values);
            }
        };
    }

    // Impl tests for unsigned scalar types
    impl_test_binary_form_scalar_unsigned! { test_binary_form_round_trip_u8,  u8 }
    impl_test_binary_form_scalar_unsigned! { test_binary_form_round_trip_u32, u32 }
    impl_test_binary_form_scalar_unsigned! { test_binary_form_round_trip_u16, u16 }
    impl_test_binary_form_scalar_unsigned! { test_binary_form_round_trip_u64, u64 }

    // Impl tests for signed scalar types
    impl_test_binary_form_scalar_signed! { test_binary_form_round_trip_i8,  i8 }
    impl_test_binary_form_scalar_signed! { test_binary_form_round_trip_i16, i16 }
    impl_test_binary_form_scalar_signed! { test_binary_form_round_trip_i32, i32 }
    impl_test_binary_form_scalar_signed! { test_binary_form_round_trip_i64, i64 }

    // Tests for the other types

    #[test]
    fn test_binary_form_vec_u8() {
        let values = [vec![], vec![1], vec![1, 2, 3], vec![255; 100]];
        assert_round_trip_eq(&values);
    }

    #[test]
    fn test_binary_form_bool_correct() {
        let values = [true, false];
        assert_round_trip_eq(&values);
    }

    #[test]
    #[should_panic(expected = "Invalid value for bool: 2")]
    fn test_binary_form_bool_incorrect() {
        let bytes = 2_u8.to_bytes();
        <bool as BinaryForm>::from_bytes(bytes.into()).unwrap();
    }

    #[test]
    fn test_binary_form_string() {
        let values: Vec<_> = ["", "e", "2", "hello"]
            .iter()
            .map(|v| v.to_string())
            .collect();
        assert_round_trip_eq(&values);
    }

    #[test]
    fn test_binary_form_datetime() {
        use chrono::TimeZone;

        let times = [
            Utc.timestamp(0, 0),
            Utc.timestamp(13, 23),
            Utc::now(),
            Utc::now() + Duration::seconds(17) + Duration::nanoseconds(15),
            Utc.timestamp(0, 999_999_999),
            Utc.timestamp(0, 1_500_000_000), // leap second
        ];
        assert_round_trip_eq(&times);
    }

    #[test]
    fn test_binary_form_uuid() {
        let values = [
            Uuid::nil(),
            Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap(),
            Uuid::parse_str("0000002a-000c-0005-0c03-0938362b0809").unwrap(),
        ];
        assert_round_trip_eq(&values);
    }

    #[test]
    fn test_binary_form_decimal() {
        let values = [
            Decimal::from_str("3.14").unwrap(),
            Decimal::from_parts(1102470952, 185874565, 1703060790, false, 28),
            Decimal::new(9497628354687268, 12),
            Decimal::from_str("0").unwrap(),
            Decimal::from_str("-0.000000000000000000019").unwrap(),
        ];
        assert_round_trip_eq(&values);
    }
}
