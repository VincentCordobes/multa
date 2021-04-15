use std::process;

fn main() {
    env_logger::init();

    if let Err(e) = multa::run() {
        println!("Application error: {:?}", e);

        process::exit(1);
    }
}
