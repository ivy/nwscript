//! The dialogue lints over an install: [`rakata_dialog::lint`] with the
//! install answering, every dialogue of the install or of one module, and
//! the list of findings already known.
//!
//! A dialogue in a module's capsules is linted in that module: its scripts,
//! voice-overs and lip sync resolve as the engine resolves them there. A
//! dialogue the global layers hold (`Override/`, the BIFs) plays in any
//! module, so it is linted against the install alone, and a script or lip
//! file that no global layer holds counts as present when any module's
//! capsules hold it.
//!
//! The lip sync of the global dialogues (the party's, `banter.dlg`) lies in
//! `lips/localization.mod`, which the engine loads whatever the module and
//! the install's search order does not list; a lip file there counts as
//! present for every dialogue.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use nwscript::diag::SourceMap;
use nwscript::ncs::{Instruction, Opcode, Operands, Program};
use nwscript::sema::{EngineTable, Type};
use rakata_dialog::{Finding, GlobalReads, Place, References};
use rakata_game::{ModuleName, Profile};
use rakata_res::{ResId, ResRef, ResType};
use serde::{Deserialize, Serialize};

use super::{Loaded, Result, Session};

/// Registered here; the known list's own error.
pub const BAD_KNOWN: &str = "RE1109";

/// One dialogue to lint: its id, and the module it is linted in, `None`
/// for a dialogue of the global layers.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Target {
    pub id: ResId,
    pub module: Option<ModuleName>,
}

/// A finding as the known list records it: no text, only where and what.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Entry {
    /// The dialogue, `NAME.dlg`.
    pub dialog: String,
    /// The module it was linted in; absent for a global dialogue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    /// The node (`E3`, `R0`), the link (`E3>R4`), the start (`start>E0`),
    /// or the dialogue's end script (`end` for `EndConversation`, `abort`
    /// for `EndConverAbort`).
    pub at: String,
    pub code: String,
}

impl Entry {
    pub fn of(dialog: &str, module: Option<&str>, f: &Finding) -> Self {
        Entry {
            dialog: dialog.to_ascii_lowercase(),
            module: module.map(str::to_ascii_lowercase),
            at: at(&f.place),
            code: f.code.to_string(),
        }
    }
}

/// A place as the known list spells it.
pub fn at(p: &Place) -> String {
    match p {
        Place::Node(n) => n.to_string(),
        Place::Link { from, to } => format!("{from}>{to}"),
        Place::Start { to } => format!("start>{to}"),
        Place::End { aborted: false } => "end".to_string(),
        Place::End { aborted: true } => "abort".to_string(),
    }
}

/// The known list: the findings a run accepts, `[[finding]]` tables of
/// [`Entry`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Known {
    #[serde(default, rename = "finding")]
    pub findings: BTreeSet<Entry>,
}

impl Known {
    /// Read a known list. Fails with RE1109 when the file does not read or
    /// parse.
    pub fn read(path: &Path) -> std::result::Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("{} is not a known list: {e}", path.display()))
    }

    /// The list as TOML, sorted, one table per finding.
    pub fn to_toml(&self) -> String {
        let mut out = String::from(
            "# Findings `rakata dialog lint` accepts: the dialogue, the module it was\n# linted in (none for a global dialogue), where (E3, R0, E3>R4, start>E0,\n# and end or abort for its EndConversation or EndConverAbort script) and the\n# code. Written by `rakata dialog lint --write-known`.\n\n",
        );
        for e in &self.findings {
            out.push_str("[[finding]]\n");
            out.push_str(&format!("dialog = {:?}\n", e.dialog));
            if let Some(m) = &e.module {
                out.push_str(&format!("module = {m:?}\n"));
            }
            out.push_str(&format!("at = {:?}\ncode = {:?}\n\n", e.at, e.code));
        }
        out
    }

    pub fn contains(&self, e: &Entry) -> bool {
        self.findings.contains(e)
    }
}

/// The findings of one dialogue.
#[derive(Debug)]
pub struct Linted {
    pub loaded: Loaded,
    pub findings: Vec<Finding>,
}

/// An install opened for linting dialogues.
pub struct Linter {
    session: Session,
    /// For every resource some module's capsules hold, the modules.
    holders: Option<HashMap<ResId, Vec<ModuleName>>>,
    routines: Option<Vec<Routine>>,
    /// Whether the dialogue being linted lies in a global layer.
    global: bool,
    declared: Option<Option<HashSet<String>>>,
    reads: HashMap<(ResRef, Option<ModuleName>), GlobalReads>,
    /// The members of `lips/localization.mod`.
    global_lips: Option<HashSet<ResId>>,
}

