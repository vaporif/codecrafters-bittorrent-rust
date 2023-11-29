use core::fmt;
use std::{assert_eq, fmt::Debug, net::SocketAddrV4, time::Duration};

use bytes::{Buf, BufMut};
use futures::{sink::SinkExt, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::codec::{Decoder, Encoder, Framed};

use crate::prelude::*;

use super::TorrentInfo;

const BITTORRENT_PROTOCOL: &[u8; 19] = b"BitTorrent protocol";
const BITTORRENT_PROTOCOL_LENGTH: u8 = BITTORRENT_PROTOCOL.len() as u8;
const HANDSHAKE_MEM_SIZE: usize = std::mem::size_of::<Handshake>();

#[repr(C)]
pub struct Handshake {
    pub length: u8,
    pub protocol: [u8; 19],
    pub reserved: [u8; 8],
    pub info_hash: InfoHash,
    pub peer_id: PeerId,
}

impl Handshake {
    pub fn new(info_hash: InfoHash, peer_id: PeerId) -> Self {
        Self {
            length: BITTORRENT_PROTOCOL_LENGTH,
            protocol: BITTORRENT_PROTOCOL.to_owned(),
            reserved: [0; 8],
            info_hash,
            peer_id,
        }
    }

    pub fn serialize(&self) -> [u8; HANDSHAKE_MEM_SIZE] {
        let pointer_to_serialized = self as *const Handshake as *const [u8; HANDSHAKE_MEM_SIZE];
        unsafe { *pointer_to_serialized }
    }

    pub fn deserialize(data: [u8; 68]) -> Result<Self> {
        let length = data[0];

        if length != BITTORRENT_PROTOCOL_LENGTH {
            bail!("Bittorrent length is expected {BITTORRENT_PROTOCOL_LENGTH} but got {length}")
        }

        // unsafe { data.as_ptr().cast() as Handshake }
        let deserialized = Self {
            length,
            protocol: {
                let mut protocol = [0; 19];
                protocol.copy_from_slice(&data[1..20]);
                protocol
            },
            reserved: {
                let mut reserved = [0; 8];
                reserved.copy_from_slice(&data[20..28]);
                reserved
            },
            info_hash: {
                let mut info_hash = [0; 20];
                info_hash.copy_from_slice(&data[28..48]);
                info_hash.into()
            },
            peer_id: {
                let mut peer_id = [0; 20];
                peer_id.copy_from_slice(&data[48..68]);
                peer_id.into()
            },
        };

        Ok(deserialized)
    }
}

type AvailablePieces = Vec<u8>;

#[allow(dead_code)]
#[derive(Debug)]
enum PeerMessage {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u8),
    Bitfield(AvailablePieces),
    Request(RequestBlock),
    Piece(ReceivedBlock),
    Cancel,
    Heartbeat,
}

#[allow(dead_code)]
struct RequestBlock {
    index: [u8; 4],
    begin: [u8; 4],
    length: [u8; 4],
}

impl RequestBlock {
    fn new(index: u32, begin: u32, length: u32) -> Self {
        RequestBlock {
            index: index.to_be_bytes(),
            begin: begin.to_be_bytes(),
            length: length.to_be_bytes(),
        }
    }

    fn into_vec(self) -> Vec<u8> {
        vec![self.index, self.begin, self.length]
            .into_iter()
            .flatten()
            .collect()
    }
}

impl Debug for RequestBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestBlock")
            .field("index", &u32::from_be_bytes(self.index))
            .field("begin", &u32::from_be_bytes(self.begin))
            .field("length", &u32::from_be_bytes(self.length))
            .finish()
    }
}

