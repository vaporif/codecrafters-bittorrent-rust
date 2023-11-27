use anyhow::Result;
use serde::ser::SerializeMap;
use std::collections::BTreeMap;
const END_CHAR: &[u8; 1] = b"e";

use crate::prelude::*;
pub fn to_bytes<T>(data: T) -> Result<Vec<u8>>
where
    T: serde::Serialize,
{
    let mut serializer = Serializer { data: Vec::new() };
    data.serialize(&mut serializer)?;
    Ok(serializer.data.clone())
}

pub struct Serializer {
    data: Vec<u8>,
}

impl Serializer {
    fn add<T>(&mut self, value: T)
    where
        T: Iterator<Item = u8>,
    {
        self.data.extend(value);
    }

    fn end(&mut self) {
        self.add(END_CHAR.iter().copied());
    }
}

pub struct SerializerMap<'a> {
    ser: &'a mut Serializer,
    entries: BTreeMap<Vec<u8>, Vec<u8>>,
    current_key: Option<Vec<u8>>,
}

impl<'a> SerializerMap<'a> {
    fn new(ser: &'a mut Serializer) -> Self {
        Self {
            ser,
            entries: BTreeMap::new(),
            current_key: None,
        }
    }
}

impl<'a> serde::ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();

    type Error = crate::error::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.end();
        Ok(())
    }
}

impl<'a> serde::ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = crate::error::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.end();
        Ok(())
    }
}

impl<'a> serde::ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = crate::error::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.end();
        Ok(())
    }
}

impl<'a> serde::ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = crate::error::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.end();
        Ok(())
    }
}

impl<'a> serde::ser::SerializeMap for SerializerMap<'a> {
    type Ok = ();

    type Error = crate::error::Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> std::result::Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        if let Some(ref key) = self.current_key {
            let key = String::from_utf8_lossy(key);
            let err = anyhow!("Key {} already added", key);
            return Err(err.into());
        }

        let mut serializer = Serializer { data: Vec::new() };
        key.serialize(&mut serializer)?;
        self.current_key = Some(serializer.data);

        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> std::result::Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        if let Some(key) = self.current_key.take() {
            let mut serializer = Serializer { data: Vec::new() };
            value.serialize(&mut serializer)?;
            let value = serializer.data;

            self.entries.insert(key, value);

            return Ok(());
        }

        let err = anyhow!("Key shoud be already added");
        Err(err.into())
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        self.ser.add("d".bytes());
        for (k, v) in self.entries.into_iter() {
            self.ser.data.extend_from_slice(&k);
            self.ser.data.extend_from_slice(&v);
        }
        self.ser.add(END_CHAR.iter().copied());
        Ok(())
    }
}

impl<'a> serde::ser::SerializeStruct for SerializerMap<'a> {
    type Ok = ();

    type Error = crate::error::Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        self.serialize_entry(key, value)
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        serde::ser::SerializeMap::end(self)
    }
}

impl<'a> serde::ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();

    type Error = crate::error::Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _: &'static str,
        _: &T,
    ) -> std::result::Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }

    fn end(self) -> std::result::Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'a> serde::ser::Serializer for &'a mut Serializer {
    type Ok = ();

    type Error = crate::error::Error;

    type SerializeSeq = Self;

    type SerializeTuple = Self;

    type SerializeTupleStruct = Self;

    type SerializeTupleVariant = Self;

    type SerializeMap = SerializerMap<'a>;

    type SerializeStruct = SerializerMap<'a>;

    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v.into())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v.into())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.add(format!("i{}e", v).bytes());
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v.into())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v.into())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v.into())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.add(format!("i{}e", v).bytes());
        Ok(())
    }

    fn serialize_f32(self, _: f32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_f64(self, _: f64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_bytes(v.encode_utf8(&mut [0, 4]).as_bytes())?;
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.serialize_bytes(v.as_bytes())?;
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let mut r = Vec::new();
        r.extend_from_slice(format!("{}:", v.len()).as_bytes());
        r.extend_from_slice(v);
        self.add(r.into_iter());
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_some<T: ?Sized>(self, _: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _: &'static str,
        _: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.add("l".bytes());

        Ok(self)
    }

    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        todo!()
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        todo!()
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!()
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let serializer = SerializerMap::new(self);
        Ok(serializer)
    }

    fn serialize_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!()
    }
}
