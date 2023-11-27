use std::net::SocketAddrV4;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use super::TorrentMetadataInfo;
use crate::prelude::*;

const BITTORRENT_PROTOCOL: &[u8; 19] = b"BitTorrent protocol";

pub struct Handshake {
    pub length: u8,
    pub protocol: [u8; 19],
    pub reserved: [u8; 8],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
}

impl Handshake {
    pub fn new(torrent: &TorrentMetadataInfo, peer_id: [u8; 20]) -> Self {
        Self {
            length: 19,
            protocol: BITTORRENT_PROTOCOL.to_owned(),
            reserved: [0; 8],
            info_hash: torrent.info_hash,
            peer_id,
        }
    }

    pub fn serialize(&self) -> [u8; 68] {
        let mut array: [u8; 68] = [0; 68];

        array[0] = self.length;
        array[1..20].copy_from_slice(&self.protocol);
        array[20..28].copy_from_slice(&self.reserved);
        array[28..48].copy_from_slice(&self.info_hash);
        array[48..68].copy_from_slice(&self.peer_id);

        array
    }

    pub fn deserialize(data: [u8; 68]) -> Self {
        Self {
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
        }
    }
}

#[allow(unused)]
pub struct Peer {
    peer_ipsocket: SocketAddrV4,
    peer_id: [u8; 20],
    stream: TcpStream,
}

#[allow(unused)]
pub struct PeerConnected<'a> {
    peer_id: [u8; 20],
    connected_peer_id: [u8; 20],
    stream: TcpStream,
    torrent: &'a TorrentMetadataInfo,
}

impl Peer {
    pub async fn connect(peer_ipsocket: SocketAddrV4, peer_id: [u8; 20]) -> Result<Peer> {
        let stream = TcpStream::connect(peer_ipsocket)
            .await
            .context("connection failed")?;
        Ok(Peer {
            peer_ipsocket,
            peer_id,
            stream,
        })
    }

    pub async fn handshake(mut self, metadata: &TorrentMetadataInfo) -> Result<PeerConnected> {
        let handshake = Handshake::new(metadata, self.peer_id);
        self.stream
            .write_all(&handshake.serialize())
            .await
            .context("Failed to do handshake")?;

        let mut buffer = [0u8; 68];
        self.stream
            .read_exact(&mut buffer)
            .await
            .with_context(|| format!("Read only {} bytes", buffer.len()))?;

        let handshake = Handshake::deserialize(buffer);

        let Self {
            peer_id, stream, ..
        } = self;

        Ok(PeerConnected {
            peer_id,
            connected_peer_id: handshake.peer_id,
            stream,
            torrent: metadata,
        })
    }
}

impl<'a> PeerConnected<'a> {
    pub fn connected_peer_id_hex(&self) -> String {
        hex::encode(self.connected_peer_id)
    }
}
