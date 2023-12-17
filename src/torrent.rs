mod file;
mod peer;
mod tracker;

use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
    net::SocketAddrV4,
    path::PathBuf,
};

use crate::prelude::*;
pub use file::*;
use futures::StreamExt;
pub use peer::*;
mod piece;
use piece::*;
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
        tracing::trace!("File {:?}", metadata.info);
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

    async fn get_peers(&self, limit: usize) -> Result<Vec<Peer>> {
        let peers = self.get_peers_addresses().await?;

        let mut peers = futures::stream::iter(peers)
            .map(|f| async move {
                Peer::connect(
                    f,
                    self.peer_id,
                    self.metadata.info_hash,
                    &self.metadata.info,
                )
                .await
            })
            .buffer_unordered(5);
        let mut peers_connected = Vec::new();
        while let Some(connection) = peers.next().await {
            match connection {
                Ok(peer) => {
                    peers_connected.push(peer);
                    if peers_connected.len() >= limit {
                        break;
                    }
                }
                Err(e) => eprint!("Error {e}"),
            }
        }

        Ok(peers_connected)
    }

    // NOTE: well, just passing peers to piece
    // to filter peers with pieces would have been easier
    fn get_pieces(&self, peers: &[Peer<'_>]) -> Vec<Piece> {
        peers
            .iter()
            .flat_map(|f| {
                f.available_pieces()
                    .into_iter()
                    .map(|piece_number| (piece_number, f.socket_addr()))
            })
            .fold(
                HashMap::new(),
                |mut acc: HashMap<usize, HashSet<SocketAddrV4>>, (piece_number, socket_addr)| {
                    acc.entry(piece_number).or_default().insert(socket_addr);
                    acc
                },
            )
            .into_iter()
            .filter_map(|(k, v)| Piece::new(k, &self.metadata.info, v).ok())
            .collect()
    }

    pub async fn download(&self, output: PathBuf) -> Result<()> {
        let mut peers = self.get_peers(5).await?;
        let pieces = self.get_pieces(&peers);

        let mut heap = BinaryHeap::new();

        for piece in pieces.into_iter().filter(|f| f.has_peers()) {
            heap.push(Reverse(piece));
        }

        anyhow::ensure!(heap.len() == self.metadata.info.pieces.len());

        let mut file_bytes = vec![0u8; self.metadata.info.length];
        while let Some(piece) = heap.pop() {
            let piece = piece.0;
            let mut peers: Vec<_> = peers
                .iter_mut()
                .filter(|f| piece.peer_has_piece(f))
                .collect();
            let peer = peers.get_mut(0).unwrap();
            let piece_data = peer.receive_file_piece(piece.piece_index()).await?;

            let index = piece.piece_index() * self.metadata.info.piece_length;
            file_bytes
                .get_mut(index..index + piece_data.len())
                .context("getting slice to copy piece")?
                .copy_from_slice(&piece_data);
        }

        anyhow::ensure!(file_bytes.len() == self.metadata.info.length);
        tokio::fs::write(output, file_bytes)
            .await
            .context("writing torrent file")?;

        Ok(())
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
