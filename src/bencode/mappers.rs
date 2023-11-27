use std::{
    fmt,
    net::{Ipv4Addr, SocketAddrV4},
};

use super::prelude::*;
use reqwest::Url;

pub fn deserialize_hashes<'de, D>(deserializer: D) -> Result<Vec<Vec<u8>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserializer.deserialize_byte_buf(HashesVisitor)
}

pub fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserializer.deserialize_str(UrlVisitor)
}

pub fn deserialize_ips<'de, D>(deserializer: D) -> Result<Vec<SocketAddrV4>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserializer.deserialize_bytes(IpsVisitor)
}

pub fn bytes_serialize<S>(x: &[Vec<u8>], s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let bytes: Vec<u8> = x.iter().flatten().copied().collect();
    s.serialize_bytes(&bytes)
}

struct UrlVisitor;

impl<'de> Visitor<'de> for UrlVisitor {
    type Value = Url;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid URL string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Url::parse(value).map_err(E::custom)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let value = String::from_utf8_lossy(v);
        Url::parse(&value).map_err(E::custom)
    }
}

struct HashesVisitor;

impl<'de> Visitor<'de> for HashesVisitor {
    type Value = Vec<Vec<u8>>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid Vec string")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut hashes = Vec::new();
        for hash in v.chunks(20) {
            hashes.push(hash.into())
        }

        Ok(hashes)
    }
}

struct IpsVisitor;

impl<'de> Visitor<'de> for IpsVisitor {
    type Value = Vec<SocketAddrV4>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid URL string")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let ips = v
            .chunks_exact(6)
            .map(|f| {
                let ip = Ipv4Addr::new(f[0], f[1], f[2], f[3]);
                let port = ((f[4] as u16) << 8) | f[5] as u16;
                SocketAddrV4::new(ip, port)
            })
            .collect();
        Ok(ips)
    }
}
