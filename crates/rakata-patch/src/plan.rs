//! The install plan: what an install of a [`Changes`] would do, as
//! operations in the order TSLPatcher runs its phases, with the memory
//! tokens left as placeholders. A dry run prints it; the executor walks it.

use crate::changes::*;
use crate::issue::{Issue, codes};
use crate::value::{Token, Value};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fmt;

/// The phases, in the order TSLPatcher runs them (`docs/formats/changes-ini.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Tlk,
    Install,
    TwoDa,
    Gff,
    Hack,
    Compile,
    Ssf,
}

impl Phase {
    pub const ALL: [Phase; 7] = [
        Phase::Tlk,
        Phase::Install,
        Phase::TwoDa,
        Phase::Gff,
        Phase::Hack,
        Phase::Compile,
        Phase::Ssf,
    ];

    /// The section the phase reads.
    pub fn section(self) -> &'static str {
        match self {
            Phase::Tlk => "TLKList",
            Phase::Install => "InstallList",
            Phase::TwoDa => "2DAList",
            Phase::Gff => "GFFList",
            Phase::Hack => "HACKList",
            Phase::Compile => "CompileList",
            Phase::Ssf => "SSFList",
        }
    }
}

/// The plan: operations grouped by phase.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Plan {
    pub requirements: Vec<Requirement>,
    pub steps: Vec<Step>,
}

/// One operation and the phase it runs in.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Step {
    pub phase: Phase,
    #[serde(flatten)]
    pub op: Operation,
}

/// Where a file goes: a folder or a capsule under the game.
pub type Destination = String;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operation {
    /// Append the entries of `file` to dialog.tlk; `tokens` names which get a `StrRef#`.
    TlkAppend {
        file: String,
        tokens: Vec<TlkAppend>,
    },
    /// Write entries of `file` over existing dialog.tlk entries.
    TlkReplace {
        file: String,
        entries: Vec<TlkReplaceEntry>,
    },
    /// Copy a file from tslpatchdata.
    InstallFile {
        source: String,
        destination: Destination,
        name: String,
        replace: bool,
    },
    /// One 2DA operation on `file`.
    Table {
        file: String,
        key: String,
        kind: TableOpKind,
    },
    /// Edit or create a GFF file.
    Gff {
        file: String,
        destination: Destination,
        name: String,
        replace_file: bool,
        edits: Vec<GffEdit>,
        adds: Vec<AddField>,
    },
    /// Write bytes into a file.
    Hack {
        file: String,
        destination: Destination,
        replace_file: bool,
        writes: Vec<HackWrite>,
    },
    /// Compile a script, with the tokens in its source filled in.
    Compile {
        file: String,
        destination: Destination,
        name: String,
        replace: bool,
    },
    /// Set a soundset's entries.
    Ssf {
        file: String,
        replace: bool,
        entries: Vec<SsfEntry>,
    },
}

