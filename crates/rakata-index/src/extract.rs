//! From one resource's bytes to its rows: the format crates' extractors
//! (`rakata_dialog::facts`, `rakata_gff::facts`, `rakata_2da::facts`,
//! `rakata_tlk::facts`, `nwscript_ncs::facts`) chosen by the resource's
//! type, and their facts laid out as the tables' columns. Nothing here
//! touches the database or the file system.

use nwscript_ncs::facts::{ArgType, Literal};
use rakata_res::{Ref, RefKind, ResType};
use rusqlite::types::Value;
use serde_json::json;

/// One engine routine as the tree's `nwscript.nss` declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routine {
    pub name: String,
    pub params: Vec<Param>,
    /// What it returns.
    pub ret: ArgType,
}

/// One parameter of a routine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: ArgType,
}

/// What the index made of a content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Status {
    /// A reader read it; its facts are in the tables.
    Parsed,
    /// An archive; its members have rows of their own.
    Container,
    /// No reader for its type: located and hashed only.
    NoReader,
    /// A reader rejected it; the reason says why.
    Failed,
    /// A file of the tree outside the folders the game loads resources
    /// from: not read.
    Outside,
}

impl Status {
    pub const ALL: [Status; 5] = [
        Status::Parsed,
        Status::Container,
        Status::NoReader,
        Status::Failed,
        Status::Outside,
    ];

    /// The name the `coverage` and `contents` tables store.
    pub const fn name(self) -> &'static str {
        match self {
            Status::Parsed => "parsed",
            Status::Container => "container",
            Status::NoReader => "no_reader",
            Status::Failed => "failed",
            Status::Outside => "outside",
        }
    }
}

/// A fact table a row goes to. The writer prepends the content hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Table {
    DialogNodes,
    DialogLinks,
    GffFields,
    Cells,
    Strings,
    Calls,
    Refs,
    Search,
}

impl Table {
    /// The insert statement, the hash first.
    pub(crate) const fn insert(self) -> &'static str {
        match self {
            Table::DialogNodes => {
                "INSERT INTO dialog_nodes VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)"
            }
            Table::DialogLinks => "INSERT INTO dialog_links VALUES (?1,?2,?3,?4,?5,?6)",
            Table::GffFields => "INSERT INTO gff_fields VALUES (?1,?2,?3,?4,?5,?6)",
            Table::Cells => "INSERT INTO cells VALUES (?1,?2,?3,?4,?5)",
            Table::Strings => "INSERT INTO strings VALUES (?1,?2,?3,?4,?5,?6)",
            Table::Calls => "INSERT INTO calls VALUES (?1,?2,?3,?4,?5)",
            Table::Refs => "INSERT INTO refs VALUES (?1,?2,?3,?4)",
            Table::Search => "INSERT INTO search(hash, body, kind, site) VALUES (?1,?2,?3,?4)",
        }
    }
}

/// A content's rows, before the hash is attached.
#[derive(Debug, Default)]
pub(crate) struct Extracted {
    pub status: Option<Status>,
    pub reason: Option<String>,
    pub rows: Vec<(Table, Vec<Value>)>,
}

impl Extracted {
    fn status(status: Status, reason: Option<String>) -> Self {
        Extracted {
            status: Some(status),
            reason,
            rows: Vec::new(),
        }
    }

    fn push(&mut self, table: Table, row: Vec<Value>) {
        self.rows.push((table, row));
    }

    fn refs(&mut self, refs: Vec<Ref>) {
        for r in refs {
            self.push(
                Table::Refs,
                vec![
                    text(r.kind.name()),
                    Value::Text(r.value),
                    Value::Text(r.site),
                ],
            );
        }
    }

    fn search(&mut self, body: String, kind: &str, site: String) {
        if !body.trim().is_empty() {
            self.push(
                Table::Search,
                vec![Value::Text(body), text(kind), Value::Text(site)],
            );
        }
    }
}

fn text(s: &str) -> Value {
    Value::Text(s.to_string())
}

fn opt_text(s: Option<String>) -> Value {
    s.map_or(Value::Null, Value::Text)
}

fn opt_int(v: Option<impl Into<i64>>) -> Value {
    v.map_or(Value::Null, |v| Value::Integer(v.into()))
}

