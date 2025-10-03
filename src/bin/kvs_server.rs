use std::{net::SocketAddr, str::FromStr};

use kvs::KvError;

use {
    clap::{Args, Parser, Subcommand},
    kvs::{KvStore, Result},
};

const HELP: &str = "\
{before-help}{name} {version}
{author}
{about}

{usage-heading} {usage}

{all-args}{after-help}
";

#[derive(Parser)]
#[command(
    version,
    author,
    about,
    long_about = None,
    help_template = HELP
)]
struct Cli {
    #[arg(short, long)]
    addr: SocketAddr,

    #[arg(short, long)]
    engine: EngineName,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum EngineName {
    Kvs,
    Sled,
}

impl FromStr for EngineName {
    type Err = KvError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "kvs" | "kvstore" => Ok(EngineName::Kvs),
            "sled" | "sld" => Ok(EngineName::Sled),
            _ => Err(KvError::InvalidEngineName),
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    println!("addr: {:?}", cli.addr);
    println!("engine: {:?}", cli.engine);

    Ok(())
}
