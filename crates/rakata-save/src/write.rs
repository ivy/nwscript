//! A save written anew: a copy of a read one whose globals or party differ,
//! every other file and member copied byte for byte. The layout is in
//! `docs/formats/save.md`, "Globals" and "The directory".

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rakata_gff::{Change, Gff, Label, Struct, Value};
use rakata_res::{ResId, ResRef, ResType};

use crate::fields::{At, int, string};
use crate::globals::{GlobalValue, Location};
use crate::save::{SAVEGAME, parse_gff};
use crate::{Error, Result, Save};

/// The most party members the game takes beside the player character.
pub const PARTY_MAX: usize = 2;

/// The type of a global, as the scripts and `globalcat.2da` name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlobalKind {
    Boolean,
    Number,
    String,
    Location,
}

impl GlobalKind {
    /// Every kind, in the order the file's catalogues come.
    pub const ALL: [GlobalKind; 4] = [
        GlobalKind::Boolean,
        GlobalKind::Number,
        GlobalKind::String,
        GlobalKind::Location,
    ];

    /// The kind `globalcat.2da`'s `type` column or a `TYPE:` prefix names,
    /// without regard to case: `Boolean`, `Number`, `String`, `Location`.
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|k| k.name().eq_ignore_ascii_case(text.trim()))
    }

    /// The name, as [`GlobalValue::kind`] spells it.
    pub const fn name(self) -> &'static str {
        match self {
            GlobalKind::Boolean => "boolean",
            GlobalKind::Number => "number",
            GlobalKind::String => "string",
            GlobalKind::Location => "location",
        }
    }

    /// What a value of this kind is written as, for messages.
    pub const fn syntax(self) -> &'static str {
        match self {
            GlobalKind::Boolean => "true, false, 1 or 0",
            GlobalKind::Number => "an integer from -128 to 127",
            GlobalKind::String => "any text",
            GlobalKind::Location => "six numbers x,y,z,fx,fy,fz: the position and the facing",
        }
    }

    /// The value `text` writes in this kind, if it reads as one.
    pub fn value(self, text: &str) -> Option<GlobalValue> {
        Some(match self {
            GlobalKind::Boolean => GlobalValue::Boolean(match text.trim() {
                t if t.eq_ignore_ascii_case("true") || t == "1" => true,
                t if t.eq_ignore_ascii_case("false") || t == "0" => false,
                _ => return None,
            }),
            GlobalKind::Number => GlobalValue::Number(text.trim().parse().ok()?),
            GlobalKind::String => GlobalValue::String(text.to_owned()),
            GlobalKind::Location => {
                let f: Vec<f32> = text
                    .split(',')
                    .map(|p| p.trim().parse::<f32>())
                    .collect::<std::result::Result<_, _>>()
                    .ok()?;
                let [x, y, z, fx, fy, fz] = f[..] else {
                    return None;
                };
                GlobalValue::Location(Location {
                    position: [x, y, z],
                    orientation: [fx, fy, fz],
                })
            }
        })
    }

    const fn catalogue(self) -> &'static str {
        match self {
            GlobalKind::Boolean => "CatBoolean",
            GlobalKind::Number => "CatNumber",
            GlobalKind::String => "CatString",
            GlobalKind::Location => "CatLocation",
        }
    }
}

impl GlobalValue {
    /// The value's kind.
    pub const fn global_kind(&self) -> GlobalKind {
        match self {
            GlobalValue::Boolean(_) => GlobalKind::Boolean,
            GlobalValue::Number(_) => GlobalKind::Number,
            GlobalValue::String(_) => GlobalKind::String,
            GlobalValue::Location(_) => GlobalKind::Location,
        }
    }
}

/// The globals an install declares (its `globalcat.2da`), by name compared
/// without regard to case. Only a declared global is read by the game's
/// scripts; the caller fills this from the install.
#[derive(Debug, Clone, Default)]
pub struct Declarations {
    by_name: BTreeMap<String, (String, GlobalKind)>,
}

impl Declarations {
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare `name` as `kind`; the first declaration of a name stands.
    pub fn add(&mut self, name: &str, kind: GlobalKind) {
        self.by_name
            .entry(name.to_ascii_lowercase())
            .or_insert_with(|| (name.to_owned(), kind));
    }

