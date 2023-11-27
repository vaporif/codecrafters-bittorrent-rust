use clap::{arg, command, Parser, Subcommand};

const DEFAULT_PORT: u16 = 6881;

#[derive(Parser)]
#[command(author = "Dmytro Onypko", name = "Torrent Sample Client")]
pub struct Cli {
    #[command(subcommand, name = "action")]
    pub command: Command,
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    pub port: u16,
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
    #[command(long_about = "Encode Bencode Value")]
    Encode {
        #[arg(name = "value", help = "value to encode")]
        value: String,
    },
    #[command(long_about = "Print metadata info of a torrent")]
    Info {
        #[arg(name = "torrent path", help = "torrent path")]
        torrent_path: String,
    },
    #[command(long_about = "Print ips of peers")]
    Peers {
        #[arg(name = "torrent path", help = "torrent path")]
        torrent_path: String,
    },
}
