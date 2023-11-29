use std::{assert_eq, net::SocketAddrV4, println};

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

#[repr(u8)]
#[allow(dead_code)]
#[derive(Debug)]
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
    payload: Option<Box<[u8]>>,
}

const PEER_MESSAGE_LENGTH_WITH_ID: usize = 4 + 1;

impl TryFrom<[u8; PEER_MESSAGE_LENGTH_WITH_ID]> for PeerMessage {
    type Error = anyhow::Error;
    fn try_from(
        value: [u8; PEER_MESSAGE_LENGTH_WITH_ID],
    ) -> std::result::Result<Self, Self::Error> {
        let message_id: PeerMessageId = value[4].try_into().context("try_into message_id")?;
        dbg!(&message_id);
        let mut length = [0u8; 4];
        length.copy_from_slice(&value[0..4]);

        Ok(Self {
            length,
            message_id,
            payload: None,
        })
    }
}

impl PeerMessage {
    async fn read_payload(&mut self, stream: &mut TcpStream) -> Result<()> {
        if self.payload.is_some() {
            bail!("message already read")
        }
        let mut buffer = vec![0u8; self.length()];
        println!("reading payload, legth is {}", self.length());
        stream
            .read_exact(&mut buffer)
            .await
            .context("reading payload of message")?;

        println!("payload read");
        self.payload = Some(buffer.into_boxed_slice());
        Ok(())
    }

    fn length(&self) -> usize {
        u32::from_be_bytes(self.length) as usize - 1
    }
}

impl TryFrom<u8> for PeerMessageId {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        let message_id = match value {
            0 => Some(PeerMessageId::Choke),
            1 => Some(PeerMessageId::Unchoke),
            2 => Some(PeerMessageId::Interested),
            3 => Some(PeerMessageId::NotInterested),
            4 => Some(PeerMessageId::Have),
            5 => Some(PeerMessageId::Bitfield),
            6 => Some(PeerMessageId::Request),
            7 => Some(PeerMessageId::Piece),
            8 => Some(PeerMessageId::Cancel),
            _ => None, // Return None if the value doesn't correspond to any variant
        };

        message_id.ok_or(anyhow!("Unsupported message_id {value}"))
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
    stream: TcpStream,
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

    pub async fn connect(&self) -> Result<PeerConnected> {
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
            stream,
            torrent_info_hash: self.torrent_info_hash,
        })
    }
}

impl PeerConnected {
    async fn fill_message(&mut self) -> Result<PeerMessage> {
        let mut buffer = [0u8; PEER_MESSAGE_LENGTH_WITH_ID];
        self.stream
            .read_exact(&mut buffer)
            .await
            .context("read next message legth+message_id")?;
        println!("read message");

        let mut message: PeerMessage = buffer.try_into()?;
        println!("message length {}", message.length());
        match self.stream.peek(&mut buffer).await {
            Ok(n) => {
                if n > 0 {
                    println!("has {n} bytes");
                } else {
                    println!("no bytes");
                }
            }
            Err(e) => {
                println!("{}", e);
            }
        }

        message
            .read_payload(&mut self.stream)
            .await
            .context("reading payload of message")?;

        Ok(message)
    }

    pub async fn receive_file(&mut self) -> Result<()> {
        let message = self.fill_message().await?;
        assert_eq!(PeerMessageId::Bitfield as u8, message.message_id as u8);

        Ok(())
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
        let remote_peer_id: Bytes20 = self.remote_peer_id.into();
        hex::encode(remote_peer_id)
    }
}
