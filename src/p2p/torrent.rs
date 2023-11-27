use core::fmt;
use reqwest::Url;
use serde::de::Error;
use serde::Serialize;
use serde::{de::Visitor, Deserialize};
use sha1::{Digest, Sha1};
use std::writeln;

use crate::bencode::{from_bytes, to_bytes};
use crate::prelude::*;

#[derive(Deserialize)]
pub struct TorrentMetadataInfo {
    #[serde(deserialize_with = "deserialize_url")]
    pub announce: Url,
    pub info: TorrentInfo,
    #[serde(skip)]
    hash: Option<String>,
}

impl TorrentMetadataInfo {
    pub fn from_file(torrent_path: String) -> Result<TorrentMetadataInfo> {
        let torrent = std::fs::read(torrent_path).context("could not read torrent file")?;
        let mut metadata: TorrentMetadataInfo =
            from_bytes(&torrent).context("invalid torrent file")?;
        metadata
            .compute_hash()
            .context("Failed to compute hash of torrent")?;
        Ok(metadata)
    }
}

impl std::fmt::Display for TorrentMetadataInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Tracker URL: {}", &self.announce)?;
        writeln!(f, "Length: {}", self.info.length)?;
        if let Some(ref hash) = self.hash {
            writeln!(f, "Info Hash: {}", hash)?;
        }

        writeln!(f, "Piece Length: {}", self.info.piece_length)?;

        f.write_str("Piece Hashes:\n")?;
        for hash in &self.info.pieces {
            writeln!(f, "{}", hex::encode(hash))?;
        }

        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
pub struct TorrentInfo {
    pub length: i64,
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: i64,
    #[serde(
        deserialize_with = "deserialize_hashes",
        serialize_with = "bytes_serialize"
    )]
    pub pieces: Vec<Vec<u8>>,
}

fn bytes_serialize<S>(x: &[Vec<u8>], s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let bytes: Vec<u8> = x.iter().flatten().copied().collect();
    s.serialize_bytes(&bytes)
}
impl TorrentMetadataInfo {
    pub fn compute_hash(&mut self) -> Result<()> {
        let info_bytes = to_bytes(&self.info).context("Failed to serialize")?;
        // println!("Serialized: {}", String::from_utf8_lossy(&info_bytes));
        let mut hasher = Sha1::new();
        hasher.update(&info_bytes);
        let hash = hex::encode(hasher.finalize());
        self.hash = Some(hash);
        Ok(())
    }
}

fn deserialize_hashes<'de, D>(deserializer: D) -> Result<Vec<Vec<u8>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserializer.deserialize_byte_buf(HashesVisitor)
}

fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
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

struct HashesVisitor;

impl<'de> Visitor<'de> for HashesVisitor {
    type Value = Vec<Vec<u8>>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid Vec string")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let mut hashes = Vec::new();
        for hash in v.chunks(20) {
            hashes.push(hash.into())
        }

        Ok(hashes)
    }
}
