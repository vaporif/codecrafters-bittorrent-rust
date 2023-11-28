use crate::prelude::*;

pub trait WithInfoHash {
    fn info_hash(&self) -> Bytes20;
}

impl WithInfoHash for [u8; 20] {
    fn info_hash(&self) -> Bytes20 {
        *self
    }
}
