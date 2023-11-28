use std::net::SocketAddrV4;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::prelude::*;

const BITTORRENT_PROTOCOL: &[u8; 19] = b"BitTorrent protocol";
const BITTORRENT_PROTOCOL_LENGTH: u8 = BITTORRENT_PROTOCOL.len() as u8;
const HANDSHAKE_MEM_SIZE: usize = std::mem::size_of::<Handshake>();

#[repr(C)]
pub struct Handshake {
    pub length: u8,
    pub protocol: [u8; 19],
    pub reserved: [u8; 8],
    pub info_hash: Bytes20,
    pub peer_id: Bytes20,
}

impl Handshake {
    pub fn new<I: WithInfoHash>(info_hash_container: &I, peer_id: Bytes20) -> Self {
        Self {
            length: BITTORRENT_PROTOCOL_LENGTH,
            protocol: BITTORRENT_PROTOCOL.to_owned(),
            reserved: [0; 8],
            info_hash: info_hash_container.info_hash(),
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
            length: data[0],
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
                peer_id
            },
        };

        Ok(deserialized)
    }
}

#[repr(u8)]
#[allow(dead_code)]
enum PeerMessageId {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
}

#[allow(unused)]
#[repr(C)]
pub struct PeerMessage {
    length: [u8; 4],
    message_id: PeerMessageId,
    payload: Vec<u8>,
}

pub struct Peer {
    socket_addr: SocketAddrV4,
    peer_id: Bytes20,
    torrent_info_hash: Bytes20,
}

#[allow(unused)]
pub struct PeerConnected {
    socket_addr: SocketAddrV4,
    peer_id: Bytes20,
    remote_peer_id: Bytes20,
    stream: TcpStream,
    torrent_info_hash: Bytes20,
}

impl Peer {
    pub fn from(socket_addr: SocketAddrV4, peer_id: Bytes20, torrent_info_hash: Bytes20) -> Peer {
        Peer {
            socket_addr,
            peer_id,
            torrent_info_hash,
        }
    }

    pub async fn connect(&self) -> Result<PeerConnected> {
        let mut stream = TcpStream::connect(self.socket_addr)
            .await
            .context("establishing connection")?;
        let handshake = Handshake::new(&self.torrent_info_hash, self.peer_id);
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
            stream,
            torrent_info_hash: self.torrent_info_hash,
        })
    }
}

impl From<PeerConnected> for Peer {
    fn from(value: PeerConnected) -> Self {
        let PeerConnected {
            socket_addr,
            peer_id,
            torrent_info_hash,
            ..
        } = value;
        Peer {
            socket_addr,
            peer_id,
            torrent_info_hash,
        }
    }
}

impl PeerConnected {
    pub fn connected_peer_id_hex(&self) -> String {
        hex::encode(self.remote_peer_id)
    }
}