    /// The declaration of `name`: its spelling there, and its kind.
    pub fn get(&self, name: &str) -> Option<(&str, GlobalKind)> {
        self.by_name
            .get(&name.to_ascii_lowercase())
            .map(|(n, k)| (n.as_str(), *k))
    }

    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

/// One `NAME=VALUE` or `NAME=TYPE:VALUE` from the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub name: String,
    /// The `TYPE:` prefix, when one names a kind.
    pub kind: Option<GlobalKind>,
    /// The value's text, after any prefix.
    pub text: String,
}

impl Assignment {
    /// Parse `NAME=VALUE` or `NAME=TYPE:VALUE`. A prefix counts only when
    /// it names a kind, so a string value may hold a colon.
    pub fn parse(text: &str) -> Result<Self> {
        let Some((name, value)) = text.split_once('=') else {
            return Err(bad(text, "it is not NAME=VALUE".to_owned()));
        };
        let name = name.trim();
        if name.is_empty() {
            return Err(bad(text, "it names no global".to_owned()));
        }
        let (kind, rest) = match value.split_once(':') {
            Some((k, rest)) => match GlobalKind::parse(k) {
                Some(k) => (Some(k), rest),
                None => (None, value),
            },
            None => (None, value),
        };
        Ok(Assignment {
            name: name.to_owned(),
            kind,
            text: rest.to_owned(),
        })
    }
}

fn bad(text: &str, reason: String) -> Error {
    Error::BadAssignment {
        text: text.to_owned(),
        reason,
    }
}

/// What to change in a save.
#[derive(Debug, Clone, Default)]
pub struct Edits {
    /// The globals to set, in order.
    pub globals: Vec<Assignment>,
    /// The party to travel with the player, by NPC number or by the tag of
    /// its `AVAILNPC<n>.utc`; `None` leaves `PARTYTABLE.res` as it is.
    pub party: Option<Vec<String>>,
    /// Set globals the install does not declare, adding their names to the
    /// save's catalogues where the save lacks them.
    pub declare: bool,
}

/// One global set.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalChange {
    /// The name as the file spells it.
    pub name: String,
    /// The value before; `None` for a name added to the save's catalogue.
    pub before: Option<GlobalValue>,
    pub after: GlobalValue,
    /// Whether the install declares the name.
    pub declared: bool,
}

/// The party before and after, by NPC number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyChange {
    pub before: Vec<i64>,
    pub after: Vec<i64>,
}

/// One field of a rewritten GFF that differs from the original.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldChange {
    /// The GFF: `GLOBALVARS.res`, `PARTYTABLE.res`.
    pub member: String,
    pub change: Change,
}

/// What [`Save::write`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct Written {
    /// The new save's directory.
    pub dir: PathBuf,
    pub globals: Vec<GlobalChange>,
    pub party: Option<PartyChange>,
    /// The fields of the rewritten GFFs that differ, in file order.
    pub fields: Vec<FieldChange>,
    /// The files written anew, relative to the save's directory.
    pub rewritten: Vec<String>,
    /// The files copied byte for byte.
    pub copied: Vec<String>,
}

