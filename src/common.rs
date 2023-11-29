use crate::prelude::*;

pub trait WithInfoHash {
    fn info_hash(&self) -> Bytes20;
}

impl WithInfoHash for [u8; 20] {
    fn info_hash(&self) -> Bytes20 {
        *self
    }
}
#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct InfoHash(Bytes20);

impl From<Bytes20> for InfoHash {
    fn from(value: Bytes20) -> Self {
        InfoHash(value)
    }
}

impl From<InfoHash> for Bytes20 {
    fn from(value: InfoHash) -> Self {
        value.0
    }
}
