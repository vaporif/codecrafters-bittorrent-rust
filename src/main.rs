use bencode::*;
use clap::Parser;
use cli::{pares_peer_arg, Cli, Command};

use crate::{p2p::*, prelude::*};
mod bencode;
mod cli;
mod p2p;
mod prelude;

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
            let value = to_bytes(value).context("Failed to encode")?;
            let value = String::from_utf8_lossy(&value);
            println!("{}", value);
        }
        Command::Peers { torrent_path } => {
            let tracker = TorrentConnection::from_torrent_path(torrent_path, cli.port)
                .context("Could not establish connection")?;

            println!("{}", tracker.peers().await?);
        }
        Command::Handshake { torrent_path, peer } => {
            let peer = pares_peer_arg(&peer).context("Parsing peer arg failed")?;
            let metadata = TorrentMetadataInfo::from_file(torrent_path)?;
            let peer_id = generate_peer_id();
            let peer = Peer::connect(peer, peer_id).await?;
            let peer = peer.handshake(&metadata).await?;

            println!("Peer ID: {}", peer.connected_peer_id_hex());
        }
    }
    Ok(())
}
