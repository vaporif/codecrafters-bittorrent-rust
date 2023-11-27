use crate::bencode::deserialize_ips;
use std::net::SocketAddrV4;

use serde::Deserialize;

pub struct PeersRequest {
    pub info_hash: String,
    pub peer_id: String,
    pub port: u16,
    pub uploaded: usize,
    pub downloaded: usize,
    pub compact: u8,
}

// impl PeersRequest {
//     pub fn
// }

#[derive(Deserialize)]
pub struct PeersResponse {
    pub interval: u64,
    #[serde(deserialize_with = "deserialize_ips")]
    pub peers: Vec<SocketAddrV4>,
}
