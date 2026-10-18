//! The install side of the dialogue checks: [`References`] answered from
//! the install, in the mod's module, and from what the mod has written.

use std::collections::HashSet;

use nwscript::diag::SourceMap;
use nwscript::ncs::Operands;
use nwscript::sema::EngineTable;
use rakata_dialog::References;
use rakata_res::{ResId, ResRef, ResType};

use super::Mod;

impl Mod {
    /// The engine's routine names by `ACTION` number, from the
    /// `nwscript.nss` the game loads; empty when it has none.
    fn routines(&mut self) -> &[String] {
        if self.routines.is_none() {
            let id = ResId::new(ResRef::new("nwscript").expect("short"), ResType::Nss);
            let names = match self.loaded(id) {
                Ok(l) => {
                    let mut map = SourceMap::new();
                    let file = map.add("nwscript.nss", l.bytes.clone());
                    EngineTable::from_nwscript(file, &l.bytes)
                        .0
                        .functions
                        .into_iter()
                        .map(|f| f.name)
                        .collect()
                }
                Err(_) => Vec::new(),
            };
            self.routines = Some(names);
        }
        self.routines.as_deref().unwrap_or_default()
    }

    /// The names `globalcat.2da` declares, uppercase; `None` without one.
    fn declared(&mut self) -> Option<&HashSet<String>> {
        if self.globals.is_none() {
            let id = ResId::new(ResRef::new("globalcat").expect("short"), ResType::TwoDa);
            let names = self
                .loaded(id)
                .ok()
                .and_then(|l| rakata_2da::TwoDa::parse(&l.bytes).ok())
                .map(|t| super::globals::declared(&t));
            self.globals = Some(names);
        }
        self.globals.as_ref().and_then(Option::as_ref)
    }
}

impl References for Mod {
    fn resolves(&mut self, name: &ResRef, ty: ResType) -> bool {
        let id = ResId::new(*name, ty);
        if self.written.contains_key(&id) {
            return true;
        }
        if ty == ResType::Wav
            && self
                .voices()
                .contains(&name.to_string().to_ascii_lowercase())
        {
            return true;
        }
        matches!(self.install.read(id, self.module.as_ref()), Ok(Some(_)))
    }

    fn talk_table_len(&mut self) -> Option<u32> {
        self.tlk().map(|t| t.len() as u32)
    }

    /// The literal names passed to a `GetGlobal*` routine: a `CONSTS`
    /// directly before the one-argument `ACTION`, as a call with a string
    /// literal compiles.
    fn globals_read(&mut self, condition: &ResRef) -> Vec<String> {
        let Ok(l) = self.loaded(ResId::new(*condition, ResType::Ncs)) else {
            return Vec::new();
        };
        let mut map = SourceMap::new();
        let Ok(program) =
            nwscript::ncs::decode_named(format!("{condition}.ncs"), &l.bytes, &mut map)
        else {
            return Vec::new();
        };
        let routines = self.routines();
        let mut out: Vec<String> = Vec::new();
        for pair in program.instructions().windows(2) {
            if let (Operands::String(s), Operands::Action { routine, argc: 1 }) =
                (&pair[0].operands, &pair[1].operands)
                && routines
                    .get(usize::from(*routine))
                    .is_some_and(|n| n.starts_with("GetGlobal"))
            {
                let name = rakata_tlk::codepage::decode(s);
                if !out.contains(&name) {
                    out.push(name);
                }
            }
        }
        out
    }

    fn declares_global(&mut self, name: &str) -> Option<bool> {
        self.declared()
            .map(|d| d.contains(&name.to_ascii_uppercase()))
    }
}