impl From<&[u8]> for RequestBlock {
    fn from(value: &[u8]) -> Self {
        RequestBlock {
            index: {
                let mut index = [0; 4];
                index.copy_from_slice(&value[..4]);
                index
            },
            begin: {
                let mut begin = [0; 4];
                begin.copy_from_slice(&value[4..9]);
                begin
            },
            length: {
                let mut length = [0; 4];
                length.copy_from_slice(&value[9..12]);
                length
            },
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct ReceivedBlock {
    index: [u8; 4],
    begin: [u8; 4],
    block: Vec<u8>,
}

impl From<&[u8]> for ReceivedBlock {
    fn from(value: &[u8]) -> Self {
        ReceivedBlock {
            index: {
                let mut index = [0; 4];
                index.copy_from_slice(&value[..4]);
                index
            },
            begin: {
                let mut begin = [0; 4];
                begin.copy_from_slice(&value[4..8]);
                begin
            },
            block: value[8..].to_vec(),
        }
    }
}

impl ReceivedBlock {
    fn into_vec(self) -> Vec<u8> {
        vec![
            self.index.as_slice(),
            self.begin.as_slice(),
            self.block.as_slice(),
        ]
        .into_iter()
        .flatten()
        .copied()
        .collect()
    }
}

impl PeerMessage {
    fn new(message_id: u8, payload: Option<Vec<u8>>) -> Result<PeerMessage> {
        let message = match message_id {
            0 => PeerMessage::Choke,
            1 => PeerMessage::Unchoke,
            2 => PeerMessage::Interested,
            3 => PeerMessage::NotInterested,
            4 => PeerMessage::Have(payload.context("payload expected")?[0]),
            5 => PeerMessage::Bitfield(payload.context("payload expected")?),
            6 => PeerMessage::Request(payload.context("payload expected")?.as_slice().into()),
            7 => PeerMessage::Piece(payload.context("payload expected")?.as_slice().into()),
            8 => PeerMessage::Cancel,
            _ => bail!("Unknown message id {message_id}"),
        };
        Ok(message)
    }

    fn get_message_bytes(self) -> Vec<u8> {
        match self {
            PeerMessage::Have(byte) => vec![byte],
            PeerMessage::Request(bytes) => bytes.into_vec(),
            PeerMessage::Piece(bytes) => bytes.into_vec(),
            PeerMessage::Bitfield(vec) => vec,
            _ => Vec::new(),
        }
    }

    fn get_message_id(&self) -> Result<u8> {
        let message_id = match self {
            PeerMessage::Choke => 0,
            PeerMessage::Unchoke => 1,
            PeerMessage::Interested => 2,
            PeerMessage::NotInterested => 3,
            PeerMessage::Have(_) => 4,
            PeerMessage::Bitfield(_) => 5,
            PeerMessage::Request(_) => 6,
            PeerMessage::Piece(_) => 7,
            PeerMessage::Cancel => 8,
            PeerMessage::Heartbeat => bail!("Heartbeat has no message"),
        };

        Ok(message_id)
    }
}

impl fmt::Display for PeerMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

struct PeerProtocolFramer;

const PEER_MESSAGE_LENGTH: usize = 4;

impl Decoder for PeerProtocolFramer {
    type Item = PeerMessage;

    type Error = anyhow::Error;

    #[instrument(skip(self, src))]

    fn decode(
        &mut self,
        src: &mut bytes::BytesMut,
    ) -> std::result::Result<Option<Self::Item>, Self::Error> {
        trace!("buf len is {}", src.len());

        if src.len() < PEER_MESSAGE_LENGTH {
            return Ok(None);
        }

        let mut length = [0u8; PEER_MESSAGE_LENGTH];
        length.copy_from_slice(&src[..PEER_MESSAGE_LENGTH]);

        let length = u32::from_be_bytes(length) as usize;
        trace!("message len is {}", length);

        if length == 0 {
            src.advance(PEER_MESSAGE_LENGTH);
            return Ok(Some(PeerMessage::Heartbeat));
        }

        if src.len() < PEER_MESSAGE_LENGTH + length {
            trace!("not enough data, re-running to query more");
            return Ok(None);
        }

        let data = &src[4..length + PEER_MESSAGE_LENGTH];

        let message_id = data[0];
        trace!("message_id is {message_id}");
        let payload = if src.len() > 5 {
            Some(data[1..].to_vec())
        } else {
            None
        };

        let message = PeerMessage::new(message_id, payload).context("Peer message parse")?;
        src.advance(PEER_MESSAGE_LENGTH + length);
        Ok(Some(message))
    }
}
impl Encoder<PeerMessage> for PeerProtocolFramer {
    type Error = anyhow::Error;

    #[instrument(skip(self))]
    fn encode(
        &mut self,
        item: PeerMessage,
        dst: &mut bytes::BytesMut,
    ) -> std::result::Result<(), Self::Error> {
        if let PeerMessage::Heartbeat = item {
            dst.copy_from_slice(&[0u8; 4]);
            return Ok(());
        }

        let message_id = item.get_message_id().context("get message id")?;
        let payload_bytes = item.get_message_bytes();
        trace!("payload length {}", payload_bytes.len());
        let length = PEER_MESSAGE_LENGTH + 1 + payload_bytes.len();
        trace!("message len {length}");

        let length = length.to_be_bytes();
        dst.extend_from_slice(&length);
        dst.put_u8(message_id);
        dst.extend_from_slice(&payload_bytes);

        trace!("destination buf {:?}", dst);

        Ok(())
    }
}

pub struct Peer<'a> {
    socket_addr: SocketAddrV4,
    peer_id: PeerId,
    torrent_info_hash: InfoHash,
    torrent_info: &'a TorrentInfo,
}

#[allow(unused)]
pub struct PeerConnected<'a> {
    socket_addr: SocketAddrV4,
    peer_id: PeerId,
    remote_peer_id: PeerId,
    stream: Framed<TcpStream, PeerProtocolFramer>,
    torrent_info_hash: InfoHash,
    torrent_info: &'a TorrentInfo,
}

impl<'a> Peer<'a> {
    pub fn from(
        socket_addr: SocketAddrV4,
        peer_id: PeerId,
        torrent_info_hash: InfoHash,
        torrent_info: &'a TorrentInfo,
    ) -> Peer {
        Peer {
            socket_addr,
            peer_id,
            torrent_info_hash,
            torrent_info,
        }
    }

