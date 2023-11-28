use crate::prelude::*;

pub trait WithInfoHash {
    fn info_hash(&self) -> Bytes20;
}

impl WithInfoHash for [u8; 20] {
    fn info_hash(&self) -> Bytes20 {
        *self
    }
}
#[derive(Clone, Copy)]
pub struct PeerId(Bytes20);

impl From<Bytes20> for PeerId {
    fn from(value: Bytes20) -> Self {
        PeerId(value)
    }
}

impl From<PeerId> for Bytes20 {
    fn from(value: PeerId) -> Self {
        value.0
    }
}

#[derive(Clone, Copy)]
pub struct TorrentInfoHash(Bytes20);

impl From<Bytes20> for TorrentInfoHash {
    fn from(value: Bytes20) -> Self {
        TorrentInfoHash(value)
    }
}

impl From<TorrentInfoHash> for Bytes20 {
    fn from(value: TorrentInfoHash) -> Self {
        value.0
    }
}
