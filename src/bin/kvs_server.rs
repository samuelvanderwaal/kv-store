use std::{
    io::Read,
    net::{SocketAddr, TcpListener, TcpStream},
    str::FromStr,
};

use {
    clap::Parser,
    tracing::info,
    tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt},
};

use kvs::{KvError, Result};

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

    init_logging();

    info!("version {}", env!("CARGO_PKG_VERSION"));
    info!("Engine: {:?}", cli.engine);

    let listener = TcpListener::bind(cli.addr)?;

    info!("Listening on {}", cli.addr);

    for stream in listener.incoming() {
        handle_connection(stream?)?;
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut data: Vec<u8> = vec![];
    stream.read(&mut data)?;
    info!("read {} bytes", data.len());

    Ok(())
}

fn init_logging() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // Write to the terminal with pretty formatting.
    let terminal_layer = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false) // enable later for multi-threaded support
        .pretty();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(terminal_layer)
        .init();
}
