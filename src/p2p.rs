mod peer;
mod torrent;
mod tracker;
pub use peer::*;
pub use torrent::*;
pub use tracker::*;

type Bytes20 = [u8; 20];

pub trait WithInfoHash {
    fn info_hash(&self) -> Bytes20;
}

impl WithInfoHash for [u8; 20] {
    fn info_hash(&self) -> Bytes20 {
        *self
    }
}
