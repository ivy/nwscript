//! A mod's scripts compiled through the `nwscript` façade, as `nws compile`
//! compiles them, against the install: `nwscript.nss` and every include
//! come from the mod's own sources first, then from the copy the game
//! loads (`Override/`, the module's capsules, `patch.erf`, the BIFs), the
//! mod's earlier writes among them.

use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use nwscript::diag::render::{Color, render};
use nwscript::diag::{SourceMap, has_errors};
use nwscript::modes::Mode;
use nwscript::{Checker, Declarations, Game, InputRecord, Resolved, Resolver, sha256_hex};
use rakata_game::Profile;
use rakata_res::{ResId, ResRef, ResType};

use super::{Error, Mod, Result};

/// A script's source: its name without `.nss` and its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    name: String,
    text: Vec<u8>,
}

impl Source {
    /// A source named `name`. Fails with `RE0101` when the name is longer
    /// than the sixteen bytes the compiled script's resref holds.
    pub fn new(name: impl Into<String>, text: impl Into<Vec<u8>>) -> Result<Self> {
        let name = name.into();
        ResRef::new(&name).map_err(Error::Res)?;
        Ok(Source {
            name,
            text: text.into(),
        })
    }

    /// The name, without `.nss`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The source's bytes.
    pub fn text(&self) -> &[u8] {
        &self.text
    }
}

/// How to compile, as `nws`'s flags say it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    /// `--declarations`: the shipped `nwscript.nss`, or with the routines
    /// BioWare's own scripts were compiled against restored.
    pub declarations: Declarations,
    /// `--compat`: strict, or what nwnnsscomp or BioWare's compiler
    /// accepted.
    pub mode: Mode,
}

impl Flags {
    /// `--declarations bioware --compat bioware`: a decompiled BioWare
    /// script compiles to BioWare's bytes.
    pub const BIOWARE: Flags = Flags {
        declarations: Declarations::BioWare,
        mode: Mode::Bioware,
    };
}

/// A script compiled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compiled {
    /// The script's name, without `.ncs`.
    pub name: String,
    /// The bytecode, for `write_override` as `NAME.ncs`.
    pub ncs: Vec<u8>,
    /// The warnings, as `nws` prints them; empty when there were none.
    pub warnings: String,
    /// Every include and `nwscript.nss` read, with its hash.
    pub inputs: Vec<InputRecord>,
}

/// The mod's sources, then the install, as `nwscript`'s resolver.
struct ModResolver<'a> {
    sources: &'a HashMap<String, Arc<[u8]>>,
    m: &'a Mod,
    inputs: Vec<InputRecord>,
}

impl ModResolver<'_> {
    fn find(&mut self, name: &str, role: &str) -> io::Result<Option<Resolved>> {
        let key = name.to_ascii_lowercase();
        let found = if let Some(bytes) = self.sources.get(&key) {
            Some(Resolved {
                path: format!("{name}.nss"),
                bytes: bytes.clone(),
            })
        } else {
            let Ok(resref) = ResRef::new(name) else {
                return Ok(None);
            };
            match self.m.loaded(ResId::new(resref, ResType::Nss)) {
                Ok(l) => Some(Resolved {
                    path: l.location,
                    bytes: l.bytes.into(),
                }),
                Err(Error::Game(rakata_game::Error::NotFound { .. })) => None,
                Err(e) => return Err(io::Error::other(e.to_string())),
            }
        };
        if let Some(r) = &found {
            self.inputs.push(InputRecord {
                role: role.into(),
                path: r.path.clone(),
                sha256: sha256_hex(&r.bytes),
            });
        }
        Ok(found)
    }
}

impl Resolver for ModResolver<'_> {
    fn include(&mut self, name: &str) -> io::Result<Option<Resolved>> {
        self.find(name, "include")
    }

    fn nwscript(&mut self) -> io::Result<Option<Resolved>> {
        self.find("nwscript", "nwscript")
    }

    fn inputs(&self) -> &[InputRecord] {
        &self.inputs
    }
}

/// Compile each of `sources` for the install's game, each one's includes
/// found among `sources` and `includes` (libraries the mod's scripts
/// include, its own or generated, compiled with them but not alone), then
/// in the install. Nothing is written: the bytecode is for
/// [`Mod::write_override`]. Fails on the first script with an error after
/// `flags.mode`'s levels apply, with its diagnostics (`RE1408`).
pub fn compile(
    m: &Mod,
    sources: &[Source],
    includes: &[Source],
    flags: Flags,
) -> Result<Vec<Compiled>> {
    let game = match m.install().profile() {
        Profile::K1 => Game::K1,
        Profile::K2 => Game::K2,
    };
    let library: HashMap<String, Arc<[u8]>> = sources
        .iter()
        .chain(includes)
        .map(|s| (s.name.to_ascii_lowercase(), Arc::from(s.text.as_slice())))
        .collect();
    let levels = flags.mode.levels();
    let mut checker = Checker::with_declarations(flags.declarations, game);
    let mut out = Vec::with_capacity(sources.len());
    for s in sources {
        let mut map = SourceMap::new();
        let mut resolver = ModResolver {
            sources: &library,
            m,
            inputs: Vec::new(),
        };
        let name = format!("{}.nss", s.name);
        let checked = checker.check(&mut resolver, &mut map, &name, &s.text);
        let mut diags = nwscript::modes::finish(
            checked.analysis.as_ref(),
            checked.diagnostics,
            &map,
            &levels,
        );
        let program = match (&checked.analysis, has_errors(&diags)) {
            (Some(a), false) => match nwscript::compile::compile(&a.program, game) {
                Ok(p) => Some(p),
                Err(d) => {
                    diags.push(d);
                    None
                }
            },
            _ => None,
        };
        let rendered = diags
            .iter()
            .map(|d| render(d, &map, Color::Never))
            .collect::<Vec<_>>()
            .join("\n");
        let Some(program) = program else {
            return Err(Error::DoesNotCompile {
                script: name,
                rendered,
                diagnostics: diags,
            });
        };
        out.push(Compiled {
            name: s.name.clone(),
            ncs: nwscript::ncs::encode(&program),
            warnings: rendered,
            inputs: resolver.inputs,
        });
    }
    Ok(out)
}
