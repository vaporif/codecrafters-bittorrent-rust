mod file;
mod peer;
mod tracker;

use std::{net::SocketAddrV4, path::PathBuf};

use crate::prelude::*;
pub use file::*;
pub use peer::*;
use rand::{distributions::Alphanumeric, Rng};
pub use tracker::*;

#[allow(unused)]
#[derive(Debug)]
pub struct Torrent {
    pub metadata: TorrentMetadataInfo,
    pieces: Vec<TorrentPiece>,
    peer_id: PeerId,
    tracker: Tracker,
    port: u16,
}

impl Torrent {
    pub fn from_file(file_path: PathBuf, port: u16) -> Result<Self> {
        let metadata = TorrentMetadataInfo::from_file(file_path)?;
        Ok(Torrent::new(metadata, port))
    }

    pub fn new(metadata: TorrentMetadataInfo, port: u16) -> Self {
        let peer_id = generate_peer_id();
        Self {
            peer_id,
            tracker: Tracker::new(&metadata.announce, port, peer_id),
            metadata,
            port,
            pieces: Vec::new(),
        }
    }

    pub async fn download(&self, output: PathBuf) -> Result<()> {
        let mut peers = self.get_peers_addresses().await?;
        if let Some(random_peer) = remove_random_element(&mut peers) {
            let mut file_bytes = Vec::with_capacity(self.metadata.info.length as usize);
            for (piece_number, _) in self.metadata.info.pieces.iter().enumerate() {
                let mut peer = Peer::connect(
                    random_peer,
                    self.peer_id,
                    self.metadata.info_hash,
                    &self.metadata.info,
                )
                .await
                .context("failed to connect to peer")?;
                let piece_data = peer.receive_file_piece(piece_number).await?;
                file_bytes.extend_from_slice(&piece_data);
            }

            tokio::fs::write(output, file_bytes)
                .await
                .context("writing torrent file")?;

            return Ok(());
        }

        bail!("Peers not found");
    }

    pub async fn get_peers_tracker_response(&self) -> Result<PeersResponse> {
        self.tracker
            .peers(&self.metadata)
            .await
            .context("getting peers")
    }

    pub async fn get_peers_addresses(&self) -> Result<Vec<SocketAddrV4>> {
        let peer_response = self.get_peers_tracker_response().await?;
        Ok(peer_response.peers)
    }
}

#[allow(unused)]
#[derive(Debug)]
struct TorrentPiece {
    hash: Bytes20,
}

pub fn generate_peer_id() -> PeerId {
    let data: Vec<_> = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .collect();

    let mut arr = [0u8; 20];
    arr.copy_from_slice(&data);
    arr.into()
}
