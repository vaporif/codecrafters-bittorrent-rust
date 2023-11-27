use crate::bencode::deserialize_ips;
use crate::prelude::*;
use rand::{distributions::Alphanumeric, Rng};
use reqwest::blocking;
use serde::Deserialize;
use std::net::SocketAddrV4;

use super::TorrentMetadataInfo;

#[derive(serde::Serialize)]
struct PeersRequest<'a> {
    pub info_hash: [u8; 20],
    pub peer_id: &'a str,
    pub port: u16,
    pub uploaded: usize,
    pub downloaded: usize,
    pub compact: u8,
}

impl<'a> PeersRequest<'a> {
    pub fn new(torrent: &TorrentMetadataInfo, peer_id: &'a str, port: u16) -> Self {
        Self {
            info_hash: torrent.info_hash,
            peer_id,
            port,
            uploaded: 0,
            downloaded: 0,
            compact: 1,
        }
    }
}

#[derive(Deserialize)]
pub struct PeersResponse {
    pub interval: u64,
    #[serde(deserialize_with = "deserialize_ips")]
    pub peers: Vec<SocketAddrV4>,
}

impl std::fmt::Display for PeersResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for peer in &self.peers {
            writeln!(f, "{}", peer)?;
        }
        Ok(())
    }
}
pub struct TorrentConnection {
    torrent: TorrentMetadataInfo,
    port: u16,
    peer_id: String,
    client: blocking::Client,
}

impl TorrentConnection {
    pub fn new(torrent: TorrentMetadataInfo, port: u16) -> Result<Self> {
        let client = blocking::Client::builder()
            .build()
            .context("Could not create client")?;
        Ok(Self {
            torrent,
            port,
            client,
            peer_id: generate_peer_id(),
        })
    }

    pub fn peers(&self) -> Result<PeersResponse> {
        let params = PeersRequest::new(&self.torrent, &self.peer_id, self.port);
        let response = self
            .client
            .get(self.torrent.announce.clone())
            .query(&params)
            .send()
            .context("peers request has failed")?;

        let response = response
            .bytes()
            .context("Failed to get response byte stream")?;

        let response: PeersResponse =
            crate::from_bytes(response.as_ref()).context("Failed to decode response stream")?;

        Ok(response)
    }
}

fn generate_peer_id() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .map(char::from)
        .collect()
}
