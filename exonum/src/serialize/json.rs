/// trait `ExonumSerializeJson` implemented for all field that allows serializing in 
/// json like format.
///

//\TODO refer to difference between json serialization and exonum_json 
//\TODO implement Field for float, signed integers
//\TODO implement Field for crypto structures
//\TODO remove WriteBufferWraper hack (after refactor storage), should be moved into storage
//\TODO split deserialization for `in-place` and regular
use serde::{Serializer, Serialize, Deserializer, Deserialize};
use serde_json::value::Value;
use bit_vec::BitVec;
use hex::ToHex;
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::sync::Arc;
use std::net::SocketAddr;

use crypto::{Hash, PublicKey, SecretKey, Seed, Signature};

use messages::Field;
use super::HexValue;

pub trait ExonumJsonSerialize {
    fn serialize<S: Serializer>(& self, serializer: S) -> Result<S::Ok, S::Error>;
}

/// helper function, fro wrapping value that should be serialized as `ExonumJsonSerialize`
pub fn wrap<'a, T: ExonumJsonSerialize>(val: &'a T) ->  ExonumJsonSerializeWrapper<'a, T> {
    ExonumJsonSerializeWrapper(val)
}

/// Wrapping struct that allows implementing custom serializing aspects in json.
pub struct ExonumJsonSerializeWrapper<'a, T: ExonumJsonSerialize + 'a>( &'a T);

impl<'a, T: ExonumJsonSerialize + 'a > Serialize for ExonumJsonSerializeWrapper<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.0.serialize(serializer)                
    }
}

impl_default_serialize!{ExonumJsonSerialize => 
    u8; u16; u32; i8; i16; i32; bool;
    Signature; SocketAddr}

impl_default_serialize_deref!{ExonumJsonSerialize =>
    Hash; PublicKey; SecretKey; Seed}