    #[instrument(skip_all)]
    pub async fn connect(self) -> Result<PeerConnected<'a>> {
        let mut stream = TcpStream::connect(self.socket_addr)
            .await
            .context("establishing connection")?;
        let handshake = Handshake::new(self.torrent_info_hash, self.peer_id);
        stream
            .write_all(&handshake.serialize())
            .await
            .context("Send handshake")?;

        let mut buffer = [0u8; HANDSHAKE_MEM_SIZE];
        stream
            .read_exact(&mut buffer)
            .await
            .with_context(|| format!("Read only {} bytes", buffer.len()))?;

        let handshake = Handshake::deserialize(buffer).context("deserialize handshake")?;

        Ok(PeerConnected {
            socket_addr: self.socket_addr,
            peer_id: self.peer_id,
            remote_peer_id: handshake.peer_id,
            stream: Framed::new(stream, PeerProtocolFramer),
            torrent_info_hash: self.torrent_info_hash,
            torrent_info: self.torrent_info,
        })
    }
}

// NOTE: 16 KB Big endian
const BLOCK_SIZE: u32 = 16 * 1024;
impl TorrentInfo {
    #[instrument(skip(self))]
    fn calc_blocks(&self, piece_num: usize) -> Vec<RequestBlock> {
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
        } = calc_block_size(piece_num, self.length, self.piece_length, number_of_pieces);

        (0..block_count)
            .map(|index| {
                let is_last_block = index == block_count - 1;
                let begin = index as u32 * BLOCK_SIZE;
                let block_size = if is_last_block {
                    last_block_size
                } else {
                    BLOCK_SIZE
                };
                RequestBlock::new(piece_num as u32, begin, block_size)
            })
            .collect()
    }
}

