use anyhow::{anyhow, bail, Context, Result};
use serde::{de::SeqAccess, forward_to_deserialize_any};
use std::todo;

pub fn from_str<'de, T, V>(data: T) -> Result<V>
where
    T: AsRef<str>,
    V: serde::de::Deserialize<'de>,
{
    from_bytes(data.as_ref().as_bytes())
}

pub fn from_bytes<'de, 'a, V>(data: &'a [u8]) -> Result<V>
where
    V: serde::de::Deserialize<'de>,
{
    let mut iter = data.iter().copied();
    let deserialize = Deserializer::new(&mut iter);
    V::deserialize(deserialize).context("Failed")
}

pub enum ElemenentParse {
    Integer(i64),
    String(Vec<u8>),
    List,
    Map,
    End,
}

struct Deserializer<'a, T: Iterator> {
    data: &'a mut T,
}

impl<'a, 'de, T: Iterator<Item = u8>> SeqAccess<'de> for Deserializer<'a, T> {
    type Error = crate::error::Error;

    fn next_element_seed<V>(
        &mut self,
        seed: V,
    ) -> std::result::Result<Option<V::Value>, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let res = match self.get_next_element()? {
            ElemenentParse::End => Ok(None),
            r => {
                let ele = seed
                    .deserialize(Deserializer::new(self.data))
                    .context("deserialize failure")?;
                return Ok(Some(ele));
            }
        };

        res
    }
}

impl<'a, 'de, T: Iterator<Item = u8>> serde::Deserializer<'de> for Deserializer<'a, T> {
    type Error = crate::error::Error;

    fn deserialize_any<V>(mut self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.get_next_element()? {
            ElemenentParse::Integer(v) => visitor.visit_i64(v),
            ElemenentParse::String(v) => visitor.visit_bytes(&v),
            ElemenentParse::List => self.deserialize_seq(visitor),
            ElemenentParse::Map => todo!(),
            ElemenentParse::End => todo!(),
        }
    }

    forward_to_deserialize_any! { enum i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 bytes struct map char unit unit_struct option str string }

    fn deserialize_bool<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_byte_buf<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _: &'static str,
        _: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_seq<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_seq(self)
    }

    fn deserialize_tuple<V>(self, _: usize, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple_struct<V>(
        self,
        _: &'static str,
        _: usize,
        _: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_ignored_any<V>(self, _: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }
}

impl<'a, T: Iterator<Item = u8>> Deserializer<'a, T> {
    fn new(data: &'a mut T) -> Self {
        Self { data }
    }

    fn get_int(&mut self) -> Result<i64> {
        let mut int_vec = Vec::new();

        for byte in &mut self.data {
            if byte == b'e' {
                let integer = String::from_utf8(int_vec)
                    .context("utf8 expected as char for int")?
                    .parse::<i64>()
                    .context("failed to parse")?;
                return Ok(integer);
            }

            int_vec.push(byte);
        }

        bail!("'e' character was expected");
    }

    fn get_string_bytes(&mut self, first_number: u8) -> Result<Vec<u8>> {
        let string_len = self.get_length_of_bytes(first_number)?;
        let byte_string = self.data.take(string_len).collect::<Vec<u8>>();
        let byte_string_len = byte_string.len();
        if byte_string_len != string_len {
            bail!("Unexpected len of string, Expected: {string_len}, got {byte_string_len}")
        }
        Ok(byte_string)
    }

    fn get_length_of_bytes(&mut self, first_number: u8) -> Result<usize> {
        let mut number_len = vec![first_number];

        for byte in &mut self.data {
            if byte == b':' {
                let integer = String::from_utf8(number_len)
                    .context("utf8 expected as char for int")?
                    .parse::<usize>()
                    .context("failed to parse")?;
                return Ok(integer);
            } else if !byte.is_ascii_digit() {
                bail!("number was expected, got {byte}")
            }

            number_len.push(byte);
        }

        bail!("':' character was expected");
    }

    fn get_next_element(&mut self) -> Result<ElemenentParse> {
        let next = self.data.next().ok_or(anyhow!("Empty bencode"))?;

        match next {
            x if x.is_ascii_digit() => Ok(ElemenentParse::String(self.get_string_bytes(x)?)),
            b'i' => Ok(ElemenentParse::Integer(self.get_int()?)),
            b'l' => Ok(ElemenentParse::List),
            b'd' => Ok(ElemenentParse::Map),
            b'e' => Ok(ElemenentParse::End),
            s => bail!("invalid character {}", s), // should be None
        }
    }
}

// impl<'de, T: Iterator<Item = &'de u8>> SeqAccess<'de> for Deserializer<'de, T> {
//     type Error = crate::error::Error;

//     fn next_element_seed<U>(
//         &mut self,
//         seed: U,
//     ) -> std::result::Result<Option<U::Value>, Self::Error>
//     where
//         U: serde::de::DeserializeSeed<'de>,
//     {
//         match self.get_next_element()? {
//             ElemenentParse::End => Ok(None),
//             r => {
//                 let deserialized = seed.deserialize(self);

//                 Ok(self.get_next_element())
//             }
//         }
//     }
// }
