use crate::bencode::*;
use crate::prelude::*;
use reqwest::Client;
use reqwest::Url;
use serde::Deserialize;
use std::net::SocketAddrV4;
use std::usize;

use super::TorrentMetadataInfo;

#[derive(serde::Serialize)]
struct PeersRequest {
    #[serde(serialize_with = "bytes_lossy_string_serialize")]
    pub info_hash: Bytes20,
    #[serde(serialize_with = "bytes_lossy_string_serialize")]
    pub peer_id: Bytes20,
    pub port: u16,
    pub left: usize,
    pub uploaded: u64,
    pub downloaded: u64,
    pub compact: u8,
}

impl PeersRequest {
    pub fn new(torrent: &TorrentMetadataInfo, peer_id: PeerId, port: u16) -> Self {
        Self {
            info_hash: torrent.info_hash,
            peer_id: peer_id.into(),
            port,
            left: torrent.info.length,
            uploaded: 0,
            downloaded: 0,
            compact: 1,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct PeersResponse {
    pub interval: u64,
    #[serde(deserialize_with = "deserialize_ips")]
    pub peers: Vec<SocketAddrV4>,
}

#[derive(Deserialize, Debug)]
pub struct TrackerResponseFailure {
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

#[derive(Debug)]
pub struct Tracker {
    url: Url,
    port: u16,
    peer_id: PeerId,
}

impl Tracker {
    pub fn new(url: &Url, port: u16, peer_id: PeerId) -> Self {
        Self {
            url: url.clone(),
            port,
            peer_id,
        }
    }

    #[instrument(skip(self))]
    pub async fn peers(&self, torrent_metadata: &TorrentMetadataInfo) -> Result<PeersResponse> {
        let client = Client::new();
        let params = PeersRequest::new(torrent_metadata, self.peer_id, self.port);
        let response = client
            .get(self.url.clone())
            .query(&params)
            .send()
            .await
            .context("get peers list")?;
        let is_success = response.status().is_success();
        let response_bytes = response.bytes().await.context("get peers response bytes")?;

        if is_success {
            let response: PeersResponse =
                crate::from_bytes(&response_bytes).context("parse peers response")?;

            trace!("Peers response got {:?}", response);

            Ok(response)
        } else {
            let response: TrackerResponseFailure =
                crate::from_bytes(&response_bytes).context("parse peers failed response")?;

            Err(anyhow::anyhow!(response.failure_reason))
        }
    }
}
