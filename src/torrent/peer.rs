use core::fmt;
use std::{assert_eq, fmt::Debug, format, net::SocketAddrV4, time::Duration};

use async_channel::{Receiver, Sender};
use bitvec::{order::Msb0, vec::BitVec};
use bytes::{Buf, BufMut};
use futures::{sink::SinkExt, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::codec::{Decoder, Encoder, Framed};

use crate::prelude::*;

use super::{piece::PieceBlock, TorrentInfo};

const BITTORRENT_PROTOCOL: &[u8; 19] = b"BitTorrent protocol";
const BITTORRENT_PROTOCOL_LENGTH: u8 = BITTORRENT_PROTOCOL.len() as u8;
const HANDSHAKE_MEM_SIZE: usize = std::mem::size_of::<Handshake>();

// NOTE: using alignment to spice up things
#[repr(C)]
pub struct Handshake {
    pub length: u8,
    pub protocol: [u8; 19],
    pub reserved: [u8; 8],
    pub info_hash: Bytes20,
    pub peer_id: PeerId,
}

impl Handshake {
    pub fn new(info_hash: Bytes20, peer_id: PeerId) -> Self {
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
                info_hash
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

type PiecesIndexes = Vec<u8>;

#[allow(dead_code)]
#[derive(Debug)]
enum PeerMessage {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u8),
    Bitfield(PiecesIndexes),
    Request(RequestBlock),
    Piece(ReceivedBlock),
    Cancel,
    Heartbeat,
}

impl From<PieceBlock> for RequestBlock {
    fn from(val: PieceBlock) -> Self {
        RequestBlock::new(val.piece_index, val.block_offset, val.block_size)
    }
}

#[allow(dead_code)]
struct RequestBlock {
    index: [u8; 4],
    begin: [u8; 4],
    length: [u8; 4],
}

impl Debug for RequestBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestBlock")
            .field("index", &self.index)
            .field("begin", &self.begin)
            .field("length", &self.length)
            .finish()
    }
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
pub struct ReceivedBlock {
    index: [u8; 4],
    begin: [u8; 4],
    block: Vec<u8>,
}

impl Debug for ReceivedBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let binary_vec = self.block.iter().copied().take(20).collect::<Vec<_>>();
        let binary_string = String::from_utf8_lossy(&binary_vec);
        let block_binary_repr = format!(
            "binary legth: {}, data(start): {}",
            self.block.len(),
            binary_string
        );
        f.debug_struct("RequestBlock")
            .field("index", &u32::from_be_bytes(self.index))
            .field("begin", &u32::from_be_bytes(self.begin))
            .field("block", &block_binary_repr as &dyn Debug)
            .finish()
    }
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

    #[allow(dead_code)]
    pub fn index(&self) -> u32 {
        u32::from_be_bytes(self.index)
    }

    pub fn begin(&self) -> u32 {
        u32::from_be_bytes(self.begin)
    }

    pub fn data(&self) -> &[u8] {
        &self.block
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

#[allow(dead_code)]
pub struct Peer<'a> {
    socket_addr: SocketAddrV4,
    remote_peer_id: PeerId,
    stream: PeerTcpStream,
    torrent_info_hash: Bytes20,
    torrent_info: &'a TorrentInfo,
    bitfield: bitvec::vec::BitVec<u8, Msb0>,
    chocked: bool,
}

impl Debug for Peer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Peer")
            .field("socket_addr", &self.socket_addr)
            .finish()
    }
}

impl<'a> Peer<'a> {
    #[instrument]
    pub async fn connect(
        socket_addr: SocketAddrV4,
        peer_id: PeerId,
        torrent_info_hash: Bytes20,
        torrent_info: &'a TorrentInfo,
    ) -> Result<Peer<'a>> {
        let mut stream = TcpStream::connect(socket_addr)
            .await
            .context("establishing connection")?;
        let handshake = {
            let handshake = Handshake::new(torrent_info_hash, peer_id);
            stream
                .write_all(&handshake.serialize())
                .await
                .context("Send handshake")?;

            let mut buffer = [0u8; HANDSHAKE_MEM_SIZE];
            stream
                .read_exact(&mut buffer)
                .await
                .with_context(|| format!("Read only {} bytes", buffer.len()))?;

            Handshake::deserialize(buffer).context("deserialize handshake")?
        };

        anyhow::ensure!(
            &handshake.protocol == BITTORRENT_PROTOCOL,
            "Incorrect protocol"
        );

        let mut stream = PeerTcpStream(Framed::new(stream, PeerProtocolFramer));

        let received_msg = stream.next_message().await?;
        let PeerMessage::Bitfield(bitfield_bytes) = received_msg else {
            bail!("Expected type of message bitfield got {}", received_msg)
        };

        let bitfield = BitVec::<_, Msb0>::from_vec(bitfield_bytes);

