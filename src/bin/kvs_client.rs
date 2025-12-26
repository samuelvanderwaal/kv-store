use std::net::SocketAddr;

use clap::{Parser, Subcommand};

use kvs::{KvClient, KvCommand, Result};

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
    #[arg(short, long, global = true)]
    addr: Option<SocketAddr>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Get { key: String },
    Set { key: String, value: String },
    Rm { key: String },
}

impl From<Commands> for KvCommand {
    fn from(val: Commands) -> Self {
        match val {
            Commands::Get { key } => KvCommand::Get { key },
            Commands::Set { key, value } => KvCommand::Set { key, value },
            Commands::Rm { key } => KvCommand::Rm { key },
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let addr = cli.addr.unwrap_or_else(|| {
        eprintln!("error: required argument '--addr <ADDR>' not provided");
        eprintln!("\nFor more information, try '--help'");
        std::process::exit(2); // Exit code 2 is what clap uses for usage errors
    });

    let mut client = KvClient::new(addr)?;
    let kv_command: KvCommand = cli.command.into();

    client.send_command(kv_command)?;

    Ok(())
}