impl Linter {
    /// Open the install at `game` and its `dialog.tlk`.
    pub fn open(game: &Path, profile: Profile) -> Result<Self> {
        Ok(Linter {
            session: Session::open(game, profile)?,
            holders: None,
            routines: None,
            global: false,
            declared: None,
            reads: HashMap::new(),
            global_lips: None,
        })
    }

    pub fn session(&mut self) -> &mut Session {
        &mut self.session
    }

    fn holders(&mut self) -> Result<&HashMap<ResId, Vec<ModuleName>>> {
        if self.holders.is_none() {
            let mut map: HashMap<ResId, Vec<ModuleName>> = HashMap::new();
            let install = &self.session.install;
            for m in install.modules() {
                for id in install.module_ids(m)? {
                    map.entry(id).or_default().push(m.clone());
                }
            }
            self.holders = Some(map);
        }
        Ok(self.holders.as_ref().expect("built"))
    }

    /// The dialogues to lint. With a module, every dialogue its capsules
    /// hold, linted in it. Without, the same for every module, then every
    /// dialogue of the global layers, linted against the install alone,
    /// except one in `Override/` that a module's capsules also hold: that
    /// copy is the one those modules load, and is linted in each of them.
    pub fn targets(&mut self, module: Option<&ModuleName>) -> Result<Vec<Target>> {
        let mut out = Vec::new();
        let modules: Vec<ModuleName> = match module {
            Some(m) => vec![m.clone()],
            None => self.session.install.modules().to_vec(),
        };
        for m in &modules {
            for id in self.session.install.module_ids(m)? {
                if id.ty == ResType::Dlg {
                    out.push(Target {
                        id,
                        module: Some(m.clone()),
                    });
                }
            }
        }
        if module.is_none() {
            let install = &self.session.install;
            let overridden: BTreeSet<ResId> = install
                .override_ids()
                .into_iter()
                .filter(|id| id.ty == ResType::Dlg)
                .collect();
            let mut global: BTreeSet<ResId> = install
                .key()
                .entries()
                .iter()
                .map(|e| e.id)
                .filter(|id| id.ty == ResType::Dlg)
                .collect();
            global.extend(overridden.iter().copied());
            let holders = self.holders()?;
            global.retain(|id| !(overridden.contains(id) && holders.contains_key(id)));
            out.extend(global.into_iter().map(|id| Target { id, module: None }));
        }
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// Load and lint one target.
    pub fn lint_target(&mut self, t: &Target) -> Result<Linted> {
        self.session.set_module(t.module.clone());
        let loaded = self.session.load_id_with(t.id, false)?;
        self.global = !self.in_module(loaded.dialog.name())?;
        Ok(self.lint_loaded(loaded))
    }

    /// Load `what` through `module`, and lint it: in the module when its
    /// capsules hold the dialogue (or it is a file), else as a global
    /// dialogue. With no module named, a dialogue in `Override/` is a
    /// global one, a script no global layer holds counting as present when
    /// any module holds it.
    pub fn lint_named(&mut self, what: &str, module: Option<ModuleName>) -> Result<Linted> {
        self.session.set_module(module);
        let loaded = self.session.load_with(what, false)?;
        self.global = !Path::new(what).is_file() && !self.in_module(loaded.dialog.name())?;
        Ok(self.lint_loaded(loaded))
    }

    /// Whether the session's module's capsules hold `name`.
    fn in_module(&self, name: &str) -> Result<bool> {
        let Some(m) = self.session.module.as_ref() else {
            return Ok(false);
        };
        let Ok(id) = ResId::parse(name) else {
            return Ok(false);
        };
        Ok(self.session.install.module_ids(m)?.contains(&id))
    }

    fn lint_loaded(&mut self, loaded: Loaded) -> Linted {
        let findings = rakata_dialog::lint(&loaded.dialog, self);
        Linted { loaded, findings }
    }

    /// The bytes of a resource where the dialogue plays: through its
    /// module, and for a global dialogue, else in the first module whose
    /// capsules hold it.
    fn find(&mut self, id: ResId) -> Option<Vec<u8>> {
        let module = self.session.module.clone();
        if let Ok(Some((_, b))) = self.session.install.read(id, module.as_ref()) {
            return Some(b);
        }
        if !self.global {
            return None;
        }
        let first = self.holders().ok()?.get(&id)?.first()?.clone();
        self.session
            .install
            .read(id, Some(&first))
            .ok()
            .flatten()
            .map(|(_, b)| b)
    }

    fn exists(&mut self, id: ResId) -> bool {
        let module = self.session.module.clone();
        if matches!(self.session.install.read(id, module.as_ref()), Ok(Some(_))) {
            return true;
        }
        self.global
            && self
                .holders()
                .is_ok_and(|h| h.get(&id).is_some_and(|m| !m.is_empty()))
    }

    fn global_lips(&mut self) -> &HashSet<ResId> {
        self.global_lips.get_or_insert_with(|| {
            let root = self.session.install.root();
            crate::corpus::find_ci(root, "lips/localization.mod")
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| {
                    rakata_archive::Erf::parse(&b)
                        .ok()
                        .map(|e| e.members().iter().map(|m| m.id).collect())
                })
                .unwrap_or_default()
        })
    }

