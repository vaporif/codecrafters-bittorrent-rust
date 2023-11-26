use clap::{arg, command, Parser, Subcommand};

#[derive(Parser)]
#[command(author = "Dmytro Onypko", name = "Torrent Sample Client")]
pub struct Cli {
    #[command(subcommand, name = "action")]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(long_about = "Decode Bencode Value")]
    Decode {
        #[arg(
            name = "bencoded value",
            help = "value to decode, could be string of non utf8 chars"
        )]
        bencoded_value: String,
    },
    #[command(long_about = "Print metadata info of a torrent")]
    Info {
        #[arg(name = "torrent path", help = "torrent path")]
        torrent_path: String,
    },
}
