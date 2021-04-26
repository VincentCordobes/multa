use clap::Clap;
use std::process;

#[derive(Clap)]
struct Opts {
    #[clap(short, long, default_value = "default")]
    profile: String,
    #[clap(short, long)]
    examination: bool,
}

impl From<Opts> for multa::Opts {
    fn from(opts: Opts) -> Self {
        Self {
            profile: opts.profile,
            examination: opts.examination,
        }
    }
}

fn main() {
    env_logger::init();
    let opts: Opts = Opts::parse();

    if let Err(e) = multa::run(multa::Opts::from(opts)) {
        println!("Application error: {:?}", e);

        process::exit(1);
    }
}
