mod file;
mod peer;
mod tracker;

use std::path::PathBuf;

use crate::prelude::*;
pub use file::*;
pub use peer::*;
use rand::{distributions::Alphanumeric, Rng};
pub use tracker::*;

#[allow(unused)]
#[derive(Debug)]
pub struct Torrent {
    metadata: TorrentMetadataInfo,
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
        let mut peers = self.get_peers().await?;
        if let Some(random_peer) = remove_random_element(&mut peers) {
            let mut peer = random_peer
                .connect()
                .await
                .context("connecting to random peer")?;

            let mut file_bytes = Vec::with_capacity(self.metadata.info.length as usize);
            for (piece_number, _) in self.metadata.info.pieces.iter().enumerate() {
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

    pub async fn get_peers(&self) -> Result<Vec<Peer>> {
        let peer_response = self.get_peers_tracker_response().await?;
        let peers: Vec<_> = peer_response
            .peers
            .into_iter()
            .map(|socket_addr| {
                Peer::from(
                    socket_addr,
                    self.peer_id,
                    self.metadata.info_hash.into(),
                    &self.metadata.info,
                )
            })
            .collect();

        Ok(peers)
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
