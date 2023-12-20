use rand::Rng;
use sha1::{Digest, Sha1};

use crate::prelude::*;

pub const BLOCK_SIZE: u32 = 16 * 1024;

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

pub fn sha1_hash(value: &[u8]) -> Bytes20 {
    let mut hasher = Sha1::new();
    hasher.update(value);
    let hash: Bytes20 = hasher.finalize().into();
    hash
}

pub fn remove_random_element<T>(vec: &mut Vec<T>) -> Option<T> {
    if vec.is_empty() {
        return None;
    }
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..vec.len());
    Some(vec.remove(index))
}