/// The resource types read as GFF.
pub fn is_gff(ty: ResType) -> bool {
    use ResType::*;
    matches!(
        ty,
        Res | Are
            | Ifo
            | Bic
            | Git
            | Bti
            | Uti
            | Btc
            | Utc
            | Dlg
            | Itp
            | Btt
            | Utt
            | Bts
            | Uts
            | Gff
            | Fac
            | Bte
            | Ute
            | Btd
            | Utd
            | Btp
            | Utp
            | Gic
            | Gui
            | Btm
            | Utm
            | Btg
            | Utg
            | Jrl
            | Utw
            | Ptm
            | Ptt
            | Pth
    )
}

/// The resource types read as archives, whose members the build lists.
pub fn is_archive(ty: ResType) -> bool {
    matches!(
        ty,
        ResType::Key
            | ResType::Bif
            | ResType::Erf
            | ResType::Mod
            | ResType::Sav
            | ResType::Hak
            | ResType::Rim
    )
}

/// The rows of the resource `name` of type `ty`.
pub(crate) fn extract(ty: ResType, name: &str, bytes: &[u8], routines: &[Routine]) -> Extracted {
    if ty == ResType::Dlg {
        dialog(name, bytes)
    } else if is_gff(ty) {
        gff(bytes)
    } else {
        match ty {
            ResType::TwoDa => twoda(name, bytes),
            ResType::Tlk => tlk(bytes),
            ResType::Ncs => script(bytes, routines),
            ResType::Nss => source(bytes),
            _ => Extracted::status(Status::NoReader, None),
        }
    }
}

fn failed(e: impl std::fmt::Display) -> Extracted {
    Extracted::status(Status::Failed, Some(e.to_string()))
}

fn dialog(name: &str, bytes: &[u8]) -> Extracted {
    let d = match rakata_dialog::Dialog::parse(format!("{name}.dlg"), bytes) {
        Ok(d) => d,
        Err(e) => return failed(e),
    };
    let f = rakata_dialog::facts::facts(&d);
    let mut out = Extracted::status(Status::Parsed, None);
    for n in f.nodes {
        let node = n.at.to_string();
        if let Some(t) = &n.text {
            out.search(t.clone(), "node", node.clone());
        }
        if let Some(c) = &n.comment {
            out.search(c.clone(), "comment", node.clone());
        }
        let animations = if n.animations.is_empty() {
            Value::Null
        } else {
            Value::Text(
                serde_json::to_string(
                    &n.animations
                        .iter()
                        .map(|a| json!({"participant": a.participant, "animation": a.animation}))
                        .collect::<Vec<_>>(),
                )
                .expect("JSON serializes"),
            )
        };
        out.push(
            Table::DialogNodes,
            vec![
                Value::Text(node),
                text(n.at.kind.name()),
                Value::Integer(n.at.index as i64),
                opt_text(n.speaker),
                opt_text(n.listener),
                opt_int(n.strref),
                opt_text(n.text),
                opt_text(n.voice),
                opt_text(n.sound),
                opt_text(n.script),
                opt_int(n.delay),
                opt_int(n.camera_angle),
                opt_int(n.camera_id),
                opt_int(n.camera_animation),
                opt_text(n.quest),
                opt_int(n.quest_entry),
                opt_text(n.comment),
                animations,
            ],
        );
    }
    for l in f.links {
        out.push(
            Table::DialogLinks,
            vec![
                Value::Text(
                    l.from
                        .map_or_else(|| "start".to_string(), |n| n.to_string()),
                ),
                Value::Integer(l.position as i64),
                Value::Text(l.to.to_string()),
                opt_text(l.condition),
                Value::Integer(i64::from(l.is_child)),
            ],
        );
    }
    out.refs(f.refs);
    out
}

fn gff(bytes: &[u8]) -> Extracted {
    let g = match rakata_gff::Gff::parse(bytes) {
        Ok(g) => g,
        Err(e) => return failed(e),
    };
    let f = rakata_gff::facts::facts(&g);
    let mut out = Extracted::status(Status::Parsed, None);
    for x in f.fields {
        use rakata_gff::facts::FactValue;
        let value = match x.value {
            FactValue::Null => Value::Null,
            FactValue::Int(v) => Value::Integer(v),
            FactValue::Real(v) => Value::Real(v),
            FactValue::Bytes(b) => Value::Text(rakata_tlk::codepage::decode(&b)),
            FactValue::Text(s) => Value::Text(s),
        };
        out.push(
            Table::GffFields,
            vec![
                Value::Text(x.path),
                Value::Text(x.label),
                text(x.ty.name()),
                value,
                opt_int(x.strref),
            ],
        );
    }
    out.refs(f.refs);
    out
}

