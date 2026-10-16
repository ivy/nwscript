//! Development tasks. `cargo xtask corpus --help` for the corpus commands,
//! `cargo xtask grammar-census --help` for the grammar's evidence.

mod archive;
mod corpus;
mod grammar_census;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("corpus") => corpus::main(&args[1..]),
        Some("grammar-census") => grammar_census::main(&args[1..]),
        _ => {
            eprintln!("usage: cargo xtask corpus --help | cargo xtask grammar-census --help");
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