impl Save {
    /// Write a copy of this save to `out` with `edits` applied: the globals
    /// set in `GLOBALVARS.res` through `rakata-gff` (and the party in
    /// `PARTYTABLE.res` when `edits.party` is given), every other file
    /// copied byte for byte. A global `declarations` does not hold is
    /// refused unless `edits.declare`; a declared one the save lacks is
    /// added to its catalogue. `out` must not exist, or be an empty
    /// directory. Nothing is written when an edit is refused.
    pub fn write(&self, out: &Path, edits: &Edits, declarations: &Declarations) -> Result<Written> {
        let mut rewritten: Vec<(String, Vec<u8>)> = Vec::new();
        let mut fields = Vec::new();

        let mut globals = Vec::new();
        if !edits.globals.is_empty() {
            let Some((at, original)) = &self.globals else {
                return Err(Error::BadShape {
                    member: GLOBALVARS_RES.to_owned(),
                    path: String::new(),
                    expected: "in the save",
                });
            };
            let mut gff = original.clone();
            for a in &edits.globals {
                globals.push(set_global(&mut gff, at, a, declarations, edits.declare)?);
            }
            fields.extend(changes(at, original, &gff));
            rewritten.push((at.clone(), gff_bytes(&gff, at)?));
        }

        let mut party = None;
        if let Some(names) = &edits.party {
            let Some((at, original)) = &self.party else {
                return Err(Error::BadShape {
                    member: PARTYTABLE_RES.to_owned(),
                    path: String::new(),
                    expected: "in the save",
                });
            };
            let npcs = self.resolve_party(names)?;
            let mut gff = original.clone();
            party = Some(set_party(&mut gff, at, &npcs)?);
            fields.extend(changes(at, original, &gff));
            rewritten.push((at.clone(), gff_bytes(&gff, at)?));
        }

        // A GFF that lies inside SAVEGAME.sav rather than beside it goes
        // back into the archive, which is then written anew.
        let (inside, beside): (Vec<_>, Vec<_>) = rewritten
            .into_iter()
            .partition(|(at, _)| at.starts_with(&format!("{SAVEGAME}/")));
        let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for (at, bytes) in beside {
            files.insert(at.to_ascii_lowercase(), bytes);
        }
        if !inside.is_empty() {
            let mut erf = self.outer()?;
            for (at, bytes) in inside {
                let file = &at[SAVEGAME.len() + 1..];
                let stem = file.strip_suffix(".res").unwrap_or(file);
                let id = ResId::new(
                    ResRef::new(stem).map_err(|_| Error::BadShape {
                        member: at.clone(),
                        path: String::new(),
                        expected: "a member named by a resref",
                    })?,
                    ResType::Res,
                );
                erf.replace(id, bytes).map_err(|source| Error::Archive {
                    member: SAVEGAME.to_owned(),
                    source,
                })?;
            }
            let bytes = erf.to_bytes().map_err(|source| Error::Archive {
                member: SAVEGAME.to_owned(),
                source,
            })?;
            files.insert(SAVEGAME.to_ascii_lowercase(), bytes);
        }

        prepare_out(&self.dir, out)?;
        let mut written = Written {
            dir: out.to_path_buf(),
            globals,
            party,
            fields,
            rewritten: Vec::new(),
            copied: Vec::new(),
        };
        copy_dir(&self.dir, out, "", &files, &mut written)?;
        Ok(written)
    }

    /// The NPC numbers `names` give: each a number or the tag of an
    /// `AVAILNPC<n>.utc`, compared without regard to case.
    fn resolve_party(&self, names: &[String]) -> Result<Vec<i64>> {
        let npcs = self.party_gffs()?;
        let available = || {
            npcs.iter()
                .map(|(n, g)| match string(&g.root, "Tag") {
                    Some(t) => format!("{n} ({t})"),
                    None => n.to_string(),
                })
                .collect::<Vec<_>>()
        };
        let mut out = Vec::new();
        for name in names {
            let name = name.trim();
            let found = match name.parse::<u32>() {
                Ok(n) => npcs.iter().find(|(i, _)| *i == n),
                Err(_) => npcs.iter().find(|(_, g)| {
                    string(&g.root, "Tag").is_some_and(|t| t.eq_ignore_ascii_case(name))
                }),
            };
            let Some((n, _)) = found else {
                return Err(Error::NoSuchNpc {
                    name: name.to_owned(),
                    available: available(),
                });
            };
            let n = i64::from(*n);
            if out.contains(&n) {
                return Err(Error::BadParty {
                    reason: format!("NPC {n} is named twice"),
                });
            }
            out.push(n);
        }
        if out.len() > PARTY_MAX {
            return Err(Error::BadParty {
                reason: format!(
                    "{} members named; the game takes at most {PARTY_MAX} beside the player",
                    out.len()
                ),
            });
        }
        Ok(out)
    }
}

const GLOBALVARS_RES: &str = "GLOBALVARS.res";
const PARTYTABLE_RES: &str = "PARTYTABLE.res";

fn gff_bytes(gff: &Gff, at: &str) -> Result<Vec<u8>> {
    let bytes = gff.to_bytes().map_err(|source| Error::Gff {
        member: at.to_owned(),
        source,
    })?;
    // What is written must read back.
    parse_gff(&bytes, at)?;
    Ok(bytes)
}

fn changes(at: &str, before: &Gff, after: &Gff) -> Vec<FieldChange> {
    rakata_gff::diff(before, after)
        .into_iter()
        .map(|change| FieldChange {
            member: at.to_owned(),
            change,
        })
        .collect()
}