    fn routines(&mut self) -> &[Routine] {
        if self.routines.is_none() {
            let id = ResId::new(ResRef::new("nwscript").expect("short"), ResType::Nss);
            let table = match self.session.install.read(id, None) {
                Ok(Some((_, bytes))) => {
                    let mut map = SourceMap::new();
                    let file = map.add("nwscript.nss", bytes.clone());
                    EngineTable::from_nwscript(file, &bytes).0.functions
                }
                _ => Vec::new(),
            };
            let routines = table
                .into_iter()
                .map(|f| Routine {
                    // A vector is three stack values; such a call is not
                    // followed backwards.
                    plain: f.params.iter().all(|p| p.ty != Type::Vector) && f.ret != Type::Vector,
                    value: f.ret != Type::Void,
                    name: f.name,
                })
                .collect();
            self.routines = Some(routines);
        }
        self.routines.as_deref().unwrap_or_default()
    }
}

/// An engine routine as the global scan needs it.
#[derive(Debug, Clone)]
pub struct Routine {
    pub name: String,
    /// Whether it returns a value.
    pub value: bool,
    /// Whether no parameter and no result is a vector.
    pub plain: bool,
}

/// The first instruction of the expression whose value the instruction at
/// `end` leaves on the stack, for the shapes a global's name takes: a
/// constant, a copy of a variable, an engine call on such expressions, or
/// the sum of two.
fn expression(ins: &[Instruction], end: usize, routines: &[Routine]) -> Option<usize> {
    let x = ins.get(end)?;
    match (&x.opcode, &x.operands) {
        (_, Operands::Int(_) | Operands::Float(_) | Operands::String(_) | Operands::Object(_))
        | (Opcode::CpTopSp | Opcode::CpTopBp, _) => Some(end),
        (_, Operands::Action { routine, argc }) => {
            let r = routines.get(usize::from(*routine))?;
            if !(r.value && r.plain) {
                return None;
            }
            let mut start = end;
            for _ in 0..*argc {
                start = expression(ins, start.checked_sub(1)?, routines)?;
            }
            Some(start)
        }
        (Opcode::Add, _) => {
            let right = expression(ins, end.checked_sub(1)?, routines)?;
            expression(ins, right.checked_sub(1)?, routines)
        }
        _ => None,
    }
}

/// The literal a name built by `+` begins with: the leftmost operand of the
/// chain of additions ending at `end`, when it is a string constant.
fn prefix(ins: &[Instruction], end: usize, routines: &[Routine]) -> Option<String> {
    let mut at = end;
    while ins.get(at)?.opcode == Opcode::Add {
        let right = expression(ins, at.checked_sub(1)?, routines)?;
        at = right.checked_sub(1)?;
    }
    match &ins.get(at)?.operands {
        Operands::String(s) if at != end => Some(rakata_tlk::codepage::decode(s)),
        _ => None,
    }
}

/// The globals a program reads: for each call of a one-argument
/// `GetGlobal*` routine, the string literal pushed directly before it, as a
/// call with a literal compiles (the rule `calls.jsonl` follows); else the
/// literal the name is built on, where it is a sum beginning with one; else
/// the routine, its argument computed.
pub fn global_reads(program: &Program, routines: &[Routine]) -> GlobalReads {
    let mut out = GlobalReads::default();
    let ins = program.instructions();
    for (i, x) in ins.iter().enumerate() {
        let Operands::Action { routine, argc: 1 } = x.operands else {
            continue;
        };
        let Some(r) = routines
            .get(usize::from(routine))
            .filter(|r| r.name.starts_with("GetGlobal"))
        else {
            continue;
        };
        let push = |list: &mut Vec<String>, v: String| {
            if !list.contains(&v) {
                list.push(v);
            }
        };
        let Some(arg) = i.checked_sub(1) else {
            continue;
        };
        match &ins[arg].operands {
            Operands::String(s) => push(&mut out.names, rakata_tlk::codepage::decode(s)),
            _ => match prefix(ins, arg, routines) {
                Some(p) if !p.is_empty() => push(&mut out.prefixes, p),
                _ => push(&mut out.computed, r.name.clone()),
            },
        }
    }
    out
}