impl ExonumJsonSerialize for u64 {
    fn serialize<S: Serializer>(& self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl ExonumJsonSerialize for i64 {
    fn serialize<S: Serializer>(& self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl ExonumJsonSerialize for BitVec {
    fn serialize<S: Serializer>(& self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = String::new();
        for i in self.iter() {
            if i {
                out.push('1');
            }
            else {
                out.push('0');
            }
        }
        serializer.serialize_str(&out)
    }
}

impl ExonumJsonSerialize for Arc<::messages::MessageBuffer>
{
    fn serialize<S: Serializer>(& self, serializer: S) -> Result<S::Ok, S::Error> {
        let slice = self.as_ref();
        serializer.serialize_str(&slice.to_hex())
    } 
}

impl<'a> ExonumJsonSerialize for &'a [Hash]
{
    fn serialize<S: Serializer>(& self, serializer: S) -> Result<S::Ok, S::Error> {
        let vec:Vec<_> = self.iter().collect();
        serializer.collect_seq(vec.iter().map(|v| wrap(v)))
    }   
}

impl<T> ExonumJsonSerialize for Vec<T>
where T: ExonumJsonSerialize
{
    fn serialize<S: Serializer>(& self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter().map(|v| wrap(v)))
    }   
}

#[derive(Serialize, Deserialize)]
struct DurationHelper {
    secs: u64,
    nanos: u32
}

impl ExonumJsonSerialize for SystemTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use ::serde::ser::Error;
        let duration = self.duration_since(UNIX_EPOCH).map_err(S::Error::custom)?;
        let duration = DurationHelper {
            secs: duration.as_secs(),
            nanos: duration.subsec_nanos()
        };
        <DurationHelper as Serialize>::serialize(&duration, serializer)
    }
}

pub trait WriteBufferWrapper {
    fn write<'a, T: Field<'a> >(&'a mut self, from: usize, to: usize, val:T);
}

impl WriteBufferWrapper for ::messages::MessageWriter {
    fn write<'a, T: Field<'a> >(&'a mut self, from: usize, to: usize, val:T){
        self.write(val, from, to)
    }
}

impl WriteBufferWrapper for Vec<u8> {
    fn write<'a, T: Field<'a>>(&'a mut self, from: usize, to: usize, val:T){
        val.write(self, from, to)
    }
}

pub trait ExonumJsonDeserialize {
    fn deserialize_default(value: &Value) -> Option<Self> where Self: Sized{
            None
    }
    fn deserialize<B: WriteBufferWrapper>(value: &Value, buffer: & mut B, from: usize, to: usize ) -> bool;
}


macro_rules! impl_deserialize_int {
    (@impl $typename:ty) => {
        impl ExonumJsonDeserialize for $typename {
            fn deserialize<B: WriteBufferWrapper>(value: &Value, buffer: & mut B, from: usize, to: usize ) -> bool {
                    value.as_i64()
                         .map(|v| v as $typename)
                         .map(|val| buffer.write(from, to, val))
                         .is_some()
            }
        }
    };
    ($($name:ty);*) => ($(impl_deserialize_int!{@impl $name})*);
}
impl_deserialize_int!{
    u8; u16; u32 /*i8; i16; i32;*/ }

macro_rules! impl_deserialize_bigint {
    (@impl $typename:ty) => {
        impl ExonumJsonDeserialize for $typename {
            fn deserialize<B: WriteBufferWrapper>(value: &Value, buffer: & mut B, from: usize, to: usize ) -> bool {
                    value.as_str()
                         .and_then(|v| v.parse().ok() )
                         .map(|val: $typename| buffer.write(from, to, val))
                         .is_some()
            }
        }
    };
    ($($name:ty);*) => ($(impl_deserialize_bigint!{@impl $name})*);
}
impl_deserialize_bigint!{u64; i64}
/*
macro_rules! impl_deserialize_float {
    (@impl $traitname:ident $typename:ty) => {
        impl<'a> ExonumJsonDeserialize for $typename {
            fn deserialize(value: &Value, buffer: &'a mut Vec<u8>, from: usize, to: usize ) -> bool {
                    value.as_f64()
                         .map(|v| v as $typename)
                         .map(|val| val.write(buffer, from, to))
                         .is_some()
            }
        }
    };
    ( $($name:ty);*) => ($(impl_deserialize_float!{@impl  $name})*);
}
impl_deserialize_int!{ f32; f64 }
*/

macro_rules! impl_deserialize_hex_segment {
    (@impl $typename:ty) => {
        impl<'a> ExonumJsonDeserialize for &'a $typename {
            fn deserialize<B: WriteBufferWrapper>(value: &Value, buffer: & mut B, from: usize, to: usize ) -> bool {
                    value.as_str()
                         .and_then(|v| <$typename as HexValue>:: from_hex(v).ok() )
                         .map(|ref val| buffer.write(from, to, val))
                         .is_some()
            }
        }
    };
    ($($name:ty);*) => ($(impl_deserialize_hex_segment!{@impl $name})*);
}


impl_deserialize_hex_segment!{
    Hash; PublicKey; Signature /*;  Seed; */}

impl ExonumJsonDeserialize for bool  {
    fn deserialize<B: WriteBufferWrapper>(value: &Value, buffer: & mut B, from: usize, to: usize ) -> bool {
        value.as_bool()
             .map(|val| buffer.write(from, to, val))
             .is_some()
    }
}

impl ExonumJsonDeserialize for SystemTime  {
    fn deserialize<B: WriteBufferWrapper>(value: &Value, buffer: & mut B, from: usize, to: usize ) -> bool {
        let helper: Option<DurationHelper> = ::serde_json::from_value(value.clone()).ok();
        helper.map(|helper|{
            let duration = Duration::new(helper.secs, helper.nanos);
            let system_time = UNIX_EPOCH + duration;
            buffer.write(from, to, system_time)
        }).is_some()
        
    }
}

impl ExonumJsonDeserialize for SocketAddr {
    fn deserialize<B: WriteBufferWrapper>(value: &Value, buffer: & mut B, from: usize, to: usize ) -> bool {
        let helper: Option<SocketAddr> = ::serde_json::from_value(value.clone()).ok();
        helper.map(|addr|{
            buffer.write(from, to, addr)
        }).is_some()
        
    }
}

impl<'a> ExonumJsonDeserialize for &'a [Hash]  {
    fn deserialize<B: WriteBufferWrapper>(value: &Value, buffer: & mut B, from: usize, to: usize ) -> bool {
        value.as_array()
             .and_then(|arr| {
                let mut vec: Vec<Hash>= Vec::new();
                for el in arr {
                    let hash = el.as_str().and_then(|v| <Hash as HexValue>:: from_hex(v).ok());
                    if let Some(hash) = hash {
                        vec.push(hash)
                    } else {
                        return None;
                    }
                }
                    Some(buffer.write(from, to, vec.as_slice()))
                
             })
             .is_some()
    }
}

impl ExonumJsonDeserialize for Vec<Arc<::messages::MessageBuffer>>
{
    fn deserialize<B: WriteBufferWrapper>(value: &Value, buffer: & mut B, from: usize, to: usize ) -> bool {
        use ::messages::MessageBuffer;
        value.as_array()
             .and_then(|arr| {
                let mut vec: Vec<_>= Vec::new();
                for el in arr {
                    let str_hex = el.as_str().and_then(|v| <Vec<u8> as HexValue>:: from_hex(v).ok());;
                    if let Some(ob) = str_hex {
                        vec.push(Arc::new(MessageBuffer::from_vec(ob)));
                    } else {
                        return None;
                    }
                }
                    Some(buffer.write(from, to, vec))
             })
             .is_some()
    }
}

impl<T> ExonumJsonDeserialize for Vec<T>
where T: ExonumJsonDeserialize,
    for<'a> Vec<T>: Field<'a>
{
    fn deserialize<B: WriteBufferWrapper>(value: &Value, buffer: & mut B, from: usize, to: usize ) -> bool {
        value.as_array()
             .and_then(|arr| {
                let mut vec: Vec<_>= Vec::new();
                for el in arr {
                    let obj = T::deserialize_default(el);
                    if let Some(ob) = obj {
                        vec.push(ob);
                    } else {
                        return None;
                    }
                }
                    Some(buffer.write(from, to, vec))
             })
             .is_some()
    }    
}

impl ExonumJsonDeserialize for BitVec  {
    fn deserialize<B: WriteBufferWrapper>(value: &Value, buffer: & mut B, from: usize, to: usize ) -> bool {
        value.as_str()
             .and_then(|val| {
                let mut vec = BitVec::new();
                for (i, ch) in val.chars().enumerate() {
                    let val = if ch == '1' {
                        true
                    }
                    else if ch == '0' {
                        false
                    }
                    else {
                        return None
                    };
                    vec.set(i, val);
                }

                buffer.write(from, to, vec);
                Some(())
             })
             .is_some()
    }
}



//api 

pub fn to_string<T: ExonumJsonSerialize>(value: &T) -> Option<String> {    
    let mut buf = Vec::new();
    
    let mut ser = ::serde_json::Serializer::new(&mut buf);
    let ret = value.serialize(&mut ser).ok();
    ret.and_then(move |_| String::from_utf8(buf).ok())
}

pub fn from_str<T: ExonumJsonDeserialize>(value: &str) -> Option<T> {    
    let value: Option<Value> = ::serde_json::from_str(value).ok();
    value.and_then(| val|
        T::deserialize_default(&val))
}