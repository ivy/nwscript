//! Checking scripts: the front end and semantic analysis, with includes and
//! `nwscript.nss` from a resolver.

use crate::Resolver;
use nwscript_diag::{Diagnostic, SourceMap, codes};
use nwscript_sema::{Analysis, EngineTable, Includes, analyze};
use nwscript_syntax::{Parse, parse};
use std::collections::HashMap;
use std::sync::Arc;

/// Checks scripts, reading each `nwscript.nss` and include once however many
/// scripts use it. Files are registered in the caller's [`SourceMap`] so
/// the diagnostics' spans resolve.
#[derive(Default)]
pub struct Checker {
    tables: HashMap<String, (Arc<[u8]>, Arc<EngineTable>)>,
    parses: HashMap<String, (Arc<[u8]>, Parse)>,
    overlay: Option<(&'static str, &'static str)>,
}

/// Which declarations of the engine's routines a script is compiled
/// against.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Declarations {
    /// The `nwscript.nss` the resolver finds, as it is.
    #[default]
    Shipped,
    /// The shipped file with the routines BioWare's own scripts were
    /// compiled against restored where they differ, as their bytecode
    /// shows. The vanilla scripts compile to BioWare's bytes only with
    /// these.
    BioWare,
}

impl Declarations {
    /// The overlay for `game`: its name and the prototypes it replaces.
    pub fn overlay(self, game: crate::Game) -> Option<(&'static str, &'static str)> {
        match (self, game) {
            (Declarations::Shipped, _) => None,
            (Declarations::BioWare, crate::Game::K1) => Some((
                "<k1-bioware overlay>",
                include_str!("overlays/k1-bioware.nss"),
            )),
            (Declarations::BioWare, crate::Game::K2) => None,
        }
    }
}

/// One script's outcome: the analysis, `None` when there was no
/// `nwscript.nss` to check against, and every diagnostic.
pub struct Checked {
    pub analysis: Option<Analysis>,
    pub diagnostics: Vec<Diagnostic>,
    /// The engine table the script was checked against.
    pub table: Option<Arc<EngineTable>>,
}

impl Checker {
    pub fn new() -> Self {
        Self::default()
    }

    /// A checker whose engine tables take these declarations.
    pub fn with_declarations(declarations: Declarations, game: crate::Game) -> Self {
        Checker {
            overlay: declarations.overlay(game),
            ..Self::default()
        }
    }

    /// Check the script `name` with contents `bytes`, finding its includes
    /// and `nwscript.nss` through `resolver`.
    ///
    /// What reading `nwscript.nss` reports, such as KotOR 1's line-5145
    /// typo, is about that file; a script that calls a routine whose
    /// declaration did not read is told so at the call.
    pub fn check(
        &mut self,
        resolver: &mut dyn Resolver,
        map: &mut SourceMap,
        name: &str,
        bytes: &[u8],
    ) -> Checked {
        let file = map.add(name.to_string(), bytes.to_vec());
        let main = parse(file, bytes);
        let table = match resolver.nwscript() {
            Ok(Some(r)) => match self.tables.get(&r.path) {
                Some((b, t)) if *b == r.bytes => t.clone(),
                _ => {
                    let f = map.add(r.path.clone(), r.bytes.clone());
                    let mut t = EngineTable::from_nwscript(f, &r.bytes).0;
                    if let Some((name, text)) = self.overlay {
                        let o = map.add(name, text.as_bytes().to_vec());
                        // A prototype the file lacks is no routine to replace.
                        let _ = t.overlay(o, text.as_bytes());
                    }
                    let t = Arc::new(t);
                    self.tables.insert(r.path, (r.bytes, t.clone()));
                    t
                }
            },
            Ok(None) => {
                let mut diagnostics = main.diagnostics().to_vec();
                diagnostics.push(
                    Diagnostic::new(&codes::E0002, "no nwscript.nss found")
                        .help("pass --nwscript FILE, or -I a directory that holds nwscript.nss"),
                );
                return Checked {
                    analysis: None,
                    diagnostics,
                    table: None,
                };
            }
            Err(e) => {
                let mut diagnostics = main.diagnostics().to_vec();
                diagnostics.push(Diagnostic::new(
                    &codes::E0001,
                    format!("cannot read nwscript.nss: {e}"),
                ));
                return Checked {
                    analysis: None,
                    diagnostics,
                    table: None,
                };
            }
        };
        let mut includes = ResolverIncludes {
            resolver,
            map,
            parses: &mut self.parses,
        };
        let analysis = analyze(name, main, &table, &mut includes);
        Checked {
            diagnostics: analysis.diagnostics.clone(),
            analysis: Some(analysis),
            table: Some(table),
        }
    }
}