fn twoda(name: &str, bytes: &[u8]) -> Extracted {
    let t = match rakata_2da::TwoDa::parse(bytes) {
        Ok(t) => t,
        Err(e) => return failed(e),
    };
    let f = rakata_2da::facts::facts(&t, name);
    let mut out = Extracted::status(Status::Parsed, None);
    for c in f.cells {
        out.push(
            Table::Cells,
            vec![
                Value::Integer(c.row as i64),
                text(c.label),
                text(c.column),
                text(c.value),
            ],
        );
    }
    out.refs(f.refs);
    out
}

fn tlk(bytes: &[u8]) -> Extracted {
    let t = match rakata_tlk::Tlk::parse(bytes) {
        Ok(t) => t,
        Err(e) => return failed(e),
    };
    let f = rakata_tlk::facts::facts(&t);
    let mut out = Extracted::status(Status::Parsed, None);
    for s in f.strings {
        out.search(s.text.to_string(), "string", s.strref.to_string());
        out.push(
            Table::Strings,
            vec![
                Value::Integer(i64::from(s.strref)),
                text(s.text),
                opt_text(
                    s.sound
                        .map(|r| String::from_utf8_lossy(r.as_bytes()).to_ascii_lowercase()),
                ),
                Value::Real(f64::from(s.sound_length)),
                Value::Integer(i64::from(s.flags)),
            ],
        );
    }
    out.refs(f.refs);
    out
}

fn source(bytes: &[u8]) -> Extracted {
    let mut out = Extracted::status(Status::Parsed, None);
    out.search(rakata_tlk::codepage::decode(bytes), "source", String::new());
    out
}

/// A literal as `calls.args` holds it: the conventions of the corpus's
/// `calls.jsonl`.
fn literal_json(l: &Literal) -> serde_json::Value {
    match l {
        Literal::Int(v) => json!(v),
        Literal::Float(v) => json!(f64::from(*v)),
        Literal::String(b) => json!(rakata_tlk::codepage::decode(b)),
        Literal::Object(0) => json!({"object": "OBJECT_SELF"}),
        Literal::Object(1) => json!({"object": "OBJECT_INVALID"}),
        Literal::Object(v) => json!({"object": v}),
        Literal::Vector(v) => json!(v.map(f64::from)),
    }
}

/// The kind of symbol a literal argument names, read from the routine's
/// declaration: a string passed to `GetGlobal*`/`SetGlobal*` names a
/// global; a string parameter whose name speaks of a tag (`sTag`,
/// `sWaypointTag`) names a tag, one that speaks of a script, template,
/// resref, dialogue, sound, movie or module names a resource; an integer
/// parameter named for a strref names a line of the talk table.
pub fn ref_kind(routine: &str, param: &str, lit: &Literal) -> Option<RefKind> {
    let p = param.to_ascii_lowercase();
    match lit {
        Literal::String(_) => {
            if routine.starts_with("GetGlobal") || routine.starts_with("SetGlobal") {
                Some(RefKind::Global)
            } else if p.contains("tag") {
                Some(RefKind::Tag)
            } else if [
                "script", "template", "resref", "dialog", "sound", "movie", "module",
            ]
            .iter()
            .any(|w| p.contains(w))
            {
                Some(RefKind::ResRef)
            } else {
                None
            }
        }
        Literal::Int(_) => {
            (p.contains("strref") || p.contains("stringreference")).then_some(RefKind::StrRef)
        }
        _ => None,
    }
}