fn calc_block_size(
    piece_num: usize,
    length: u64,
    piece_length: u64,
    number_of_pieces: usize,
) -> BlocksInfo {
    let indexes_of_pieces = number_of_pieces - 1;
    let full_pieces_count = number_of_pieces - 1;
    let last_piece_size = if number_of_pieces == 1 {
        piece_length
    } else {
        length - (full_pieces_count as u64 * piece_length)
    };

    let is_last_piece = piece_num == indexes_of_pieces;

    let current_piece_length = if is_last_piece {
        last_piece_size
    } else {
        piece_length
    };

    let block_count = (current_piece_length as f32 / BLOCK_SIZE as f32).ceil() as usize;

    trace!("bloc count: {block_count}");

    let full_blocks = block_count - 1;

    let last_block_size = (current_piece_length - (BLOCK_SIZE * full_blocks as u32) as u64) as u32;
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

#[allow(unused_variables)]
impl<'a> PeerConnected<'a> {
    #[instrument(skip(self))]
    fn get_piece_hash(&self, piece: usize) -> Result<&[u8]> {
        self.torrent_info
            .pieces
            .get(piece)
            .map(|f| f.as_slice())
            .ok_or(anyhow!("Piece not found"))
    }

    #[instrument(skip(self))]
    pub async fn receive_file_piece(&mut self, piece_num: usize) -> Result<Vec<u8>> {
        let piece_hash = self.get_piece_hash(piece_num)?;
        let received_msg = self.next_message().await?;
        let PeerMessage::Bitfield(_) = received_msg else {
            bail!("Expected type of message bitfield got {}", received_msg)
        };

        // TODO: Check if piece exists
        self.send_message(PeerMessage::Interested)
            .await
            .context("Send interested")?;

        let received_msg = self.next_message().await?;

        let PeerMessage::Unchoke = received_msg else {
            bail!("Expected type of message unchoke got {}", received_msg)
        };

        let blocks = self.torrent_info.calc_blocks(piece_num);

        let mut result = Vec::new();

        for (i, block) in blocks.into_iter().enumerate() {
            trace!("Requesting piece {piece_num} via block num {i}");
            self.send_message(PeerMessage::Request(block))
                .await
                .context("send interested {i}")?;

            let received_msg = self.next_message().await?;

            let PeerMessage::Piece(piece_data) = received_msg else {
                bail!("Expected type of message unchoke got {}", received_msg)
            };

            assert_eq!(u32::from_be_bytes(piece_data.index), piece_num as u32);

            result.extend_from_slice(&piece_data.block);
        }

        assert_eq!(
            self.torrent_info.piece_length,
            result.len() as u64,
            "Piece length does not match"
        );

        let received_hash = sha1_hash(&result);

        assert_eq!(
            self.get_piece_hash(piece_num).context("get piece hash")?,
            received_hash
        );

        Ok(result)
    }

    #[instrument(skip(self))]
    async fn send_message(&mut self, message: PeerMessage) -> Result<()> {
        self.stream
            .send(message)
            .await
            .context("peer message send")?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn next_message(&mut self) -> Result<PeerMessage> {
        loop {
            let message = tokio::time::timeout(Duration::from_secs(5), self.stream.next())
                .await
                .map(|m| m.context("stream closed")?)
                .context("timeout")?
                .context("message expected")?;
            trace!("message is {:?}", message);
            if let PeerMessage::Heartbeat = message {
                continue;
            }

            return Ok(message);
        }
    }

    pub fn connected_peer_id_hex(&self) -> String {
        let remote_peer_id: Bytes20 = self.remote_peer_id.into();
        hex::encode(remote_peer_id)
    }
}

#[cfg(test)]
mod tests {
    use std::assert_eq;

    use crate::torrent::peer::BLOCK_SIZE;

    use super::{calc_block_size, BlocksInfo};

    #[test]
    fn test_block_calc() {
        let piece_num = 0;
        let length = 820892;
        let piece_length = 262144;
        let number_of_pieces = 4;

        let BlocksInfo {
            block_count,
            last_block_size,
        } = calc_block_size(piece_num, length, piece_length, number_of_pieces);

        dbg!(block_count);
        dbg!(last_block_size);

        assert_eq!(
            piece_length as u32,
            (block_count as u32 - 1) * BLOCK_SIZE + last_block_size
        );
    }
}
