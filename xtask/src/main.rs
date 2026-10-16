//! Development tasks. `cargo xtask corpus --help` for the corpus commands.

mod archive;
mod corpus;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("corpus") => corpus::main(&args[1..]),
        _ => {
            eprintln!("usage: cargo xtask corpus --help");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
