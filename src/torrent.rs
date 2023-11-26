use core::fmt;
use reqwest::Url;
use serde::de::Error;
use serde::{de::Visitor, Deserialize};
use std::write;

#[derive(Deserialize)]
pub struct TorrentMetadataInfo {
    #[serde(deserialize_with = "deserialize_url")]
    pub announce: Url,
    pub info: TorrentInfo,
}

impl std::fmt::Display for TorrentMetadataInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tracker URL: {}", &self.announce)?;
        writeln!(f, "Length: {}", self.info.length)
    }
}

#[derive(Deserialize)]
pub struct TorrentInfo {
    pub length: i64,
    #[serde(deserialize_with = "deserialize_vec_u8")]
    pub name: Vec<u8>,
    #[serde(rename = "piece length")]
    pub piece_length: i64,
    #[serde(deserialize_with = "deserialize_vec_u8")]
    pub pieces: Vec<u8>,
}

pub fn deserialize_vec_u8<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserializer.deserialize_byte_buf(VecVisitor)
}

pub fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserializer.deserialize_str(UrlVisitor)
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
        E: Error,
    {
        let value = String::from_utf8_lossy(v);
        Url::parse(&value).map_err(E::custom)
    }
}

struct VecVisitor;

impl<'de> Visitor<'de> for VecVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid Vec string")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v.to_vec())
    }
}