fn label(text: &str) -> Label {
    Label::new(text).expect("the save's labels are at most sixteen bytes")
}

/// Where `name` is in the file's catalogues: its kind, its index and its
/// spelling there.
fn locate(gff: &Gff, at: &str, name: &str) -> Result<Option<(GlobalKind, usize, String)>> {
    let root = At::new(at, "", &gff.root);
    for kind in GlobalKind::ALL {
        let cat = kind.catalogue();
        for (i, e) in root.list(cat)?.iter().enumerate() {
            let n = At::new(at, cat, e).string("Name")?;
            if n.eq_ignore_ascii_case(name) {
                return Ok(Some((kind, i, n)));
            }
        }
    }
    Ok(None)
}

fn set_global(
    gff: &mut Gff,
    at: &str,
    a: &Assignment,
    declarations: &Declarations,
    declare: bool,
) -> Result<GlobalChange> {
    let held = locate(gff, at, &a.name)?;
    let declared = declarations.get(&a.name);
    if declared.is_none() && !declare {
        return Err(Error::Undeclared {
            name: a.name.clone(),
        });
    }
    let raw = || {
        let prefix = a.kind.map(|k| format!("{}:", k.name())).unwrap_or_default();
        format!("{}={prefix}{}", a.name, a.text)
    };
    // The save's catalogue decides the kind, then the install's, then the
    // assignment's prefix; a prefix that disagrees is refused.
    let known = held
        .as_ref()
        .map(|(k, _, _)| (*k, "the save holds it"))
        .or(declared.map(|(_, k)| (k, "globalcat.2da declares it")));
    let kind = match (known, a.kind) {
        (Some((k, _)), None) => k,
        (Some((k, _)), Some(p)) if k == p => k,
        (Some((k, why)), Some(p)) => {
            return Err(bad(
                &raw(),
                format!(
                    "`{}` is a {}, as {why}, not a {}",
                    a.name,
                    k.name(),
                    p.name()
                ),
            ));
        }
        (None, Some(p)) => p,
        (None, None) => {
            return Err(bad(
                &raw(),
                format!(
                    "`{}` is in neither the save nor globalcat.2da, so its type must be given: {}=boolean:1",
                    a.name, a.name
                ),
            ));
        }
    };
    let Some(value) = kind.value(&a.text) else {
        return Err(bad(
            &raw(),
            format!(
                "`{}` is a {}, written as {}",
                a.name,
                kind.name(),
                kind.syntax()
            ),
        ));
    };

    let (index, name, before) = match held {
        Some((_, i, name)) => {
            let before = crate::Globals::from_gff(gff, at)?
                .vars
                .into_iter()
                .find(|g| g.name == name && g.value.global_kind() == kind)
                .map(|g| g.value);
            (i, name, before)
        }
        None => {
            let name = declared.map_or_else(|| a.name.clone(), |(n, _)| n.to_owned());
            (append_name(gff, at, kind, &name)?, name, None)
        }
    };
    write_value(gff, at, index, &value)?;
    Ok(GlobalChange {
        name,
        before,
        after: value,
        declared: declared.is_some(),
    })
}

/// Add `name` at the end of its kind's catalogue; its index.
fn append_name(gff: &mut Gff, at: &str, kind: GlobalKind, name: &str) -> Result<usize> {
    let cat = kind.catalogue();
    let Some(Value::List(list)) = gff.root.get_mut(cat) else {
        return Err(shape(at, cat, "a list"));
    };
    // A new element takes the struct id its neighbours carry.
    let mut s = Struct::new(list.first().map_or(0, |e| e.id));
    s.push(label("Name"), Value::string(name));
    list.push(s);
    Ok(list.len() - 1)
}

fn shape(at: &str, path: &str, expected: &'static str) -> Error {
    Error::BadShape {
        member: at.to_owned(),
        path: path.to_owned(),
        expected,
    }
}

fn void_mut<'a>(gff: &'a mut Gff, at: &str, field: &str) -> Result<&'a mut Vec<u8>> {
    match gff.root.get_mut(field) {
        Some(Value::Void(b)) => Ok(b),
        _ => Err(shape(at, field, "a VOID field")),
    }
}

