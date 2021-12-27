use std::env;
use std::process;

use rustboy::Config;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|e| {
        eprintln!("Failed to parse args: {}", e);
        process::exit(1);
    });

    if let Err(e) = rustboy::run(config) {
        eprintln!("Some error: {}", e);
        process::exit(1);
    }
}
