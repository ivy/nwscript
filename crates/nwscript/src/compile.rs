//! Compiling a checked program to NCS bytecode.

use crate::Game;
use nwscript_codegen::Profile;
use nwscript_diag::Diagnostic;
use nwscript_sema::hir::Program;

/// The code generation profile of a game.
pub fn profile(game: Game) -> Profile {
    match game {
        Game::K1 => Profile::K1,
        Game::K2 => Profile::K2,
    }
}

/// Compile a checked program for `game`. Whether its diagnostics allow
/// compiling is the caller's decision, made after lint levels apply; the
/// bytes depend only on the program and the game.
pub fn compile(program: &Program, game: Game) -> Result<nwscript_ncs::Program, Diagnostic> {
    nwscript_codegen::generate(program, profile(game))
}