/// Write global `index`'s value, growing the value field when the index is
/// past its end; every other bit and byte stays as it was.
fn write_value(gff: &mut Gff, at: &str, index: usize, value: &GlobalValue) -> Result<()> {
    match value {
        GlobalValue::Boolean(set) => {
            let bits = void_mut(gff, at, "ValBoolean")?;
            if bits.len() <= index / 8 {
                bits.resize(index / 8 + 1, 0);
            }
            // Most significant bit first within each byte.
            let mask = 0x80u8 >> (index % 8);
            if *set {
                bits[index / 8] |= mask;
            } else {
                bits[index / 8] &= !mask;
            }
        }
        GlobalValue::Number(n) => {
            let bytes = void_mut(gff, at, "ValNumber")?;
            if bytes.len() <= index {
                bytes.resize(index + 1, 0);
            }
            bytes[index] = *n as u8;
        }
        GlobalValue::Location(l) => {
            let bytes = void_mut(gff, at, "ValLocation")?;
            let end = 24 * (index + 1);
            if bytes.len() < end {
                bytes.resize(end, 0);
            }
            for (k, f) in l.position.iter().chain(&l.orientation).enumerate() {
                let o = 24 * index + 4 * k;
                bytes[o..o + 4].copy_from_slice(&f.to_le_bytes());
            }
        }
        GlobalValue::String(text) => {
            let Some(Value::List(list)) = gff.root.get_mut("ValString") else {
                return Err(shape(at, "ValString", "a list"));
            };
            let id = list.first().map_or(0, |e| e.id);
            while list.len() <= index {
                let mut s = Struct::new(id);
                s.push(label("String"), Value::string(""));
                list.push(s);
            }
            list[index].set(label("String"), Value::string(text));
        }
    }
    Ok(())
}

/// Make the party table's members exactly `npcs`, in order. A member
/// already in the table keeps its element as it was.
fn set_party(gff: &mut Gff, at: &str, npcs: &[i64]) -> Result<PartyChange> {
    let old: Vec<Struct> = At::new(at, "", &gff.root).list("PT_MEMBERS")?.to_vec();
    let before: Vec<i64> = old
        .iter()
        .map(|m| int(m, "PT_MEMBER_ID").unwrap_or(-1))
        .collect();
    let id = old.first().map_or(0, |e| e.id);
    let members = npcs
        .iter()
        .map(|&n| {
            old.iter()
                .find(|m| int(m, "PT_MEMBER_ID") == Some(n))
                .cloned()
                .unwrap_or_else(|| {
                    let mut s = Struct::new(id);
                    s.push(label("PT_MEMBER_ID"), Value::Int(n as i32))
                        .push(label("PT_IS_LEADER"), Value::Byte(0));
                    s
                })
        })
        .collect();
    gff.root.set(label("PT_MEMBERS"), Value::List(members));
    let count = npcs.len();
    let num = match gff.root.get("PT_NUM_MEMBERS") {
        Some(Value::Int(_)) => Value::Int(count as i32),
        Some(Value::Dword(_)) => Value::Dword(count as u32),
        _ => Value::Byte(count as u8),
    };
    gff.root.set(label("PT_NUM_MEMBERS"), num);
    Ok(PartyChange {
        before,
        after: npcs.to_vec(),
    })
}

/// `out` made ready: created, or an empty directory, and neither the
/// save's own directory nor inside it.
fn prepare_out(src: &Path, out: &Path) -> Result<()> {
    let exists = |reason: &str| Error::OutExists {
        dir: out.to_path_buf(),
        reason: reason.to_owned(),
    };
    if let (Ok(a), Some(b)) = (src.canonicalize(), resolved(out)) {
        if a == b {
            return Err(exists("it is the save being read"));
        }
        if b.starts_with(&a) {
            return Err(exists("it lies inside the save being read"));
        }
    }
    if out.exists() {
        if !out.is_dir() {
            return Err(exists("it is a file"));
        }
        let empty = std::fs::read_dir(out)
            .map_err(|source| Error::Io {
                path: out.to_path_buf(),
                source,
            })?
            .next()
            .is_none();
        if !empty {
            return Err(exists("it is not empty"));
        }
    }
    std::fs::create_dir_all(out).map_err(|source| Error::Write {
        path: out.to_path_buf(),
        source,
    })
}

