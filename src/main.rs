use bencode::*;
use clap::Parser;
use cli::{pares_peer_arg, Cli, Command};

use crate::{prelude::*, torrent::*};
mod bencode;
mod cli;
mod common;
mod prelude;
mod torrent;

#[tokio::main()]
async fn main() -> Result<()> {
    let cli = Cli::parse();
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
            let peer = Peer::from(peer, peer_id, metadata.info_hash)
                .connect()
                .await
                .context("connecting to peer")?;

            println!("Peer ID: {}", peer.connected_peer_id_hex());
        }
        Command::Download {
            torrent_path,
            piece_number,
            output,
        } => {
            let dir_path = std::path::Path::new(&output);
            if !dir_path.exists() {
                bail!("path not found");
            }
            if !dir_path.is_dir() {
                bail!("is not a dir")
            }

            let torrent = Torrent::from_file(torrent_path, cli.port).context("loading torrent")?;
            let peers = torrent.get_peers().await?;
        }
    }
    Ok(())
}
