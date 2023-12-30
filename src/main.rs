use std::collections::HashSet;

use bencode::*;
use clap::Parser;
use cli::{pares_peer_arg, Cli, Command};

use tracing_subscriber::{prelude::*, EnvFilter};

use crate::{prelude::*, torrent::*};
mod bencode;
mod cli;
mod common;

mod prelude;
mod torrent;

fn init_tracing(tokio_console: bool) {
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env());

    if tokio_console {
        subscriber.with(console_subscriber::spawn()).init();
        trace!("tokio console enabled");
    } else {
        subscriber.init();
    };
}

#[tokio::main()]
#[allow(unused)]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.tokio_console);

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
            let torrent = Torrent::from_file(torrent_path, cli.port, cli.max_peers)
                .context("loading torrent")?;
            let peers = torrent.get_peers_tracker_response().await?;
            println!("{}", peers);
        }
        Command::Handshake { torrent_path, peer } => {
            let peer = pares_peer_arg(&peer).context("parsing peer param")?;
            let metadata = TorrentMetadataInfo::from_file(torrent_path)?;
            let peer_id = generate_peer_id();
            let peer_id = Peer::handshake(peer, peer_id, metadata.info_hash, &metadata.info)
                .await
                .context("connecting to peer")?;

            let remote_peer_id: Bytes20 = peer_id.into();
            let remote_peer_id = hex::encode(remote_peer_id);
            println!("Peer ID: {}", remote_peer_id);
        }
        Command::DownloadPiece {
            torrent_path,
            piece_number,
            output,
        } => {
            let dir_path = std::path::Path::new(&output);

            let torrent = Torrent::from_file(torrent_path, cli.port, cli.max_peers)
                .context("loading torrent")?;
            let mut peers = torrent.get_peers_addresses().await?;
            // nvm, hacking this in post download refactoring
            let peer_hash_sets: HashSet<_> = peers.iter().copied().collect();
            if let Some(random_peer) = remove_random_element(&mut peers) {
                let peer_id = generate_peer_id();
                let mut peer = Peer::connect(
                    random_peer,
                    peer_id,
                    torrent.metadata.info_hash,
                    &torrent.metadata.info,
                )
                .await
                .context("connecting to peer")?;

                let piece = Piece::new(piece_number, &torrent.metadata.info, peer_hash_sets)
                    .context("piece construction")?;

                let piece_data = peer
                    .receive_file_piece(
                        piece_number,
                        piece.piece_blocks(BLOCK_SIZE, &torrent.metadata.info),
                    )
                    .await?;

                std::fs::write(dir_path, piece_data).context("failed to save piece")?;
            } else {
                bail!("No peers")
            }
        }
        Command::Download {
            torrent_path,
            output,
        } => {
            let dir_path = std::path::Path::new(&output);

            let mut torrent = Torrent::from_file(torrent_path, cli.port, cli.max_peers)
                .context("loading torrent")?;
            torrent.download(output).await?;
        }
    }
    Ok(())
}
