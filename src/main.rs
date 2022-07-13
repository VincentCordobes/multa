use std::process;

use clap::Command;
use clap::CommandFactory;
use clap::Parser;
use clap::Subcommand;
use clap_complete::Shell;
use clap_complete::{generate, Generator};
use std::io;

#[derive(Parser, Debug)]
#[clap(name = "multa")]
#[clap(about = "Practice your times table", long_about = None)]
struct Cli {
    /// The profile to be used for the session
    #[clap(global = true, short, long, default_value = "default")]
    profile: String,
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Print last reviewed cards
    Report,
    /// Run multa in examination mode
    Exam,
    /// Generate tab-completion scripts for your shell
    Completion(CompletionCommand),
}

#[derive(Parser, Debug)]
pub struct CompletionCommand {
    #[clap(value_parser)]
    pub shell: Shell,
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

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Report) => {
            let opts = multa::ReportOpts::from(cli);
            multa::report(opts)
        }
        Some(Commands::Completion(CompletionCommand { shell })) => {
            let mut cmd = Cli::command();
            print_completions(shell, &mut cmd);
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
