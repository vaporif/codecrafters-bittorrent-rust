use clap::{arg, command, Parser, Subcommand};
type Chars = Vec<u8>;

#[derive(Parser)]
#[command(author = "Dmytro Onypko", name = "Torrent Sample Client")]
struct Cli {
    #[command(subcommand, name = "action")]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(long_about = "Decode Bencode Value")]
    Decode {
        #[arg(
            name = "bencoded value",
            help = "value to decode, could be string of non utf8 chars"
        )]
        bencoded_value: Chars,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Decode { bencoded_value } => {
            println!("{}", String::from_utf8(bencoded_value).unwrap());
        }
    }
}