impl Plan {
    /// The plan of `changes`, and a warning (`RW0204`) for every token an
    /// operation reads before any operation sets it, in this order.
    pub fn build(changes: &Changes, file: usize, issues: &mut Vec<Issue>) -> Plan {
        let mut plan = Plan {
            requirements: changes.settings.required.clone(),
            steps: Vec::new(),
        };
        let mut set: BTreeSet<Token> = BTreeSet::new();
        let mut check = |reads: Vec<(Token, u32)>, sets: Vec<Token>, issues: &mut Vec<Issue>| {
            for (t, line) in reads {
                if !set.contains(&t) {
                    issues.push(Issue::new(
                        codes::TOKEN_UNSET,
                        format!("`{t}` is read here, and nothing before this operation sets it"),
                        file,
                        line,
                    ));
                }
            }
            set.extend(sets);
        };

        // TLK: the appends set every StrRef# token.
        let tlk = &changes.tlk;
        if !tlk.appends.is_empty() || tlk.source_file.is_some() {
            let file_name = tlk
                .source_file
                .clone()
                .unwrap_or_else(|| "append.tlk".to_string());
            check(
                Vec::new(),
                tlk.appends.iter().map(|a| Token::StrRef(a.token)).collect(),
                issues,
            );
            plan.steps.push(Step {
                phase: Phase::Tlk,
                op: Operation::TlkAppend {
                    file: file_name,
                    tokens: tlk.appends.clone(),
                },
            });
        }
        for r in &tlk.replacements {
            plan.steps.push(Step {
                phase: Phase::Tlk,
                op: Operation::TlkReplace {
                    file: r.file.clone(),
                    entries: r.entries.clone(),
                },
            });
        }

        for folder in &changes.install {
            for f in &folder.files {
                plan.steps.push(Step {
                    phase: Phase::Install,
                    op: Operation::InstallFile {
                        source: f.source_file.clone().unwrap_or_else(|| f.name.clone()),
                        destination: folder.destination.clone(),
                        name: f.save_as.clone().unwrap_or_else(|| f.name.clone()),
                        replace: f.replace,
                    },
                });
            }
        }

        for table in &changes.tables {
            for op in &table.ops {
                let (reads, sets) = table_tokens(&op.kind);
                check(
                    reads.into_iter().map(|t| (t, op.line)).collect(),
                    sets,
                    issues,
                );
                plan.steps.push(Step {
                    phase: Phase::TwoDa,
                    op: Operation::Table {
                        file: table.file.clone(),
                        key: op.key.clone(),
                        kind: op.kind.clone(),
                    },
                });
            }
        }

        for g in &changes.gff {
            // Edits and additions run in the order the section lists them,
            // an addition's children depth-first after its own values.
            let mut events: Vec<TokenEvent> = Vec::new();
            for e in &g.edits {
                let mut reads = Vec::new();
                if let FieldPath::Memory(slot) = e.path {
                    reads.push((Token::Memory(slot), e.line));
                }
                if let Some(t) = e.value.token() {
                    reads.push((t, e.line));
                }
                events.push((e.line, reads, Vec::new()));
            }
            for a in &g.adds {
                let (mut reads, mut sets) = (Vec::new(), Vec::new());
                add_field_tokens(a, &mut reads, &mut sets);
                events.push((a.line, reads, sets));
            }
            events.sort_by_key(|(line, _, _)| *line);
            for (_, reads, sets) in events {
                check(reads, sets, issues);
            }
            plan.steps.push(Step {
                phase: Phase::Gff,
                op: Operation::Gff {
                    file: g.file.clone(),
                    destination: g.destination.clone().unwrap_or_else(override_folder),
                    name: g
                        .save_as
                        .clone()
                        .or_else(|| g.filename.clone())
                        .unwrap_or_else(|| g.file.clone()),
                    replace_file: g.replace_file,
                    edits: g.edits.clone(),
                    adds: g.adds.clone(),
                },
            });
        }

        for h in &changes.hacks {
            let reads = h
                .writes
                .iter()
                .filter_map(|w| w.value.token().map(|t| (t, w.line)))
                .collect();
            check(reads, Vec::new(), issues);
            plan.steps.push(Step {
                phase: Phase::Hack,
                op: Operation::Hack {
                    file: h.file.clone(),
                    destination: h.destination.clone().unwrap_or_else(override_folder),
                    replace_file: h.replace_file,
                    writes: h.writes.clone(),
                },
            });
        }

        for s in &changes.compile.scripts {
            let ncs = s
                .save_as
                .clone()
                .unwrap_or_else(|| replace_extension(&s.file, "ncs"));
            plan.steps.push(Step {
                phase: Phase::Compile,
                op: Operation::Compile {
                    file: s.file.clone(),
                    destination: s
                        .destination
                        .clone()
                        .or_else(|| changes.compile.default_destination.clone())
                        .unwrap_or_else(override_folder),
                    name: ncs,
                    replace: s.replace,
                },
            });
        }

        for s in &changes.ssf {
            let reads = s
                .entries
                .iter()
                .filter_map(|e| e.value.token().map(|t| (t, e.line)))
                .collect();
            check(reads, Vec::new(), issues);
            plan.steps.push(Step {
                phase: Phase::Ssf,
                op: Operation::Ssf {
                    file: s.file.clone(),
                    replace: s.replace,
                    entries: s.entries.clone(),
                },
            });
        }
        plan
    }

    /// The steps of one phase.
    pub fn phase(&self, phase: Phase) -> impl Iterator<Item = &Step> {
        self.steps.iter().filter(move |s| s.phase == phase)
    }
}

/// One operation's place in the order (a line), the tokens it reads with
/// the line of each read, and the tokens it sets.
type TokenEvent = (u32, Vec<(Token, u32)>, Vec<Token>);

fn override_folder() -> String {
    "override".to_string()
}

fn replace_extension(name: &str, ext: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, _)) => format!("{stem}.{ext}"),
        None => format!("{name}.{ext}"),
    }
}

