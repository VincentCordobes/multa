use std::process;

use clap::Parser;
use clap::Subcommand;

#[derive(Parser, Debug)]
#[clap(name = "multa")]
#[clap(about = "Practice your times table", long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
    #[clap(global = true, short, long, default_value = "global")]
    profile: String,
}

#[derive(Subcommand, Debug)]
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
