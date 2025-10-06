use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
};

use {
    bincode::config::standard,
    clap::{Parser, Subcommand},
};

use kvs::{KvCommand, KvResponse, Result};

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

    let mut stream = TcpStream::connect(addr)?;

    // Keep track of command type for response handling
    let is_get_command = matches!(cli.command, Commands::Get { .. });

    let kv_command: KvCommand = cli.command.into();
    let bytes = bincode::serde::encode_to_vec(&kv_command, standard())?;
    let len = bytes.len() as u32;

    // Send command
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(&bytes)?;
    stream.flush()?;

    // Read response
    let mut response_len_bytes = [0u8; 4];
    stream.read_exact(&mut response_len_bytes)?;
    let response_len = u32::from_be_bytes(response_len_bytes) as usize;

    let mut response_data = vec![0u8; response_len];
    stream.read_exact(&mut response_data)?;

    let (response, _): (KvResponse, usize) =
        bincode::serde::decode_from_slice(&response_data, standard())?;

    // Handle response
    match response {
        KvResponse::Ok(Some(value)) => {
            println!("{}", value);
        }
        KvResponse::Ok(None) => {
            // For Get commands with no value, print "Key not found"
            // For Set/Rm commands, no output expected on success
            if is_get_command {
                println!("Key not found");
            }
        }
        KvResponse::Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    }

    Ok(())
}
