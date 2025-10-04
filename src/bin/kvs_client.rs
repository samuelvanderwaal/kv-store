use std::{
    io::Write,
    net::{SocketAddr, TcpStream},
};

use {
    bincode::config::standard,
    clap::{Parser, Subcommand},
};

use kvs::{KvCommand, Result};

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

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Get { key: String },
    Set { key: String, value: String },
    Rm { key: String },
}

impl Into<KvCommand> for Commands {
    fn into(self) -> KvCommand {
        match self {
            Self::Get { key } => KvCommand::Get { key },
            Self::Set { key, value } => KvCommand::Set { key, value },
            Self::Rm { key } => KvCommand::Rm { key },
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut stream = TcpStream::connect(cli.addr)?;
    let kv_command: KvCommand = cli.command.into();
    let bytes = bincode::serde::encode_to_vec(&kv_command, standard())?;
    let len = bytes.len();

    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(&bytes)?;
    stream.flush()?;

    Ok(())
}
