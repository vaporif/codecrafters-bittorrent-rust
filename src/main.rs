use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

mod cli;
mod de;
mod error;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Decode { bencoded_value } => {
            let decoded = de::parse_bencode_byte_string(bencoded_value)?;
            println!("{}", decoded);
        }
    }

    Ok(())
}