        Ok(Peer {
            socket_addr,
            remote_peer_id: handshake.peer_id,
            stream,
            torrent_info_hash,
            torrent_info,
            bitfield,
            chocked: true,
        })
    }

    pub async fn connect_with_handshake_only(
        socket_addr: SocketAddrV4,
        peer_id: PeerId,
        torrent_info_hash: Bytes20,
    ) -> Result<PeerId> {
        let mut stream = TcpStream::connect(socket_addr)
            .await
            .context("establishing connection")?;
        let handshake = {
            let handshake = Handshake::new(torrent_info_hash, peer_id);
            stream
                .write_all(&handshake.serialize())
                .await
                .context("Send handshake")?;

            let mut buffer = [0u8; HANDSHAKE_MEM_SIZE];
            stream
                .read_exact(&mut buffer)
                .await
                .with_context(|| format!("Read only {} bytes", buffer.len()))?;

            Handshake::deserialize(buffer).context("deserialize handshake")?
        };

        anyhow::ensure!(
            &handshake.protocol == BITTORRENT_PROTOCOL,
            "Incorrect protocol"
        );

        Ok(handshake.peer_id)
    }

    #[instrument(skip(self))]
    pub fn has_piece(&self, piece: usize) -> bool {
        *self.bitfield.get(piece).as_deref().unwrap_or(&false)
    }

    pub fn available_pieces(&self) -> Vec<usize> {
        (0..=self.torrent_info.pieces.len())
            .filter(|piece_number| self.has_piece(*piece_number))
            .collect()
    }

    pub fn socket_addr(&self) -> SocketAddrV4 {
        self.socket_addr
    }

    #[instrument(skip(self))]
    fn get_piece_hash(&self, piece: usize) -> Result<&[u8]> {
        self.torrent_info
            .pieces
            .get(piece)
            .map(|f| f.as_slice())
            .ok_or(anyhow!("Piece not found"))
    }

    #[instrument(skip(self, requested_block, save_block), fields(self.socket_addr = %self.socket_addr))]
    pub async fn process(
        &mut self,
        _: Sender<PieceBlock>,
        requested_block: Receiver<PieceBlock>,
        save_block: Sender<ReceivedBlock>,
    ) -> Result<PeerId> {
        if self.chocked {
            self.stream
                .send_message(PeerMessage::Interested)
                .await
                .context("Send interested")?;

            let received_msg = self.stream.next_message().await?;

            let PeerMessage::Unchoke = received_msg else {
                bail!("Expected type of message unchoke got {}", received_msg)
            };
        }

        self.chocked = false;

        while let Ok(block) = requested_block.recv().await {
            trace!("received to process {}", block.piece_index,);
            let piece_index = block.piece_index;
            let request_block = PeerMessage::Request(block.into());
            self.stream
                .send_message(request_block)
                .await
                .context("sending request message")?;

            let received_msg = self.stream.next_message().await?;

            let PeerMessage::Piece(piece_data) = received_msg else {
                bail!("Expected type of message piece got {}", received_msg)
            };

            assert_eq!(u32::from_be_bytes(piece_data.index), piece_index);

            trace!("piece downloaded");
            save_block
                .send(piece_data)
                .await
                .context("sending piece back")?;
            trace!("piece sent");
        }

        Ok(self.remote_peer_id)
    }

    #[instrument(skip(self, piece_blocks))]
    pub async fn receive_file_piece(
        &mut self,
        piece_num: usize,
        piece_blocks: Vec<PieceBlock>,
    ) -> Result<Vec<u8>> {
        if self.chocked {
            self.stream
                .send_message(PeerMessage::Interested)
                .await
                .context("Send interested")?;

            let received_msg = self.stream.next_message().await?;

            let PeerMessage::Unchoke = received_msg else {
                bail!("Expected type of message unchoke got {}", received_msg)
            };
        }

        self.chocked = false;

        let blocks_len = piece_blocks.len();
        let mut result = Vec::new();

        for (i, block) in piece_blocks.into_iter().enumerate() {
            trace!(
                "Requesting piece {piece_num} via block num {}, number of blocks {}",
                i,
                blocks_len
            );
            self.stream
                .send_message(PeerMessage::Request(block.into()))
                .await
                .context("request block {i}")?;

            let received_msg = self.stream.next_message().await?;

            let PeerMessage::Piece(piece_data) = received_msg else {
                bail!("Expected type of message unchoke got {}", received_msg)
            };

            assert_eq!(u32::from_be_bytes(piece_data.index), piece_num as u32);

            result.extend_from_slice(&piece_data.block);
        }

        let received_hash = sha1_hash(&result);

        let piece_hash = self.get_piece_hash(piece_num).context("get piece hash")?;

        anyhow::ensure!(piece_hash == received_hash, "Hash incorrect");

        Ok(result)
    }
}

struct PeerTcpStream(Framed<TcpStream, PeerProtocolFramer>);
impl PeerTcpStream {
    #[instrument(skip(self))]
    async fn next_message(&mut self) -> Result<PeerMessage> {
        loop {
            let message = tokio::time::timeout(Duration::from_secs(5), self.0.next())
                .await
                .map(|m| m.context("stream closed")?)
                .context(format!("timeout at {}", line!()))?
                .context("message expected")?;
            trace!("message is {:?}", message);
            if let PeerMessage::Heartbeat = message {
                continue;
            }

            return Ok(message);
        }
    }

    #[instrument(skip(self))]
    async fn send_message(&mut self, message: PeerMessage) -> Result<()> {
        self.0.send(message).await.context("peer message send")?;
        Ok(())
    }
}
