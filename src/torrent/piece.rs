use rand::Rng;

use crate::prelude::*;
use std::{cmp::Ordering, collections::HashSet, net::SocketAddrV4, usize};

use super::{Peer, TorrentInfo};
#[derive(PartialEq, Eq)]
pub struct Piece {
    peers: HashSet<SocketAddrV4>,
    piece_index: usize,
    hash: Vec<u8>,
}

// NOTE: Introduce randomness into generation
impl Ord for Piece {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let mut rng = rand::thread_rng();
        self.peers
            .len()
            .cmp(&other.peers.len())
            .then_with(|| match rng.gen_range(0..=2) {
                0 => Ordering::Less,
                1 => Ordering::Greater,
                _ => Ordering::Equal,
            })
            .then(self.piece_index.cmp(&other.piece_index))
    }
}

impl PartialOrd for Piece {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for Piece {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl Piece {
    #[instrument]
    pub fn new(
        piece_index: usize,
        torrent_info: &TorrentInfo,
        peers: HashSet<SocketAddrV4>,
    ) -> Result<Self> {
        let hash = torrent_info
            .pieces
            .get(piece_index)
            .context("get piece hash")?;

        // let peers = peers
        //     .iter()
        //     .filter_map(|f| {
        //         if f.has_piece(piece_number) {
        //             Some(f.socket_addr())
        //         } else {
        //             None
        //         }
        //     })
        //     .collect();

        Ok(Self {
            peers,
            piece_index,
            hash: hash.clone(),
        })
    }

    pub fn has_peers(&self) -> bool {
        !self.peers.is_empty()
    }

    pub fn peer_has_piece(&self, peer: &Peer) -> bool {
        self.peers.contains(&peer.socket_addr())
    }

    pub fn piece_index(&self) -> usize {
        self.piece_index
    }
}
