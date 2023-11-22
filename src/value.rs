use std::{collections::BTreeMap, format, write};

use serde_bytes::ByteBuf;

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum Value {
    String(Vec<u8>),
    Integer(i64),
    List(Vec<Value>),
    Dict(BTreeMap<Vec<u8>, Value>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(chars) => write!(f, "\"{}\"", String::from_utf8_lossy(chars)),
            Value::Integer(number) => write!(f, "{}", number),
            Value::List(values) => write!(
                f,
                "[{}]",
                values
                    .iter()
                    .map(|e| format!("{}", e))
                    .reduce(|acc, e| format!("{acc},{e}"))
                    .unwrap_or_default()
            ),
            Value::Dict(dict) => {
                let dict_string = dict
                    .iter()
                    .map(|(k, v)| {
                        let key = String::from_utf8_lossy(k);
                        let value = format!("{}", v);
                        format!("\"{key}\":{value}")
                    })
                    .reduce(|acc, e| format!("{acc},{e}"))
                    .unwrap_or_default();
                write!(f, "{{{}}}", dict_string)
            }
        }
    }
}

struct ValueVisitor;

impl<'de> serde::de::Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Any bencode value")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut result = Vec::new();
        while let Some(ele) = seq.next_element()? {
            result.push(ele);
        }

        Ok(Value::List(result))
    }

    fn visit_map<A>(self, mut v: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut result = BTreeMap::new();
        while let Some((k, v)) = v.next_entry::<ByteBuf, Value>()? {
            result.insert(k.into_vec(), v);
        }

        Ok(Value::Dict(result))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Integer(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::String(Vec::from(v)))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_bytes(v.as_bytes())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_bytes(v.as_bytes())
    }
}

impl<'de> serde::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}
