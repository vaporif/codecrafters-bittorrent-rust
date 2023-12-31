use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
};

use crate::prelude::*;
use clap::{arg, command, Parser, Subcommand};

const DEFAULT_PORT: u16 = 6881;
const DEFAULT_MAX_PEERS: u8 = 10;

#[derive(Parser, Debug)]
#[command(author = "Dmytro Onypko", name = "Torrent Sample Client")]
pub struct Cli {
    #[command(subcommand, name = "action")]
    pub command: Command,
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    pub port: u16,
    #[arg(short, long, default_value_t = DEFAULT_MAX_PEERS)]
    pub max_peers: u8,
    #[arg(short, long)]
    pub tokio_console: bool,
}

#[derive(Subcommand, Debug)]
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
        torrent_path: PathBuf,
    },
    #[command(long_about = "Print ips of peers")]
    Peers {
        #[arg(name = "torrent path", help = "torrent path")]
        torrent_path: PathBuf,
    },
    #[command(long_about = "Handshake with peer")]
    Handshake {
        #[arg(name = "torrent path", help = "torrent path")]
        torrent_path: PathBuf,
        #[arg(name = "peer ip with port", help = "<peer_ip>:<peer_port>")]
        peer: String,
    },
    #[command(name = "download_piece", long_about = "Download piece")]
    DownloadPiece {
        #[arg(name = "torrent path", help = "torrent path")]
        torrent_path: PathBuf,
        #[arg(name = "piece number")]
        piece_number: usize,
        #[arg(
            long,
            short,
            name = "output path",
            help = "output path for piece to download"
        )]
        output: PathBuf,
    },
    #[command(long_about = "Download torrent")]
    Download {
        #[arg(name = "torrent path", help = "torrent path")]
        torrent_path: PathBuf,
        #[arg(
            long,
            short,
            name = "output path",
            help = "output path for piece to download"
        )]
        output: PathBuf,
    },
}

pub fn pares_peer_arg(arg: &str) -> Result<SocketAddrV4> {
    let parts: Vec<&str> = arg.split(':').collect();
    if parts.len() != 2 {
        bail!("please set ip correctly");
    }
    let ip = parts[0].parse::<Ipv4Addr>().context("failed to parse ip")?;
    let port = parts[1].parse::<u16>().context("failed to parse port")?;

    Ok(SocketAddrV4::new(ip, port))
}
