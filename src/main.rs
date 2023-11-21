use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

use crate::{de::from_str, value::Value};

mod cli;
mod de;
mod error;
pub mod value;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Decode { bencoded_value } => {
            let decoded: Value = from_str(bencoded_value)?;
            println!("{}", decoded);
        }
    }
    Ok(())
}
