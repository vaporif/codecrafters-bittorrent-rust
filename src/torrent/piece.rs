use rand::Rng;

use crate::prelude::*;
use std::{cmp::Ordering, collections::HashSet, net::SocketAddrV4, usize};

use super::{Peer, TorrentInfo};
#[derive(Debug, PartialEq, Eq)]
pub struct Piece {
    peers: HashSet<SocketAddrV4>,
    piece_index: usize,
    hash: Vec<u8>,
}

#[allow(dead_code)]
pub struct PieceBlock {
    pub piece_index: u32,
    pub block_offset: u32,
    pub block_size: u32,
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

        Ok(Self {
            peers,
            piece_index,
            hash: hash.clone(),
        })
    }

    // TODO: get rid from torrent_info
    pub fn piece_blocks(
        &self,
        up_to_piece_size: u32,
        torrent_info: &TorrentInfo,
    ) -> Vec<PieceBlock> {
        torrent_info.piece_blocks(self.piece_index, up_to_piece_size)
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

impl TorrentInfo {
    fn piece_blocks(&self, piece_index: usize, up_to_piece_size: u32) -> Vec<PieceBlock> {
        let piece_index = piece_index as u32;
        trace!(
            "length: {}, piece_length: {}, number of pieces: {}",
            self.length,
            self.piece_length,
            self.pieces.len()
        );
        let number_of_pieces = self.pieces.len();

        let BlocksInfo {
            block_count,
            last_block_size,
        } = calc_block_size(
            piece_index,
            self.length,
            self.piece_length,
            number_of_pieces,
        );

        (0..block_count)
            .map(|index| {
                let is_last_block = index == block_count - 1;
                let begin = index as u32 * up_to_piece_size;
                let block_size = if is_last_block {
                    last_block_size
                } else {
                    up_to_piece_size
                };
                PieceBlock {
                    piece_index,
                    block_offset: begin,
                    block_size,
                }
            })
            .collect()
    }
}

fn calc_block_size(
    piece_index: u32,
    length: usize,
    piece_length: usize,
    number_of_pieces: usize,
) -> BlocksInfo {
    let indexes_of_pieces = number_of_pieces - 1;
    let full_pieces_count = number_of_pieces - 1;
    let last_piece_size = if number_of_pieces == 1 {
        piece_length
    } else {
        length - (full_pieces_count * piece_length)
    };

    let is_last_piece = piece_index == indexes_of_pieces as u32;

    let current_piece_length = if is_last_piece {
        last_piece_size
    } else {
        piece_length
    };

    let block_count = (current_piece_length as f32 / BLOCK_SIZE as f32).ceil() as usize;

    trace!("bloc count: {block_count}");

    let full_blocks = block_count - 1;

    let last_block_size: u32 = current_piece_length as u32 - BLOCK_SIZE * full_blocks as u32;
    trace!("last block size {last_block_size}");

    BlocksInfo {
        block_count,
        last_block_size,
    }
}

struct BlocksInfo {
    block_count: usize,
    last_block_size: u32,
}