/// The tokens a 2DA operation reads and the ones it sets.
fn table_tokens(kind: &TableOpKind) -> (Vec<Token>, Vec<Token>) {
    let mut reads = Vec::new();
    let mut sets = Vec::new();
    fn cells(reads: &mut Vec<Token>, cells: &[Cell]) {
        reads.extend(cells.iter().filter_map(|c| c.value.token()));
    }
    fn target(reads: &mut Vec<Token>, t: &Option<RowTarget>) {
        if let Some(RowTarget::Label(l) | RowTarget::LabelIndex(l)) = t {
            reads.extend(Value::parse(l).token());
        }
    }
    match kind {
        TableOpKind::ChangeRow(op) => {
            target(&mut reads, &op.target);
            cells(&mut reads, &op.cells);
            sets.extend(op.stores.iter().map(|s| Token::Memory(s.slot)));
        }
        TableOpKind::AddRow(op) => {
            cells(&mut reads, &op.cells);
            reads.extend(op.row_label.as_ref().and_then(Value::token));
            sets.extend(op.stores.iter().map(|s| Token::Memory(s.slot)));
        }
        TableOpKind::CopyRow(op) => {
            target(&mut reads, &op.target);
            cells(&mut reads, &op.cells);
            reads.extend(op.new_row_label.as_ref().and_then(Value::token));
            sets.extend(op.stores.iter().map(|s| Token::Memory(s.slot)));
        }
        TableOpKind::AddColumn(op) => {
            reads.extend(op.default.as_ref().and_then(Value::token));
            reads.extend(op.cells.iter().filter_map(|c| c.value.token()));
            sets.extend(op.stores.iter().map(|s| Token::Memory(s.slot)));
        }
    }
    (reads, sets)
}

fn add_field_tokens(a: &AddField, reads: &mut Vec<(Token, u32)>, sets: &mut Vec<Token>) {
    for v in [&a.value, &a.strref].into_iter().flatten() {
        if let Some(t) = v.token() {
            reads.push((t, a.line));
        }
    }
    sets.extend(a.stores.iter().map(|s| Token::Memory(s.slot)));
    for c in &a.children {
        add_field_tokens(c, reads, sets);
    }
}

impl fmt::Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for r in &self.requirements {
            writeln!(f, "requires override\\{}", r.file)?;
        }
        for phase in Phase::ALL {
            let steps: Vec<&Step> = self.phase(phase).collect();
            if steps.is_empty() {
                continue;
            }
            writeln!(f, "[{}]", phase.section())?;
            for s in steps {
                write!(f, "{}", s.op)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operation::TlkAppend { file, tokens } => {
                writeln!(
                    f,
                    "  append {file} to dialog.tlk ({} entries with tokens)",
                    tokens.len()
                )?;
                for t in tokens {
                    writeln!(f, "    StrRef{} = entry {}", t.token, t.index)?;
                }
            }
            Operation::TlkReplace { file, entries } => {
                writeln!(
                    f,
                    "  replace {} dialog.tlk entries from {file}",
                    entries.len()
                )?;
                for e in entries {
                    writeln!(f, "    dialog.tlk[{}] = entry {}", e.strref, e.index)?;
                }
            }
            Operation::InstallFile {
                source,
                destination,
                name,
                replace,
            } => {
                let verb = if *replace { "replace" } else { "copy" };
                if source == name {
                    writeln!(f, "  {verb} {name} -> {destination}")?;
                } else {
                    writeln!(f, "  {verb} {source} -> {destination}\\{name}")?;
                }
            }
            Operation::Table { file, key, kind } => {
                write!(f, "  {file}: {key} ")?;
                match kind {
                    TableOpKind::ChangeRow(op) => {
                        writeln!(f, "change row {}", target(&op.target))?;
                        cells(f, &op.cells)?;
                        stores(f, &op.stores)?;
                    }
                    TableOpKind::AddRow(op) => {
                        write!(f, "add row")?;
                        if let Some(c) = &op.exclusive_column {
                            write!(f, " unless a row's `{c}` matches")?;
                        }
                        if let Some(l) = &op.row_label {
                            write!(f, " labelled {l}")?;
                        }
                        writeln!(f)?;
                        cells(f, &op.cells)?;
                        stores(f, &op.stores)?;
                    }
                    TableOpKind::CopyRow(op) => {
                        write!(f, "copy row {}", target(&op.target))?;
                        if let Some(c) = &op.exclusive_column {
                            write!(f, " unless a row's `{c}` matches")?;
                        }
                        if let Some(l) = &op.new_row_label {
                            write!(f, " as {l}")?;
                        }
                        writeln!(f)?;
                        cells(f, &op.cells)?;
                        stores(f, &op.stores)?;
                    }
                    TableOpKind::AddColumn(op) => {
                        write!(f, "add column {}", op.label)?;
                        if let Some(d) = &op.default {
                            write!(f, " default {d}")?;
                        }
                        writeln!(f)?;
                        for c in &op.cells {
                            writeln!(f, "      {} = {}", row_ref(&c.row), c.value)?;
                        }
                        for s in &op.stores {
                            writeln!(f, "      2DAMEMORY{} <- {}", s.slot, row_ref(&s.row))?;
                        }
                    }
                }
            }
            Operation::Gff {
                file,
                destination,
                name,
                replace_file,
                edits,
                adds,
            } => {
                let how = if *replace_file {
                    "replace then edit"
                } else {
                    "edit"
                };
                write!(f, "  {how} {destination}\\{name}")?;
                if name != file {
                    write!(f, " (from {file})")?;
                }
                writeln!(f, ": {} edits, {} additions", edits.len(), adds.len())?;
                for e in edits {
                    let path = match &e.path {
                        FieldPath::Literal(p) => p.clone(),
                        FieldPath::Memory(n) => format!("<path in 2DAMEMORY{n}>"),
                    };
                    let what = match e.target {
                        FieldTarget::Value => String::new(),
                        FieldTarget::StrRef => "(strref)".to_string(),
                        FieldTarget::Substring(l) => format!("(lang{l})"),
                    };
                    writeln!(f, "      {path}{what} = {}", e.value)?;
                }
                for a in adds {
                    add_field(f, a, 6)?;
                }
            }
            Operation::Hack {
                file,
                destination,
                replace_file,
                writes,
            } => {
                let how = if *replace_file {
                    "replace then patch"
                } else {
                    "patch"
                };
                writeln!(f, "  {how} {destination}\\{file}: {} writes", writes.len())?;
                for w in writes {
                    writeln!(f, "      0x{:x} = {}", w.offset, w.value)?;
                }
            }
            Operation::Compile {
                file,
                destination,
                name,
                replace,
            } => {
                let verb = if *replace {
                    "compile, replacing"
                } else {
                    "compile"
                };
                writeln!(f, "  {verb} {file} -> {destination}\\{name}")?;
            }
            Operation::Ssf {
                file,
                replace,
                entries,
            } => {
                let verb = if *replace { "replace" } else { "edit" };
                writeln!(f, "  {verb} {file}: {} sounds", entries.len())?;
                for e in entries {
                    writeln!(f, "      {} = {}", e.sound, e.value)?;
                }
            }
        }
        Ok(())
    }
}

