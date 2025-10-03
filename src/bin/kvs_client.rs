use std::net::{SocketAddr, TcpStream};

use {
    clap::{Parser, Subcommand},
    kvs::Result,
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

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Get { key: String },
    Set { key: String, value: String },
    Rm { key: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let _stream = TcpStream::connect(cli.addr);

    match cli.command {
        Commands::Get { key: _ } => {
            std::process::exit(0);
        }
        Commands::Set { key: _, value: _ } => {
            std::process::exit(0);
        }
        Commands::Rm { key: _ } => {
            std::process::exit(1);
        }
    }
}
