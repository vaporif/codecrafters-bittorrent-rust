use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Command};

use crate::{de::from_str, value::Value};

mod cli;
mod de;
mod error;
pub mod torrent;
mod value;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Decode { bencoded_value } => {
            let decoded: Value = from_str(bencoded_value)?;
            println!("{}", decoded);
        }
        Command::Info { torrent_path } => {
            let torrent = std::fs::read(torrent_path).context("could not read torrent file")?;
            let metadata: crate::torrent::TorrentMetadataInfo =
                serde_bencode::from_bytes(&torrent).context("invalid torrent file")?;
            println!("{}", metadata);
        }
    }
    Ok(())
}
