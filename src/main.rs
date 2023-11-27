use crate::{de::from_str, value::Value};
use clap::Parser;
use cli::{Cli, Command};

use crate::prelude::*;
mod cli;
mod de;
mod error;
mod prelude;
mod ser;
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
            let mut metadata: crate::torrent::TorrentMetadataInfo =
                crate::de::from_bytes(&torrent).context("invalid torrent file")?;
            metadata
                .compute_hash()
                .context("Failed to compute hash of torrent")?;
            println!("{}", metadata);
        }
        Command::Encode { value } => {
            let value = crate::ser::to_bytes(value).context("Failed to encode")?;
            let value = String::from_utf8_lossy(&value);
            println!("{}", value);
        }
    }
    Ok(())
}
