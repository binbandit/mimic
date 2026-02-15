use mimic::cli;
use std::process;

fn main() {
    match cli::run() {
        Ok(()) => process::exit(0),
        Err(code) => process::exit(code),
    }
}
