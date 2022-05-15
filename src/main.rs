use clap::{Clap, Subcommand};
use std::process;

#[derive(Clap)]
struct Cli {
    #[clap(short, long, default_value = "default")]
    profile: String,
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Report,
    Exam,
}

impl From<Cli> for multa::Opts {
    fn from(cli: Cli) -> Self {
        Self {
            examination: matches!(cli.command, Some(Commands::Exam)),
            profile: cli.profile,
        }
    }
}

impl From<Cli> for multa::ReportOpts {
    fn from(cli: Cli) -> Self {
        Self {
            profile: cli.profile,
        }
    }
}

fn main() {
    env_logger::init();
    let cli: Cli = Cli::parse();

    match &cli.command {
        Some(Commands::Report) => {
            let opts = multa::ReportOpts::from(cli);
            multa::report(opts)
        }
        _ => {
            let opts = multa::Opts::from(cli);
            if let Err(e) = multa::run(opts) {
                println!("Application error: {:?}", e);

                process::exit(1);
            }
        }
    }
}
