mod file;
mod peer;
mod tracker;

use std::{
    cell::RefCell,
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
    fs::OpenOptions,
    io::{Seek, SeekFrom, Write},
    net::SocketAddrV4,
    path::PathBuf,
};

use crate::prelude::*;
pub use file::*;

use futures_util::stream::FuturesUnordered;
pub use peer::*;
mod piece;
use futures::{Future, StreamExt};
pub use piece::*;
use rand::{distributions::Alphanumeric, Rng};
pub use tracker::*;

#[allow(unused)]
#[derive(Debug)]
pub struct Torrent {
    pub metadata: TorrentMetadataInfo,
    download_queue: RefCell<BinaryHeap<Reverse<Piece>>>,
    peer_id: PeerId,
    tracker: Tracker,
    port: u16,
    max_peers: u8,
}

impl Torrent {
    pub fn from_file(file_path: PathBuf, port: u16, max_peers: u8) -> Result<Self> {
        let metadata = TorrentMetadataInfo::from_file(file_path)?;
        tracing::trace!("File {:?}", metadata.info);
        Ok(Torrent::new(metadata, port, max_peers))
    }

    pub fn new(metadata: TorrentMetadataInfo, port: u16, max_peers: u8) -> Self {
        let peer_id = generate_peer_id();
        Self {
            max_peers,
            peer_id,
            tracker: Tracker::new(&metadata.announce, port, peer_id),
            metadata,
            port,
            download_queue: RefCell::new(BinaryHeap::new()),
        }
    }

    async fn get_peers(&self, limit: u8) -> Result<Vec<Peer>> {
        let peers = self.get_peers_addresses().await?;
        let limit = limit as usize;
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
            .buffer_unordered(limit);
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
    fn get_pieces(&self, peers: &[Peer]) -> Vec<Piece> {
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

    #[instrument(skip(self, peers, saved_block, save_file_piece))]
    async fn cooperative_download_piece<T: Future<Output = Result<PeerId>>>(
        &self,
        piece_index: usize,
        piece_length: usize,
        peers: &mut FuturesUnordered<T>,
        saved_block: async_channel::Receiver<ReceivedBlock>,
        save_file_piece: tokio::sync::mpsc::Sender<(u64, Vec<u8>)>,
    ) -> Result<()> {
        let average_piece_length = self.metadata.info.piece_length;

        let mut bytes_written = 0;
        let mut piece_blocks = vec![0u8; piece_length];
        loop {
            trace!("loop");
            tokio::select! {
            peer_id = peers.next() => {
                trace!("peer future");
                match peer_id {
                    Some(peer_id) => {
                        trace!("peer response {:?}", peer_id);
                    },
                    None => {
                        trace!("peers exited");
                    },
                }
            }
            block = saved_block.recv() => {
                    trace!("saved_block channel message {:?}", block);
                    match block {
                        Ok(block) => {
                            let begin = block.begin() as usize;
                            piece_blocks
                                .get_mut(begin..begin + block.data().len())
                                .context("getting slice to copy piece")?
                                .copy_from_slice(block.data());

                            bytes_written += block.data().len();
                            if bytes_written == piece_length {
                                save_file_piece.send(((piece_index * average_piece_length) as u64, piece_blocks)).await.expect("sent");
                                break;
                            }
                        },
                        Err(err) => {
                            tracing::error!("done recv() failed, {:?}", err);
                            break;
                        },
                    }
                }
            }
        }

        // let mut file = OpenOptions::new()
        //     .read(true)
        //     .write(true)
        //     .create(true)
        //     .open(output)
        //     .context("opening file")?;
        // file.set_len(self.metadata.info.length as u64)
        //     .context("setting file size")?;
        // file.seek(SeekFrom::Start((piece_index * average_piece_length) as u64))
        //     .context("seeking file")?;
        // file.write_all(&piece_blocks).context("writing file")?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn download(&mut self, output: PathBuf) -> Result<()> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(output)
            .context("opening file")?;

        file.set_len(self.metadata.info.length as u64)
            .context("setting file size")?;
        let (send_file_piece, mut receive_file_piece) =
            tokio::sync::mpsc::channel::<(u64, Vec<u8>)>(self.metadata.info.pieces.len() / 2);
        let num_pieces = self.metadata.info.pieces.len();
        let file_handle = tokio::task::spawn_blocking(move || -> Result<()> {
            let mut num_pieces_saved = 0;
            while let Some((index, data)) = receive_file_piece.blocking_recv() {
                trace!("saving {}", index);
                file.seek(SeekFrom::Start(index)).context("seeking file")?;
                file.write_all(&data).context("writing file")?;
                trace!("saved");
                num_pieces_saved += 1;
                if num_pieces_saved == num_pieces {
                    break;
                }
            }
            Ok(())
        });

        let mut peers = self.get_peers(self.max_peers).await?;
        let pieces = self.get_pieces(&peers);

        for piece in pieces.into_iter().filter(|f| f.has_peers()) {
            self.download_queue.borrow_mut().push(Reverse(piece));
        }

        anyhow::ensure!(self.download_queue.borrow().len() == self.metadata.info.pieces.len());

        while let Some(piece) = self.download_queue.borrow_mut().pop() {
            let piece = piece.0;
            trace!("downloading piece {}", piece.piece_index());
            let blocks = piece.piece_blocks(BLOCK_SIZE, &self.metadata.info);
            let total_piece_size = blocks.iter().map(|f| f.block_size).sum::<u32>() as usize;
            let (request_block, requested_block) = async_channel::bounded(blocks.len());
            let (save_block, saved_block) = async_channel::bounded(blocks.len());
            for block in blocks {
                request_block
                    .send(block)
                    .await
                    .context("sending blocks to process")?;
            }

            trace!("blocks sent to process");
            let mut peers_interacting = FuturesUnordered::new();
            for peer in peers.iter_mut().filter(|peer| piece.peer_has_piece(peer)) {
                let request_block = request_block.clone();
                let requested_block = requested_block.clone();
                let saved_block = save_block.clone();

                peers_interacting.push(peer.process(request_block, requested_block, saved_block));
            }

            trace!("futures created");

            let send_file_piece = send_file_piece.clone();

            self.cooperative_download_piece(
                piece.piece_index(),
                total_piece_size,
                &mut peers_interacting,
                saved_block,
                send_file_piece,
            )
            .await
            .context("saving file")?;
        }

        file_handle.await.context("savig file")??;

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
