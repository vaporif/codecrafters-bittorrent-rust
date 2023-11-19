use anyhow::{anyhow, bail, Context, Result};
use std::{collections::HashMap, todo};

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum Value {
    String(Vec<u8>),
    Integer(i64),
    List(Vec<Value>),
    Dict(HashMap<Vec<u8>, Value>),
}

pub enum ElemenentParse {
    Integer(i64),
    String(Vec<u8>),
    List,
    Map,
    End,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(chars) => write!(f, "\"{}\"", String::from_utf8_lossy(chars)),
            Value::Integer(number) => write!(f, "{}", number),
            Value::List(values) => write!(
                f,
                "{}",
                values
                    .iter()
                    .map(|e| format!("{}", e))
                    .reduce(|acc, e| format!("{acc}, {e}"))
                    .unwrap_or_default()
            ),
            Value::Dict(_) => todo!(),
        }
    }
}

pub fn parse_bencode_byte_string<I>(chars: I) -> Result<Value>
where
    I: AsRef<[u8]>,
{
    let mut chars = chars.as_ref().iter();
    let mut deserializer = Deserializer::new(&mut chars);
    deserializer.get_bencode_value()
}

struct Deserializer<'de, T: Iterator> {
    data: &'de mut T,
}

impl<'de, T: Iterator<Item = &'de u8>> Deserializer<'de, T> {
    fn new(data: &'de mut T) -> Self {
        Self { data }
    }

    fn get_bencode_value(&mut self) -> Result<Value> {
        match self.get_next_element()? {
            ElemenentParse::Integer(int) => Ok(Value::Integer(int)),
            ElemenentParse::List => todo!(),
            ElemenentParse::Map => todo!(),
            ElemenentParse::End => todo!(),
            ElemenentParse::String(bytes) => Ok(Value::String(bytes)),
        }
    }

    fn get_int(&mut self) -> Result<i64> {
        let mut int_vec = Vec::new();

        for byte in &mut self.data {
            if *byte == b'e' {
                let integer = String::from_utf8(int_vec)
                    .context("utf8 expected as char for int")?
                    .parse::<i64>()
                    .context("failed to parse")?;
                return Ok(integer);
            }

            int_vec.push(*byte);
        }

        bail!("'e' character was expected");
    }

    fn get_string_bytes(&mut self, first_number: &u8) -> Result<Vec<u8>> {
        let string_len = self.get_length_of_bytes(first_number)?;
        let byte_string = self.data.take(string_len).copied().collect::<Vec<u8>>();
        let byte_string_len = byte_string.len();
        if byte_string_len != string_len {
            bail!("Unexpected len of string, Expected: {string_len}, got {byte_string_len}")
        }
        Ok(byte_string)
    }

    fn get_length_of_bytes(&mut self, first_number: &u8) -> Result<usize> {
        let mut number_len = vec![*first_number];

        for byte in &mut self.data {
            if *byte == b':' {
                let integer = String::from_utf8(number_len)
                    .context("utf8 expected as char for int")?
                    .parse::<usize>()
                    .context("failed to parse")?;
                return Ok(integer);
            } else if !byte.is_ascii_digit() {
                bail!("number was expected, got {byte}")
            }

            number_len.push(*byte);
        }

        bail!("':' character was expected");
    }

    fn get_next_element(&mut self) -> Result<ElemenentParse> {
        let next = self.data.next().ok_or(anyhow!("Empty bencode"))?;

        match next {
            x if x.is_ascii_digit() => Ok(ElemenentParse::String(self.get_string_bytes(x as &u8)?)),
            b'i' => Ok(ElemenentParse::Integer(self.get_int()?)),
            b'l' => Ok(ElemenentParse::List),
            b'd' => Ok(ElemenentParse::Map),
            b'e' => Ok(ElemenentParse::End),
            s => bail!("invalid character {}", s), // should be None
        }
    }
}
