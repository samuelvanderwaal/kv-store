use std::env::current_dir;

use clap::{Args, Parser, Subcommand};
use kvs::{KvStore, Result};

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
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Get { key: String },
    Set { key: String, value: String },
    Rm { key: String },
}

#[derive(Args, Debug)]
struct KeyArg {
    key: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut store = KvStore::open(current_dir()?)?;

    match cli.command {
        Commands::Get { key } => {
            let value_opt = store.get(key)?;
            match value_opt {
                Some(value) => println!("{value}"),
                None => println!("Key not found"),
            }
            std::process::exit(0);
        }
        Commands::Set { key, value } => {
            store.set(key, value)?;
            std::process::exit(0);
        }
        Commands::Rm { key } => match store.remove(key) {
            Ok(()) => std::process::exit(0),
            Err(_) => {
                println!("Key not found");
                std::process::exit(1);
            }
        },
    }
}
