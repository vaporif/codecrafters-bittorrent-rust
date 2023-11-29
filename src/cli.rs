use std::net::{Ipv4Addr, SocketAddrV4};

use crate::prelude::*;
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
    #[command(long_about = "Handshake with peer")]
    Handshake {
        #[arg(name = "torrent path", help = "torrent path")]
        torrent_path: String,
        #[arg(name = "peer ip with port", help = "<peer_ip>:<peer_port>")]
        peer: String,
    },
    #[command(name = "download_piece", long_about = "Download piece")]
    DownloadPiece {
        #[arg(name = "torrent path", help = "torrent path")]
        torrent_path: String,
        #[arg(name = "piece number")]
        piece_number: u64,
        #[arg(
            long,
            short,
            name = "output path",
            help = "output path for piece to download"
        )]
        output: String,
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
