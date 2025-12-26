use std::{env::home_dir, net::SocketAddr, path::PathBuf};

use {
    clap::Parser,
    config::Config,
    serde::Deserialize,
    tracing::info,
    tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt},
};

use kvs::{
    EngineType, KvServer, Result,
    thread_pool::{ChannelThreadPool, ThreadPool},
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
    engine: EngineType,
}

fn main() -> Result<()> {
    init_logging();
    info!("version {}", env!("CARGO_PKG_VERSION"));

    let cli = Cli::parse();
    info!("Engine: {:?}", cli.engine);

    let settings = Settings::new()?;

    let thread_pool = ChannelThreadPool::new(8)?;
    let server = KvServer::new(cli.addr, settings.storage_path, cli.engine, thread_pool)?;
    info!("Listening on {}", cli.addr);
    server.run()?;

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
