use std::{assert_eq, net::SocketAddrV4};

use bytes::{Buf, BufMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_stream::StreamExt;
use tokio_util::codec::{Decoder, Encoder, Framed};

use crate::prelude::*;

const BITTORRENT_PROTOCOL: &[u8; 19] = b"BitTorrent protocol";
const BITTORRENT_PROTOCOL_LENGTH: u8 = BITTORRENT_PROTOCOL.len() as u8;
const HANDSHAKE_MEM_SIZE: usize = std::mem::size_of::<Handshake>();

#[repr(C)]
pub struct Handshake {
    pub length: u8,
    pub protocol: [u8; 19],
    pub reserved: [u8; 8],
    pub info_hash: TorrentInfoHash,
    pub peer_id: PeerId,
}

impl Handshake {
    pub fn new(info_hash: TorrentInfoHash, peer_id: PeerId) -> Self {
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

#[allow(dead_code)]
#[derive(Debug)]
enum PeerMessage {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u8),
    Bitfield(Vec<u8>),
    Request,
    Piece,
    Cancel,
}

impl PeerMessage {
    fn new(message_id: u8, payload: Option<Vec<u8>>) -> Result<PeerMessage> {
        let message = match message_id {
            0 => PeerMessage::Choke,
            1 => PeerMessage::Unchoke,
            2 => PeerMessage::Interested,
            3 => PeerMessage::NotInterested,
            4 => PeerMessage::Have(payload.unwrap()[0]),
            5 => PeerMessage::Bitfield(payload.unwrap()),
            6 => PeerMessage::Request,
            7 => PeerMessage::Piece,
            8 => PeerMessage::Cancel,
            _ => bail!("Unknown message id {message_id}"),
        };
        Ok(message)
    }

    fn get_message_bytes(self) -> Vec<u8> {
        match self {
            PeerMessage::Have(byte) => vec![byte],
            PeerMessage::Bitfield(vec) => vec,
            _ => Vec::new(),
        }
    }

    fn get_message_id(&self) -> u8 {
        match self {
            PeerMessage::Choke => 1,
            PeerMessage::Unchoke => 2,
            PeerMessage::Interested => 3,
            PeerMessage::NotInterested => 4,
            PeerMessage::Have(_) => 5,
            PeerMessage::Bitfield(_) => 6,
            PeerMessage::Request => 7,
            PeerMessage::Piece => 8,
            PeerMessage::Cancel => 9,
        }
    }
}

struct PeerProtocolFramer;

const PEER_MESSAGE_LENGTH: usize = 4;

impl Decoder for PeerProtocolFramer {
    type Item = PeerMessage;

    type Error = anyhow::Error;

    fn decode(
        &mut self,
        src: &mut bytes::BytesMut,
    ) -> std::result::Result<Option<Self::Item>, Self::Error> {
        if src.len() < PEER_MESSAGE_LENGTH {
            return Ok(None);
        }

        let mut length = [0u8; 4];
        length.copy_from_slice(&src[..PEER_MESSAGE_LENGTH]);

        let length = u32::from_be_bytes(length) as usize;
        // NOTE: heartbeat
        if length == 0 {
            src.advance(PEER_MESSAGE_LENGTH);
        }

        if src.len() < PEER_MESSAGE_LENGTH + 1 + length {
            return Ok(None);
        }

        let payload = if src.len() > 5 {
            Some(src[5..length + 4].to_vec())
        } else {
            None
        };

        let message_id = src[4];
        let message = PeerMessage::new(message_id, payload).context("Peer message parse")?;
        src.advance(4 + length);
        Ok(Some(message))
    }
}
impl Encoder<PeerMessage> for PeerProtocolFramer {
    type Error = anyhow::Error;

    fn encode(
        &mut self,
        item: PeerMessage,
        dst: &mut bytes::BytesMut,
    ) -> std::result::Result<(), Self::Error> {
        let message_id = item.get_message_id();
        let payload_bytes = item.get_message_bytes();
        let length = (payload_bytes.len() + 1).to_be_bytes();
        dst.extend_from_slice(&length);
        dst.put_u8(message_id);
        dst.extend_from_slice(&payload_bytes);

        Ok(())
    }
}

pub struct Peer {
    socket_addr: SocketAddrV4,
    peer_id: PeerId,
    torrent_info_hash: TorrentInfoHash,
}

#[allow(unused)]
pub struct PeerConnected {
    socket_addr: SocketAddrV4,
    peer_id: PeerId,
    remote_peer_id: PeerId,
    stream: Framed<TcpStream, PeerProtocolFramer>,
    torrent_info_hash: TorrentInfoHash,
}

impl Peer {
    pub fn from(
        socket_addr: SocketAddrV4,
        peer_id: PeerId,
        torrent_info_hash: TorrentInfoHash,
    ) -> Peer {
        Peer {
            socket_addr,
            peer_id,
            torrent_info_hash,
        }
    }

    pub async fn connect(self) -> Result<PeerConnected> {
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
        })
    }
}

impl PeerConnected {
    pub async fn receive_file(&mut self) -> Result<()> {
        let bitfield = self
            .stream
            .next()
            .await
            .context("receiving new message")?
            .context("receiving bitfield")?;
        assert_eq!(5, bitfield.get_message_id());
        Ok(())
    }

    pub fn connected_peer_id_hex(&self) -> String {
        let remote_peer_id: Bytes20 = self.remote_peer_id.into();
        hex::encode(remote_peer_id)
    }
}