impl References for Linter {
    fn resolves(&mut self, name: &ResRef, ty: ResType) -> bool {
        if ty == ResType::Wav
            && self
                .session
                .voices()
                .contains_key(&name.to_string().to_ascii_lowercase())
        {
            return true;
        }
        let id = ResId::new(*name, ty);
        if ty == ResType::Lip && self.global_lips().contains(&id) {
            return true;
        }
        self.exists(id)
    }

    fn talk_table_len(&mut self) -> Option<u32> {
        self.session.tlk.as_ref().map(|t| t.len() as u32)
    }

    fn globals_read(&mut self, condition: &ResRef) -> Vec<String> {
        self.global_reads(condition).names
    }

    fn global_reads(&mut self, condition: &ResRef) -> GlobalReads {
        let key = (*condition, self.session.module.clone());
        if let Some(r) = self.reads.get(&key) {
            return r.clone();
        }
        let reads = match self.find(ResId::new(*condition, ResType::Ncs)) {
            None => GlobalReads::default(),
            Some(bytes) => {
                let mut map = SourceMap::new();
                match nwscript::ncs::decode_named(format!("{condition}.ncs"), &bytes, &mut map) {
                    Ok(program) => global_reads(&program, self.routines()),
                    Err(_) => GlobalReads::default(),
                }
            }
        };
        self.reads.insert(key, reads.clone());
        reads
    }

    fn declares_global(&mut self, name: &str) -> Option<bool> {
        self.declared()
            .map(|d| d.contains(&name.to_ascii_uppercase()))
    }

    fn declares_prefix(&mut self, prefix: &str) -> Option<bool> {
        let p = prefix.to_ascii_uppercase();
        self.declared().map(|d| d.iter().any(|n| n.starts_with(&p)))
    }
}

impl Linter {
    /// The names `globalcat.2da` declares, uppercase; `None` without one.
    fn declared(&mut self) -> Option<&HashSet<String>> {
        if self.declared.is_none() {
            let id = ResId::new(ResRef::new("globalcat").expect("short"), ResType::TwoDa);
            let names = match self.session.install.read(id, None) {
                Ok(Some((_, bytes))) => rakata_2da::TwoDa::parse(&bytes)
                    .ok()
                    .map(|t| crate::author::globals::declared(&t)),
                _ => None,
            };
            self.declared = Some(names);
        }
        self.declared.as_ref().and_then(Option::as_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn routines() -> Vec<Routine> {
        ["GetGlobalBoolean", "IntToString", "GetGlobalNumber"]
            .into_iter()
            .map(|n| Routine {
                name: n.into(),
                value: true,
                plain: true,
            })
            .collect()
    }

    fn reads(asm: &str) -> GlobalReads {
        let program = nwscript::ncs::asm::parse(nwscript::diag::FileId(0), asm.as_bytes()).unwrap();
        global_reads(&program, &routines())
    }

    #[test]
    fn a_literal_name_is_read_as_it_is() {
        let r = reads("CONSTS \"K_ONE\"\nACTION 0, 1\nCONSTS \"K_TWO\"\nACTION 2, 1\nRETN\n");
        assert_eq!(r.names, ["K_ONE", "K_TWO"]);
        assert!(r.prefixes.is_empty() && r.computed.is_empty());
    }

    #[test]
    fn a_name_built_on_a_literal_gives_its_prefix() {
        // GetGlobalBoolean("OAJ_DOWN_" + IntToString(nNPC))
        let r =
            reads("CONSTS \"OAJ_DOWN_\"\nCPTOPSP -8, 4\nACTION 1, 1\nADDSS\nACTION 0, 1\nRETN\n");
        assert_eq!(r.prefixes, ["OAJ_DOWN_"]);
        assert!(r.names.is_empty() && r.computed.is_empty());
    }

    #[test]
    fn a_name_in_a_variable_is_computed() {
        let r = reads("CPTOPSP -8, 4\nACTION 0, 1\nRETN\n");
        assert_eq!(r.computed, ["GetGlobalBoolean"]);
        // A variable first, the literal after: no prefix to check.
        let r = reads("CPTOPSP -8, 4\nCONSTS \"_X\"\nADDSS\nACTION 2, 1\nRETN\n");
        assert_eq!(r.computed, ["GetGlobalNumber"]);
    }
}
