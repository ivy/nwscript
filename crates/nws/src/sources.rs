//! Where a command's includes and `nwscript.nss` come from: the directories
//! named on the command line first, then the install `--game-dir` names.
//!
//! The directories are `nwscript`'s own [`FsResolver`]; the install is
//! `rakata-game`'s [`Install`], which answers from the copies the game would
//! load, so this crate adds no search order of its own. Only this binary
//! takes `rakata-game`: the `nwscript-*` libraries stay free of it.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use nwscript::diag::Diagnostic;
use nwscript::{FsResolver, InputRecord, Resolved, Resolver, sha256_hex};
use rakata_game::Install;

/// The install `--game-dir` opened, shared by every resolver of a command
/// and across the decompiler's verifier threads.
#[derive(Clone)]
pub struct GameDir {
    /// The directory as the flag spelt it, for messages.
    pub dir: PathBuf,
    pub install: Arc<Mutex<Install>>,
}

/// The resolver `nws` hands the library: the explicit directories, then
/// the install when there is one.
pub struct Sources {
    fs: FsResolver,
    game: Option<GameDir>,
    inputs: Vec<InputRecord>,
    /// Set once the install was asked for `nwscript.nss` and had none.
    install_lacks_nwscript: bool,
}

impl Sources {
    pub fn new(fs: FsResolver, game: Option<GameDir>) -> Self {
        Sources {
            fs,
            game,
            inputs: Vec::new(),
            install_lacks_nwscript: false,
        }
    }

    fn record(&mut self, role: &str, r: Resolved) -> Resolved {
        self.inputs.push(InputRecord {
            role: role.into(),
            path: r.path.clone(),
            sha256: sha256_hex(&r.bytes),
        });
        r
    }

    fn in_install(
        &mut self,
        ask: impl FnOnce(&mut Install) -> io::Result<Option<Resolved>>,
    ) -> io::Result<Option<Resolved>> {
        let Some(game) = &self.game else {
            return Ok(None);
        };
        let mut install = game
            .install
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        ask(&mut install)
    }

    /// The `nwscript.nss` read from a directory, when one was: a file an
    /// external compiler can be pointed at, which an archive member is not.
    pub fn nwscript_file(&self) -> Option<PathBuf> {
        self.fs
            .inputs()
            .iter()
            .find(|i| i.role == "nwscript")
            .map(|i| PathBuf::from(&i.path))
    }

    /// The diagnostic for an install that was asked for `nwscript.nss` and
    /// had none, once the search has run; it names the flag.
    pub fn missing_nwscript(&self) -> Option<Diagnostic> {
        let game = self.game.as_ref().filter(|_| self.install_lacks_nwscript)?;
        Some(
            Diagnostic::new(
                &crate::codes::NE0001,
                format!(
                    "no nwscript.nss in the install at `{}`",
                    game.dir.display()
                ),
            )
            .help(
                "--game-dir names the folder holding the game's chitin.key; or pass --nwscript FILE, or -I a directory that holds nwscript.nss",
            ),
        )
    }

    /// A checker's diagnostics with the install's own report in place of
    /// the library's: the library says only that no `nwscript.nss` was
    /// found, and the install knows which flag named it.
    pub fn report(&self, diags: Vec<Diagnostic>) -> Vec<Diagnostic> {
        let Some(missing) = self.missing_nwscript() else {
            return diags;
        };
        let mut diags: Vec<Diagnostic> = diags
            .into_iter()
            .filter(|d| !std::ptr::eq(d.code, &nwscript::diag::codes::E0002))
            .collect();
        diags.push(missing);
        diags
    }
}

impl Resolver for Sources {
    fn include(&mut self, name: &str) -> io::Result<Option<Resolved>> {
        if let Some(r) = self.fs.include(name)? {
            return Ok(Some(self.record("include", r)));
        }
        let found = self.in_install(|i| i.include(name))?;
        Ok(found.map(|r| self.record("include", r)))
    }

    fn nwscript(&mut self) -> io::Result<Option<Resolved>> {
        if let Some(r) = self.fs.nwscript()? {
            return Ok(Some(self.record("nwscript", r)));
        }
        let found = self.in_install(Install::nwscript)?;
        if found.is_none() && self.game.is_some() {
            self.install_lacks_nwscript = true;
        }
        Ok(found.map(|r| self.record("nwscript", r)))
    }

    fn inputs(&self) -> &[InputRecord] {
        &self.inputs
    }
}

/// The install at `dir`, for `--game-dir`: opened once per command.
pub fn open(dir: &Path, game: nwscript::Game, module: Option<&str>) -> Result<GameDir, String> {
    let profile = match game {
        nwscript::Game::K1 => rakata_game::Profile::K1,
        nwscript::Game::K2 => rakata_game::Profile::K2,
    };
    let mut install =
        Install::open(dir, profile).map_err(|e| format!("--game-dir {}: {e}", dir.display()))?;
    if let Some(m) = module {
        let name = rakata_game::ModuleName::new(m);
        if !install.modules().contains(&name) {
            return Err(format!(
                "--module {m}: no capsule named `{name}` under `{}/modules`",
                dir.display()
            ));
        }
        install.set_resolver_module(Some(name));
    }
    Ok(GameDir {
        dir: dir.to_path_buf(),
        install: Arc::new(Mutex::new(install)),
    })
}
