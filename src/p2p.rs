mod torrent;
mod tracker;
pub use torrent::TorrentMetadataInfo;
use tracker::*;

// pub fn get_peers(metadata: &TorrentMetadataInfo) -> Result<tracker::PeersResponse> {
//     let client = Client::builder().build()?;
//     let
//     client.get(url)
// }
