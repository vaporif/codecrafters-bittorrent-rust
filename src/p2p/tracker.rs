use crate::bencode::*;
use crate::prelude::*;
use rand::{distributions::Alphanumeric, Rng};
use reqwest::Client;
use serde::Deserialize;
use std::net::SocketAddrV4;

use super::Bytes20;
use super::TorrentMetadataInfo;

#[derive(serde::Serialize)]
struct PeersRequest<'a> {
    #[serde(serialize_with = "bytes_lossy_string_serialize")]
    pub info_hash: Bytes20,
    pub peer_id: &'a str,
    pub port: u16,
    pub left: u64,
    pub uploaded: u64,
    pub downloaded: u64,
    pub compact: u8,
}

impl<'a> PeersRequest<'a> {
    pub fn new(torrent: &TorrentMetadataInfo, peer_id: &'a str, port: u16) -> Self {
        Self {
            info_hash: torrent.info_hash,
            peer_id,
            port,
            left: torrent.info.length,
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

#[derive(Deserialize)]
pub struct TorrentResponseFailure {
    #[serde(rename = "failure reason")]
    pub failure_reason: String,
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
    client: Client,
}

impl TorrentConnection {
    pub fn new(torrent: TorrentMetadataInfo, port: u16) -> Result<Self> {
        let client = Client::builder()
            .build()
            .context("Could not create client")?;
        Ok(Self {
            torrent,
            port,
            client,
            peer_id: String::from_utf8(generate_peer_id().to_vec())?,
        })
    }

    pub fn from_torrent_path(path: String, port: u16) -> Result<Self> {
        let file = TorrentMetadataInfo::from_file(path)?;
        Self::new(file, port)
    }

    pub async fn peers(&self) -> Result<PeersResponse> {
        let params = PeersRequest::new(&self.torrent, &self.peer_id, self.port);
        let response = self
            .client
            .get(self.torrent.announce.clone())
            .query(&params)
            .send()
            .await
            .context("peers request has failed")?;
        let is_success = response.status().is_success();
        let response_bytes = response.bytes().await.context("Failed to read response")?;

        if is_success {
            let response: PeersResponse =
                crate::from_bytes(&response_bytes).context("Failed to decode response stream")?;

            Ok(response)
        } else {
            let response: TorrentResponseFailure = crate::from_bytes(&response_bytes)
                .context("Failed to decode response stream in failure")?;

            Err(anyhow::anyhow!(response.failure_reason))
        }
    }
}

pub fn generate_peer_id() -> Bytes20 {
    let data: Vec<_> = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .collect();

    let mut arr = [0u8; 20];
    arr.copy_from_slice(&data);
    arr
}
