use bencode::*;
use clap::Parser;
use cli::{pares_peer_arg, Cli, Command};
use rand::Rng;

use crate::{prelude::*, torrent::*};
mod bencode;
mod cli;
mod common;
mod prelude;
mod torrent;

#[tokio::main()]
#[allow(unused)]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt::init();
    match cli.command {
        Command::Decode { bencoded_value } => {
            let decoded: Value = from_str(bencoded_value)?;
            println!("{}", decoded);
        }
        Command::Info { torrent_path } => {
            let metadata = TorrentMetadataInfo::from_file(torrent_path)?;
            println!("{}", metadata);
        }
        Command::Encode { value } => {
            let value = to_bytes(value).context("encoding to bencode")?;
            let value = String::from_utf8_lossy(&value);
            println!("{}", value);
        }
        Command::Peers { torrent_path } => {
            let torrent = Torrent::from_file(torrent_path, cli.port).context("loading torrent")?;
            let peers = torrent.get_peers_tracker_response().await?;
            println!("{}", peers);
        }
        Command::Handshake { torrent_path, peer } => {
            let peer = pares_peer_arg(&peer).context("parsing peer param")?;
            let metadata = TorrentMetadataInfo::from_file(torrent_path)?;
            let peer_id = generate_peer_id();
            let peer = Peer::from(
                peer,
                peer_id,
                metadata.info_hash.into(),
                metadata.info.pieces.as_slice(),
            )
            .connect()
            .await
            .context("connecting to peer")?;

            println!("Peer ID: {}", peer.connected_peer_id_hex());
        }
        Command::DownloadPiece {
            torrent_path,
            piece_number,
            output,
        } => {
            let dir_path = std::path::Path::new(&output);

            let torrent = Torrent::from_file(torrent_path, cli.port).context("loading torrent")?;
            let mut peers = torrent.get_peers().await?;
            if let Some(random_peer) = remove_random_element(&mut peers) {
                let mut peer = random_peer
                    .connect()
                    .await
                    .context("connecting to random peer")?;

                peer.receive_file(piece_number).await?;
            } else {
                bail!("No peers")
            }
        }
    }
    Ok(())
}

fn remove_random_element<T>(vec: &mut Vec<T>) -> Option<T> {
    if vec.is_empty() {
        return None;
    }
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..vec.len());
    Some(vec.remove(index))
}
