//! The rakata command line. Each command is one call into the `rakata`
//! façade; this crate parses arguments and writes either rendered text or
//! the JSON envelope, the same envelope and exit codes as `nws`.

mod cli;
mod cmd;
mod output;

use clap::Parser;
use std::process::ExitCode;
use std::sync::Mutex;

/// The documented exit codes, the table `nws` shares.
pub use rakata::Exit;

/// The input a command is working on, named in the internal-error message.
pub static CURRENT_INPUT: Mutex<Option<String>> = Mutex::new(None);

/// Record the input being processed, for the internal-error message.
pub fn working_on(input: impl Into<String>) {
    if let Ok(mut g) = CURRENT_INPUT.lock() {
        *g = Some(input.into());
    }
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let input = CURRENT_INPUT.lock().ok().and_then(|g| g.clone());
        eprintln!("error: internal error in rakata: {info}");
        match input {
            Some(i) => eprintln!("note: while processing `{i}`"),
            None => eprintln!("note: before any input was read"),
        }
        eprintln!(
            "note: this is a bug; please report it at https://github.com/ivy/nwscript/issues with the command line, `rakata --version` and that input"
        );
        std::process::exit(Exit::Internal as i32);
    }));
}

fn main() -> ExitCode {
    install_panic_hook();
    let cli = cli::Cli::parse();
    cmd::run(cli).into()
}
