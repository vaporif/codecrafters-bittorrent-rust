use core::fmt;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::borrow::Borrow;
use std::writeln;

use crate::bencode::{bytes_serialize, deserialize_hashes, deserialize_url};
use crate::bencode::{from_bytes, to_bytes};
use crate::prelude::*;

use super::Bytes20;

#[derive(Deserialize)]
pub struct TorrentMetadataInfo {
    #[serde(deserialize_with = "deserialize_url")]
    pub announce: Url,
    pub info: TorrentInfo,
    #[serde(skip)]
    pub info_hash: Bytes20,
}

impl TorrentMetadataInfo {
    pub fn from_file(torrent_path: String) -> Result<TorrentMetadataInfo> {
        let torrent = std::fs::read(torrent_path).context("could not read torrent file")?;
        let mut metadata: TorrentMetadataInfo =
            from_bytes(&torrent).context("invalid torrent file")?;

        let info_bytes = to_bytes(&metadata.info).context("Failed to serialize")?;
        let mut hasher = Sha1::new();
        hasher.update(&info_bytes);
        let info_hash: Bytes20 = hasher.finalize().into();

        metadata.info_hash = info_hash;
        Ok(metadata)
    }
}

impl<T: Borrow<TorrentMetadataInfo>> super::WithInfoHash for T {
    fn info_hash(&self) -> super::Bytes20 {
        self.borrow().info_hash
    }
}

impl std::fmt::Display for TorrentMetadataInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Tracker URL: {}", self.announce)?;
        writeln!(f, "Length: {}", self.info.length)?;
        writeln!(f, "Info Hash: {}", hex::encode(self.info_hash))?;
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
    pub length: u64,
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: u64,
    #[serde(
        deserialize_with = "deserialize_hashes",
        serialize_with = "bytes_serialize"
    )]
    pub pieces: Vec<Vec<u8>>,
}
