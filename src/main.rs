use bencode::*;
use clap::Parser;
use cli::{Cli, Command};

use crate::{p2p::*, prelude::*};
mod bencode;
mod cli;
mod p2p;
mod prelude;

fn main() -> Result<()> {
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
            let torrent = TorrentMetadataInfo::from_file(torrent_path)?;
            let tracker = TorrentConnection::new(torrent, cli.port);
        }
    }
    Ok(())
}