fn target(t: &Option<RowTarget>) -> String {
    match t {
        Some(RowTarget::Index(n)) => format!("#{n}"),
        Some(RowTarget::Label(l)) => format!("labelled {l}"),
        Some(RowTarget::LabelIndex(l)) => format!("with label column {l}"),
        None => "(no target)".to_string(),
    }
}

fn row_ref(r: &RowRef) -> String {
    match r {
        RowRef::Index(n) => format!("row #{n}"),
        RowRef::Label(l) => format!("row labelled {l}"),
    }
}

fn cells(f: &mut fmt::Formatter<'_>, cells: &[Cell]) -> fmt::Result {
    for c in cells {
        writeln!(f, "      {} = {}", c.column, c.value)?;
    }
    Ok(())
}

fn stores(f: &mut fmt::Formatter<'_>, stores: &[Store]) -> fmt::Result {
    for s in stores {
        let src = match &s.source {
            StoreSource::RowIndex => "RowIndex".to_string(),
            StoreSource::RowLabel => "RowLabel".to_string(),
            StoreSource::Column(c) => c.clone(),
        };
        writeln!(f, "      2DAMEMORY{} <- {src}", s.slot)?;
    }
    Ok(())
}

fn add_field(f: &mut fmt::Formatter<'_>, a: &AddField, indent: usize) -> fmt::Result {
    let pad = " ".repeat(indent);
    write!(f, "{pad}add {:?} {}", a.field_type, a.label)?;
    if let Some(p) = &a.path {
        write!(f, " at {p}")?;
    }
    if let Some(v) = &a.value {
        write!(f, " = {v}")?;
    }
    if let Some(s) = &a.strref {
        write!(f, " strref {s}")?;
    }
    for (lang, text) in &a.substrings {
        write!(f, " lang{lang}={text:?}")?;
    }
    if let Some(t) = a.type_id {
        write!(f, " type {t}")?;
    }
    for s in &a.stores {
        let what = match s.what {
            AddStoreWhat::ListIndex => "ListIndex",
            AddStoreWhat::FieldPath => "!FieldPath",
        };
        write!(f, " 2DAMEMORY{} <- {what}", s.slot)?;
    }
    writeln!(f)?;
    for c in &a.children {
        add_field(f, c, indent + 2)?;
    }
    Ok(())
}
