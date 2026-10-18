//! The Lua binding of the `rakata` façade, run by `rakata lua SCRIPT`.
//!
//! [`run`] loads a script into a state holding the `rakata` table: the
//! install (`Install`, `Mod`), the formats (`twoda`, `gff`, `lip`, `tlk`),
//! the assets (`tpc`, `txi`, `tga`, `lyt`, `vis`, `walkmesh`, `mdl`),
//! dialogues with stable node keys (`dialog`), the authoring operations
//! (`author`), and the diagnostics as error values carrying their codes
//! (`Error`). The names are the façade's, so a script reads as the Rust
//! it stands for. `api.md` beside this crate documents every table and
//! function ([`REFERENCE`]).
//!
//! The state is hermetic: no `io`, `os`, `package` or `debug` library, no
//! `dofile` or `loadfile`, `require` only of modules in the script's
//! directory, and `rakata.fs` reads only below `--game`, the script's
//! directory and the paths the script's arguments name, and writes only
//! below `--game` and the arguments' paths.

/// The reference for scripts: every table and function.
pub const REFERENCE: &str = include_str!("../api.md");

mod assets;
mod author;
mod dialog;
pub mod error;
mod formats;
mod install;
mod sandbox;
mod values;

use std::path::{Path, PathBuf};

use mlua::{Lua, LuaOptions, StdLib, Table};
use rakata::game::Profile;

pub use error::{Coded, codes};
pub use sandbox::Print;

/// What a run is given: everything a script may read is named here.
#[derive(Debug, Clone)]
pub struct Options {
    /// The script.
    pub script: PathBuf,
    /// Its arguments, `rakata.args`.
    pub args: Vec<String>,
    /// The install, `--game`.
    pub game: Option<PathBuf>,
    /// The profile `Install.open` takes when the script names none.
    pub profile: Profile,
    /// Where `print` writes.
    pub print: Print,
}

/// A run that finished.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    /// What the script printed, under [`Print::Capture`].
    pub output: String,
}

/// A run that failed: the code, the message, and where in the script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub code: &'static str,
    pub message: String,
    /// The Lua traceback, when the error came through a call.
    pub traceback: Option<String>,
    /// What the script printed before it failed, under [`Print::Capture`].
    pub output: String,
}

/// A state holding the `rakata` table for `opts`, the script not yet run.
pub fn new_state(opts: &Options) -> mlua::Result<Lua> {
    let lua = Lua::new_with(
        StdLib::COROUTINE | StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::MATH,
        LuaOptions::default(),
    )?;
    lua.set_app_data(sandbox::Inputs::new(
        opts.game.as_deref(),
        &opts.script,
        &opts.args,
    ));
    lua.set_app_data(install::DefaultProfile(opts.profile));
    let rakata = lua.create_table()?;
    rakata.set("version", rakata::VERSION)?;
    rakata.set("args", lua.create_sequence_from(opts.args.iter().cloned())?)?;
    rakata.set("game", opts.game.as_ref().map(|g| g.display().to_string()))?;
    rakata.set("script", opts.script.display().to_string())?;
    error::install(&lua, &rakata)?;
    sandbox::install(&lua, &rakata, opts.print)?;
    install::install(&lua, &rakata)?;
    formats::install(&lua, &rakata)?;
    assets::install(&lua, &rakata)?;
    dialog::install(&lua, &rakata)?;
    author::install(&lua, &rakata)?;
    explain(&lua, &rakata)?;
    lua.globals().set("rakata", rakata)?;
    Ok(lua)
}

/// `rakata.explain(code)`: the summary and explanation of a code.
fn explain(lua: &Lua, rakata: &Table) -> mlua::Result<()> {
    rakata.set(
        "explain",
        lua.create_function(|lua, id: String| match rakata::codes::lookup(&id) {
            None => Ok(mlua::Value::Nil),
            Some(c) => {
                let t = lua.create_table()?;
                t.set("code", c.id)?;
                t.set("summary", c.summary)?;
                t.set("explanation", c.explanation)?;
                Ok(mlua::Value::Table(t))
            }
        })?,
    )
}

/// The Lua error as a failure: the coded error inside it, else `RE1804`.
fn failure(e: &mlua::Error, output: String) -> Failure {
    let traceback = match e {
        mlua::Error::CallbackError { traceback, .. } => Some(traceback.clone()),
        _ => None,
    };
    match error::find(e) {
        Some(c) => Failure {
            code: c.code,
            message: c.message.clone(),
            traceback,
            output,
        },
        None => Failure {
            code: codes::SCRIPT_FAILED,
            message: plain_message(e),
            traceback,
            output,
        },
    }
}

/// A Lua error's message without the callback wrapping.
fn plain_message(e: &mlua::Error) -> String {
    match e {
        mlua::Error::CallbackError { cause, .. } => plain_message(cause),
        mlua::Error::RuntimeError(m) => m.clone(),
        mlua::Error::SyntaxError { message, .. } => message.clone(),
        other => other.to_string(),
    }
}

fn captured(lua: &Lua) -> String {
    lua.app_data_ref::<sandbox::Captured>()
        .map(|c| c.0.borrow().clone())
        .unwrap_or_default()
}

/// Run the script `opts` names.
pub fn run(opts: &Options) -> Result<Outcome, Failure> {
    let lua = new_state(opts).map_err(|e| failure(&e, String::new()))?;
    let code = std::fs::read(&opts.script).map_err(|e| Failure {
        code: codes::READ_FAILED,
        message: format!("cannot read `{}`: {e}", opts.script.display()),
        traceback: None,
        output: String::new(),
    })?;
    let name = format!("@{}", display(&opts.script));
    match lua.load(&code).set_name(name).exec() {
        Ok(()) => Ok(Outcome {
            output: captured(&lua),
        }),
        Err(e) => Err(failure(&e, captured(&lua))),
    }
}

fn display(p: &Path) -> String {
    p.display().to_string()
}