fn script(bytes: &[u8], routines: &[Routine]) -> Extracted {
    let program = match nwscript_ncs::decode(nwscript_diag::FileId(0), bytes) {
        Ok(p) => p,
        Err(d) => return failed(format!("{}: {}", d.code.id, d.message)),
    };
    let param = |r: u16, k: usize| {
        routines
            .get(usize::from(r))
            .and_then(|f| f.params.get(k))
            .map(|p| p.ty)
    };
    let ret = |r: u16| routines.get(usize::from(r)).map(|f| f.ret);
    let mut out = Extracted::status(Status::Parsed, None);
    let mut refs = Vec::new();
    for c in nwscript_ncs::facts::calls(&program, param, ret) {
        let f = routines.get(usize::from(c.routine));
        let name = f.map_or_else(|| format!("routine_{}", c.routine), |f| f.name.clone());
        let args: Vec<serde_json::Value> = c
            .args
            .iter()
            .map(|a| a.as_ref().map_or(serde_json::Value::Null, literal_json))
            .collect();
        let strings: Vec<String> = c
            .args
            .iter()
            .filter_map(|a| match a {
                Some(Literal::String(b)) => Some(rakata_tlk::codepage::decode(b)),
                _ => None,
            })
            .collect();
        let site = format!("@{}", c.offset);
        if !strings.is_empty() {
            out.search(
                format!("{name} {}", strings.join(" ")),
                "call",
                site.clone(),
            );
        }
        for (k, a) in c.args.iter().enumerate() {
            let (Some(lit), Some(p)) = (a, f.and_then(|f| f.params.get(k))) else {
                continue;
            };
            let Some(kind) = ref_kind(&name, &p.name, lit) else {
                continue;
            };
            let value = match lit {
                Literal::String(b) => rakata_tlk::codepage::decode(b),
                Literal::Int(v) => (*v as u32).to_string(),
                _ => continue,
            };
            if value.is_empty() {
                continue;
            }
            let site = format!("@{} {name}.{}", c.offset, p.name);
            refs.push(match kind {
                RefKind::ResRef => Ref::resref(&value, site),
                kind => Ref { kind, value, site },
            });
        }
        out.push(
            Table::Calls,
            vec![
                Value::Integer(i64::from(c.offset)),
                Value::Text(name),
                Value::Integer(i64::from(c.routine)),
                Value::Text(serde_json::to_string(&args).expect("JSON serializes")),
            ],
        );
    }
    out.refs(refs);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_arguments_name_symbols_by_their_parameter() {
        let s = Literal::String(b"x".to_vec());
        assert_eq!(
            ref_kind("ExecuteScript", "sScript", &s),
            Some(RefKind::ResRef)
        );
        assert_eq!(ref_kind("GetObjectByTag", "sTag", &s), Some(RefKind::Tag));
        assert_eq!(
            ref_kind("GetGlobalBoolean", "sIdentifier", &s),
            Some(RefKind::Global)
        );
        assert_eq!(
            ref_kind("CreateObject", "sTemplate", &s),
            Some(RefKind::ResRef)
        );
        assert_eq!(ref_kind("SpeakString", "sStringToSpeak", &s), None);
        let n = Literal::Int(42724);
        assert_eq!(
            ref_kind("GetStringByStrRef", "nStrRef", &n),
            Some(RefKind::StrRef)
        );
        assert_eq!(ref_kind("GetHitDice", "nX", &n), None);
    }

    #[test]
    fn a_script_yields_calls_refs_and_search_rows() {
        let program = nwscript_ncs::asm::parse(
            nwscript_diag::FileId(0),
            b"JSR main\nRETN\nmain:\nCONSTI -1\nCONSTO 0\nCONSTS \"k_pman_aioff\"\nACTION 0, 3\nRETN\n",
        )
        .unwrap();
        let bytes = nwscript_ncs::encode(&program);
        let routines = vec![Routine {
            name: "ExecuteScript".into(),
            params: vec![
                Param {
                    name: "sScript".into(),
                    ty: ArgType::String,
                },
                Param {
                    name: "oTarget".into(),
                    ty: ArgType::Object,
                },
                Param {
                    name: "nScriptVar".into(),
                    ty: ArgType::Int,
                },
            ],
            ret: ArgType::Void,
        }];
        let x = extract(ResType::Ncs, "k_pdan_juhani27", &bytes, &routines);
        assert_eq!(x.status, Some(Status::Parsed));
        let call = x.rows.iter().find(|(t, _)| *t == Table::Calls).unwrap();
        assert_eq!(call.1[1], text("ExecuteScript"));
        assert_eq!(
            call.1[3],
            text(r#"["k_pman_aioff",{"object":"OBJECT_SELF"},-1]"#)
        );
        let r = x.rows.iter().find(|(t, _)| *t == Table::Refs).unwrap();
        assert_eq!(r.1[0], text("resref"));
        assert_eq!(r.1[1], text("k_pman_aioff"));
        assert!(x.rows.iter().any(|(t, row)| *t == Table::Search
            && row[0] == text("ExecuteScript k_pman_aioff")));
    }

    #[test]
    fn unknown_types_have_no_reader_and_bad_bytes_fail() {
        let x = extract(ResType::Tpc, "x", b"", &[]);
        assert_eq!(x.status, Some(Status::NoReader));
        let x = extract(ResType::Utc, "x", b"not a gff", &[]);
        assert_eq!(x.status, Some(Status::Failed));
        assert!(x.reason.unwrap().contains("RE06"));
    }
}
