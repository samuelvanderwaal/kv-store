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

pub struct KvClient {
    stream: TcpStream,
}

impl KvClient {
    fn new(addr: SocketAddr) -> Result<Self> {
        let stream = TcpStream::connect(addr)?;

        Ok(KvClient { stream })
    }

    /// Create a client from an existing stream (useful for testing)
    pub fn from_stream(stream: TcpStream) -> Self {
        KvClient { stream }
    }

    fn send_command(&mut self, command: Commands) -> Result<()> {
        let kv_command: KvCommand = command.into();
        let bytes = bincode::serde::encode_to_vec(&kv_command, standard())?;
        let len = bytes.len() as u32;

        // Send command
        self.stream.write_all(&len.to_be_bytes())?;
        self.stream.write_all(&bytes)?;
        self.stream.flush()?;

        // Read response
        let mut response_len_bytes = [0u8; 4];
        self.stream.read_exact(&mut response_len_bytes)?;
        let response_len = u32::from_be_bytes(response_len_bytes) as usize;

        let mut response_data = vec![0u8; response_len];
        self.stream.read_exact(&mut response_data)?;

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
                if matches!(kv_command, KvCommand::Get { .. }) {
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let addr = cli.addr.unwrap_or_else(|| {
        eprintln!("error: required argument '--addr <ADDR>' not provided");
        eprintln!("\nFor more information, try '--help'");
        std::process::exit(2); // Exit code 2 is what clap uses for usage errors
    });

    let mut client = KvClient::new(addr)?;
    client.send_command(cli.command)?;

    Ok(())
}