/// `path` with its nearest existing ancestor resolved, symbolic links and
/// all, and the rest appended: where it is or would be.
fn resolved(path: &Path) -> Option<PathBuf> {
    let path = std::path::absolute(path).ok()?;
    let mut rest = Vec::new();
    let mut at = path.as_path();
    loop {
        if let Ok(real) = at.canonicalize() {
            return Some(rest.iter().rev().fold(real, |p, c| p.join(c)));
        }
        rest.push(at.file_name()?.to_owned());
        at = at.parent()?;
    }
}

/// Copy every entry of `src` into `out`, writing the files `rewritten`
/// holds (keyed by lowercase relative path) in place of their originals.
fn copy_dir(
    src: &Path,
    out: &Path,
    rel: &str,
    rewritten: &BTreeMap<String, Vec<u8>>,
    written: &mut Written,
) -> Result<()> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| Error::Io { path, source }
    };
    let mut entries: Vec<PathBuf> = std::fs::read_dir(src)
        .map_err(io(src))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for from in entries {
        let Some(name) = from.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        let here = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let to = out.join(&name);
        let wrote = |source| Error::Write {
            path: to.clone(),
            source,
        };
        if from.is_dir() {
            std::fs::create_dir_all(&to).map_err(wrote)?;
            copy_dir(&from, &to, &here, rewritten, written)?;
        } else if let Some(bytes) = rewritten.get(&here.to_ascii_lowercase()) {
            std::fs::write(&to, bytes).map_err(wrote)?;
            written.rewritten.push(here);
        } else {
            let bytes = std::fs::read(&from).map_err(io(&from))?;
            std::fs::write(&to, bytes).map_err(wrote)?;
            written.copied.push(here);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> Struct {
        let mut s = Struct::new(0);
        s.push(label("Name"), Value::string(name));
        s
    }

    /// Ten booleans, B0 and B9 set; one number.
    fn gvt() -> Gff {
        let mut g = Gff::new(*b"GVT ");
        let r = &mut g.root;
        r.push(
            label("CatBoolean"),
            Value::List((0..10).map(|i| named(&format!("B{i}"))).collect()),
        );
        r.push(label("CatNumber"), Value::List(vec![named("N0")]));
        r.push(label("CatLocation"), Value::List(vec![]));
        r.push(label("CatString"), Value::List(vec![]));
        r.push(label("ValBoolean"), Value::Void(vec![0x80, 0x40]));
        r.push(label("ValNumber"), Value::Void(vec![7]));
        r.push(label("ValLocation"), Value::Void(vec![]));
        r.push(label("ValString"), Value::List(vec![]));
        g
    }

    fn decls(names: &[(&str, GlobalKind)]) -> Declarations {
        let mut d = Declarations::new();
        for (n, k) in names {
            d.add(n, *k);
        }
        d
    }

    fn set(g: &mut Gff, text: &str, d: &Declarations, declare: bool) -> Result<GlobalChange> {
        set_global(g, "GLOBALVARS.res", &Assignment::parse(text)?, d, declare)
    }

    fn value(g: &Gff, name: &str) -> GlobalValue {
        crate::Globals::from_gff(g, "GLOBALVARS.res")
            .unwrap()
            .get(name)
            .unwrap()
            .value
            .clone()
    }

    #[test]
    fn a_boolean_set_changes_its_bit_alone() {
        let d = decls(&[("B1", GlobalKind::Boolean), ("B9", GlobalKind::Boolean)]);
        let mut g = gvt();
        let c = set(&mut g, "b1=true", &d, false).unwrap();
        assert_eq!(c.name, "B1");
        assert_eq!(c.before, Some(GlobalValue::Boolean(false)));
        assert_eq!(
            g.root.get("ValBoolean"),
            Some(&Value::Void(vec![0xC0, 0x40]))
        );
        set(&mut g, "B9=0", &d, false).unwrap();
        assert_eq!(
            g.root.get("ValBoolean"),
            Some(&Value::Void(vec![0xC0, 0x00]))
        );
        let diff = rakata_gff::diff(&gvt(), &g);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].path.to_string(), "ValBoolean");
    }

    #[test]
    fn an_undeclared_global_is_refused_unless_declared_here() {
        let mut g = gvt();
        let e = set(&mut g, "NEW_FLAG=boolean:1", &Declarations::new(), false).unwrap_err();
        assert_eq!(e.code(), crate::codes::UNDECLARED);
        assert!(e.message().contains("NEW_FLAG"), "{}", e.message());
        assert_eq!(g, gvt());

        let c = set(&mut g, "NEW_FLAG=boolean:1", &Declarations::new(), true).unwrap();
        assert_eq!(c.before, None);
        assert!(!c.declared);
        assert_eq!(value(&g, "NEW_FLAG"), GlobalValue::Boolean(true));
        // Boolean 10 is the third bit of the second byte.
        assert_eq!(
            g.root.get("ValBoolean"),
            Some(&Value::Void(vec![0x80, 0x60]))
        );
        // Booleans 11 to 16: the seventeenth is the top bit of a third byte.
        for i in 11..=16 {
            set(
                &mut g,
                &format!("X{i}=boolean:{}", i % 2),
                &Declarations::new(),
                true,
            )
            .unwrap();
        }
        assert_eq!(
            g.root.get("ValBoolean"),
            Some(&Value::Void(vec![0x80, 0x75, 0x00]))
        );
        assert_eq!(value(&g, "B9"), GlobalValue::Boolean(true));
    }

    #[test]
    fn a_declared_name_the_save_lacks_is_added_with_the_installs_spelling_and_kind() {
        let d = decls(&[
            ("TAR_Count", GlobalKind::Number),
            ("S_Name", GlobalKind::String),
        ]);
        let mut g = gvt();
        let c = set(&mut g, "tar_count=-3", &d, false).unwrap();
        assert_eq!(c.name, "TAR_Count");
        assert!(c.declared);
        assert_eq!(value(&g, "TAR_COUNT"), GlobalValue::Number(-3));
        assert_eq!(value(&g, "N0"), GlobalValue::Number(7));
        set(&mut g, "s_name=a:b", &d, false).unwrap();
        assert_eq!(value(&g, "S_Name"), GlobalValue::String("a:b".into()));
    }

    #[test]
    fn values_and_types_that_do_not_fit_are_refused() {
        let d = decls(&[("B1", GlobalKind::Boolean), ("N0", GlobalKind::Number)]);
        let mut g = gvt();
        for text in [
            "B1=yes",
            "N0=200",
            "B1=number:1",
            "N0",
            "=1",
            "L=location:1,2",
        ] {
            let e = set(&mut g, text, &d, true).unwrap_err();
            assert_eq!(e.code(), crate::codes::BAD_ASSIGNMENT, "{text}");
        }
        let e = set(&mut g, "FRESH=1", &d, true).unwrap_err();
        assert!(
            e.message().contains("its type must be given"),
            "{}",
            e.message()
        );
        assert_eq!(g, gvt());
    }

    #[test]
    fn a_location_is_six_floats() {
        let mut g = gvt();
        set(
            &mut g,
            "L0=location:1,2,3,0,1,0",
            &Declarations::new(),
            true,
        )
        .unwrap();
        assert_eq!(
            value(&g, "L0"),
            GlobalValue::Location(Location {
                position: [1.0, 2.0, 3.0],
                orientation: [0.0, 1.0, 0.0]
            })
        );
    }

    #[test]
    fn the_party_keeps_members_it_already_has() {
        let mut pt = Gff::new(*b"PT  ");
        let mut m = Struct::new(0);
        m.push(label("PT_MEMBER_ID"), Value::Int(6))
            .push(label("PT_IS_LEADER"), Value::Byte(1));
        pt.root
            .push(label("PT_NUM_MEMBERS"), Value::Byte(1))
            .push(label("PT_MEMBERS"), Value::List(vec![m.clone()]));
        let c = set_party(&mut pt, "PARTYTABLE.res", &[2, 6]).unwrap();
        assert_eq!(c.before, vec![6]);
        assert_eq!(c.after, vec![2, 6]);
        let list = pt.root.get("PT_MEMBERS").unwrap().as_list().unwrap();
        assert_eq!(list[1], m);
        assert_eq!(int(&list[0], "PT_MEMBER_ID"), Some(2));
        assert_eq!(pt.root.get("PT_NUM_MEMBERS"), Some(&Value::Byte(2)));
    }
}
