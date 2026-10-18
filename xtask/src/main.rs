//! Development tasks. `cargo xtask corpus --help` for the corpus commands,
//! `cargo xtask grammar-census --help` for the grammar's evidence,
//! `cargo xtask ini-census --help` for what the mods' change files contain.
//! `cargo xtask readme --help` for the README's generated tables.

mod corpus;
mod grammar_census;
mod ini_census;
mod readme;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("corpus") => corpus::main(&args[1..]),
        Some("grammar-census") => grammar_census::main(&args[1..]),
        Some("ini-census") => ini_census::main(&args[1..]),
        Some("readme") => readme::main(&args[1..]).map_err(Into::into),
        _ => {
            eprintln!(
                "usage: cargo xtask corpus --help | cargo xtask grammar-census --help | cargo xtask ini-census --help | cargo xtask readme --help"
            );
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
