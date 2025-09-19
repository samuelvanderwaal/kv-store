use clap::{Args, Parser, Subcommand};
use kvs::KvStore;

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
    Get(KeyArg),
    Set {
        #[command(flatten)]
        key: KeyArg,

        value: String,
    },
    Rm(KeyArg),
}

#[derive(Args, Debug)]
struct KeyArg {
    key: String,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Get(arg) => {
            eprintln!("unimplemented");
            std::process::exit(1);
        }
        Commands::Set { key, value } => {
            eprintln!("unimplemented");
            std::process::exit(1);
        }
        Commands::Rm(arg) => {
            eprintln!("unimplemented");
            std::process::exit(1);
        }
    }
}
