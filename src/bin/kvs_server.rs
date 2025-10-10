use std::{
    cell::RefCell,
    env::home_dir,
    fs::{self, File},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
};

use {
    clap::Parser,
    config::Config,
    serde::Deserialize,
    tracing::info,
    tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt},
};

use bincode::config::standard;
use kvs::{EngineType, KvCommand, KvEngine, KvError, KvResponse, KvSled, KvStore, Result};

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
    engine: EngineType,
}

fn get_engine_type(p: &Path) -> Result<Option<EngineType>> {
    let engine_file = p.join(".engine.lock");
    if !engine_file.exists() {
        return Ok(None);
    }

    Ok(Some(serde_json::from_reader(File::open(engine_file)?)?))
}

fn set_engine_type(p: &Path, engine: EngineType) -> Result<()> {
    let engine_file = p.join(".engine.lock");
    fs::write(engine_file, serde_json::to_string(&engine)?)?;

    Ok(())
}

/// Opens the appropriate engine based on what's persisted, or creates new with specified type
pub fn open_engine(
    path: &Path,
    requested_engine: EngineType,
) -> Result<RefCell<Box<dyn KvEngine>>> {
    match get_engine_type(path)? {
        Some(existing_engine) if existing_engine != requested_engine => {
            return Err(KvError::WrongEngine {
                requested: requested_engine,
                existing: existing_engine,
            });
        }
        None => {
            // First time, persist the engine choice
            set_engine_type(path, requested_engine)?;
        }
        _ => {} // Engine types match, proceed
    }

    match requested_engine {
        EngineType::Kvs => Ok(RefCell::new(Box::new(KvStore::open(path)?))),
        EngineType::Sled => Ok(RefCell::new(Box::new(KvSled::open(path)?))),
        _ => panic!("unimplemented engine type!"),
    }
}

fn main() -> Result<()> {
    init_logging();

    let settings = Settings::new()?;

    let cli = Cli::parse();

    let engine = open_engine(&settings.storage_path, cli.engine)?;

    info!("version {}", env!("CARGO_PKG_VERSION"));
    info!("Engine: {:?}", cli.engine);

    let listener = TcpListener::bind(cli.addr)?;

    info!("Listening on {}", cli.addr);

    for stream in listener.incoming() {
        handle_connection(stream?, &engine)?;
    }

    Ok(())
}

fn init_logging() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // Write to the terminal with pretty formatting.
    let terminal_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false) // enable later for multi-threaded support
        .pretty();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(terminal_layer)
        .init();
}

#[derive(Deserialize)]
struct Settings {
    storage_path: PathBuf,
}

impl Settings {
    fn new() -> Result<Settings> {
        let path = home_dir()
            .expect("failed to set platform home dir")
            .join(".kvsrc.toml");

        let config = Config::builder()
            .add_source(config::File::with_name(
                path.to_str().expect("couldn't convert path to str"),
            ))
            .add_source(config::Environment::with_prefix("KVS"))
            .build()?;

        Ok(config.try_deserialize()?)
    }
}

fn handle_connection(mut stream: TcpStream, engine: &RefCell<Box<dyn KvEngine>>) -> Result<()> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    // Vec w/ len and capacity of `len`.
    let mut data = vec![0u8; len];
    stream.read_exact(&mut data)?;

    let (command, _): (KvCommand, usize) = bincode::serde::decode_from_slice(&data, standard())?;

    // Process command and create response
    let response = match command {
        KvCommand::Get { key } => {
            let mut engine = engine.borrow_mut();
            match engine.get(key) {
                Ok(value) => KvResponse::Ok(value),
                Err(e) => KvResponse::Err(e.to_string()),
            }
        }
        KvCommand::Set { key, value } => {
            let mut engine = engine.borrow_mut();
            match engine.set(key, value) {
                Ok(()) => KvResponse::Ok(None),
                Err(e) => KvResponse::Err(e.to_string()),
            }
        }
        KvCommand::Rm { key } => {
            let mut engine = engine.borrow_mut();
            match engine.rm(key) {
                Ok(()) => KvResponse::Ok(None),
                Err(e) => KvResponse::Err(e.to_string()),
            }
        }
    };

    // Serialize and send response back to client
    let response_bytes = bincode::serde::encode_to_vec(&response, standard())?;
    let response_len = response_bytes.len() as u32;

    stream.write_all(&response_len.to_be_bytes())?;
    stream.write_all(&response_bytes)?;
    stream.flush()?;

    Ok(())
}
