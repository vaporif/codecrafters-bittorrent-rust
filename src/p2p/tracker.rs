pub struct PeersRequest {
    pub info_hash: String,
    pub peer_id: String,
    pub port: u16,
    pub uploaded: usize,
    pub downloaded: usize,
    pub compact: u8,
}

pub struct PeersResponse {
    pub interval: u64,
    pub peers: Vec<u8>,
}