/// A resolver as analysis's source of includes, parsing each file once.
struct ResolverIncludes<'a> {
    resolver: &'a mut dyn Resolver,
    map: &'a mut SourceMap,
    parses: &'a mut HashMap<String, (Arc<[u8]>, Parse)>,
}

impl Includes for ResolverIncludes<'_> {
    fn include(&mut self, name: &str) -> Result<Option<(String, Parse)>, String> {
        let Some(r) = self.resolver.include(name).map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        if let Some((b, p)) = self.parses.get(&r.path)
            && *b == r.bytes
        {
            return Ok(Some((r.path, p.clone())));
        }
        let file = self.map.add(r.path.clone(), r.bytes.clone());
        let p = parse(file, &r.bytes);
        self.parses.insert(r.path.clone(), (r.bytes, p.clone()));
        Ok(Some((r.path, p)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemResolver;

    const NWSCRIPT: &str = "int TRUE = 1;\r\nvoid PrintInteger(int n);\r\nvoid Typo(INT n);\r\n";

    #[test]
    fn checks_with_includes_and_records_inputs() {
        let mut r = MemResolver::new()
            .with("nwscript.nss", NWSCRIPT)
            .with("k_inc_x.nss", "int X() { return TRUE; }");
        let mut map = SourceMap::new();
        let mut c = Checker::new();
        let out = c.check(
            &mut r,
            &mut map,
            "t.nss",
            b"#include \"k_inc_x\"\r\nvoid main() { PrintInteger(X()); }",
        );
        assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
        assert!(out.analysis.unwrap().program.entry.is_some());
        let roles: Vec<&str> = r.inputs().iter().map(|i| i.role.as_str()).collect();
        assert_eq!(roles, ["nwscript", "include"]);
        // A second script reuses the parses.
        let files = map.files().count();
        let out = c.check(
            &mut r,
            &mut map,
            "u.nss",
            b"#include \"k_inc_x\"\r\nvoid main() { X(); }",
        );
        assert!(out.diagnostics.is_empty());
        assert_eq!(map.files().count(), files + 1);
    }

    #[test]
    fn nwscript_typos_are_reported_only_where_used() {
        let mut r = MemResolver::new().with("nwscript.nss", NWSCRIPT);
        let mut map = SourceMap::new();
        let mut c = Checker::new();
        let out = c.check(&mut r, &mut map, "t.nss", b"void main() { }");
        assert!(out.diagnostics.is_empty());
        let out = c.check(&mut r, &mut map, "t.nss", b"void main() { Typo(1); }");
        assert_eq!(out.diagnostics[0].code.id, "E0301");
    }

    #[test]
    fn no_nwscript_is_e0002() {
        let mut r = MemResolver::new();
        let mut map = SourceMap::new();
        let out = Checker::new().check(&mut r, &mut map, "t.nss", b"void main() { }");
        assert!(out.analysis.is_none());
        assert_eq!(out.diagnostics[0].code.id, "E0002");
    }

    #[test]
    fn bioware_declarations_restore_the_eleven_argument_conversation() {
        let ns = "int FALSE = 0;\r\nint CONVERSATION_TYPE_CINEMATIC = 0;\r\nvoid ActionStartConversation(object oObjectToConverse, string sDialogResRef = \"\", int bPrivateConversation = FALSE, int nConversationType = CONVERSATION_TYPE_CINEMATIC, int bIgnoreStartRange = FALSE, string sNameObjectToIgnore1 = \"\", string sNameObjectToIgnore2 = \"\", string sNameObjectToIgnore3 = \"\", string sNameObjectToIgnore4 = \"\", string sNameObjectToIgnore5 = \"\", string sNameObjectToIgnore6 = \"\", int bUseLeader = FALSE);\r\n";
        let src = b"void main() { ActionStartConversation(OBJECT_SELF); }";
        let argc = |d: Declarations| {
            let mut r = MemResolver::new().with("nwscript.nss", ns);
            let mut map = SourceMap::new();
            let out = Checker::with_declarations(d, crate::Game::K1)
                .check(&mut r, &mut map, "t.nss", src);
            assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
            let p =
                crate::compile::compile(&out.analysis.unwrap().program, crate::Game::K1).unwrap();
            p.instructions()
                .iter()
                .find_map(|i| match i.operands {
                    nwscript_ncs::Operands::Action { argc, .. } => Some(argc),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(argc(Declarations::Shipped), 12);
        assert_eq!(argc(Declarations::BioWare), 11);
    }
}
