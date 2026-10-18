//! The typed model of a `changes.ini`, built from an [`Ini`] with
//! TSLPatcher's semantics as `docs/formats/changes-ini.md` records them.
//! Every list keeps file order; every key the vocabulary does not know is
//! kept on its owner's `unknown` and reported, never dropped.

use crate::ini::{Entry, Ini, Section};
use crate::issue::{Issue, codes};
use crate::value::{Value, memory_key, token_number};
use serde::Serialize;
use std::collections::HashSet;

/// Everything a `changes.ini` says, in the order its phases run.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Changes {
    pub settings: Settings,
    pub tlk: TlkList,
    pub install: Vec<InstallFolder>,
    pub tables: Vec<TableChanges>,
    pub gff: Vec<GffChanges>,
    pub hacks: Vec<HackChanges>,
    pub compile: CompileList,
    pub ssf: Vec<SsfChanges>,
}

/// `[Settings]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Settings {
    pub window_caption: Option<String>,
    pub confirm_message: Option<String>,
    pub log_level: Option<u8>,
    pub installer_mode: Option<bool>,
    pub backup_files: Option<bool>,
    pub plaintext_log: Option<bool>,
    pub lookup_game_folder: Option<bool>,
    pub lookup_game_number: Option<u8>,
    pub save_processed_scripts: Option<bool>,
    pub file_exists: Option<bool>,
    /// `Required`/`Required#` with the `RequiredMsg`/`RequiredMsg#` of the same rank.
    pub required: Vec<Requirement>,
    pub unknown: Vec<Entry>,
}

/// A file that must exist in `override` before the install proceeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Requirement {
    pub file: String,
    /// What the log says when it is missing; TSLPatcher's own text when `None`.
    pub message: Option<String>,
}

/// `[TLKList]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TlkList {
    /// `StrRef#=N`: token `StrRef#` is the dialog.tlk index entry `N` of `append.tlk` gets.
    pub appends: Vec<TlkAppend>,
    /// `Replace#=FILE` / `ReplaceFile#=FILE`: entries of `FILE` written over existing dialog.tlk entries.
    pub replacements: Vec<TlkReplaceFile>,
    /// `!SourceFile`: the file appended instead of `append.tlk`.
    pub source_file: Option<String>,
    /// `!SourceFileF`: the same for the female dialog.tlk.
    pub source_file_f: Option<String>,
    pub unknown: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TlkAppend {
    pub token: u32,
    pub index: u32,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TlkReplaceFile {
    pub key: String,
    pub file: String,
    /// `STRREF=N` under `[FILE]`: dialog.tlk entry `STRREF` takes entry `N` of `FILE`.
    pub entries: Vec<TlkReplaceEntry>,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TlkReplaceEntry {
    pub strref: u32,
    pub index: u32,
    pub line: u32,
}

/// One `install_folder#=DEST` of `[InstallList]` with its `[install_folder#]` files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallFolder {
    pub key: String,
    /// A folder under the game (`Override`, `modules`) or a capsule in one (`modules\danm13.mod`).
    pub destination: String,
    pub files: Vec<InstallFile>,
    pub line: u32,
    pub unknown: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallFile {
    pub name: String,
    /// `Replace#` overwrites; `File#` skips a file that exists.
    pub replace: bool,
    /// `!SaveAs` in the file's own `[NAME]` section: the name it is installed under.
    pub save_as: Option<String>,
    /// `!SourceFile` there: the file in `tslpatchdata` to copy instead of `NAME`.
    pub source_file: Option<String>,
    pub line: u32,
}

/// One `Table#=FILE.2da` of `[2DAList]` with the operations of `[FILE.2da]`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TableChanges {
    pub file: String,
    pub ops: Vec<TableOp>,
    pub line: u32,
    pub unknown: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TableOp {
    /// `ChangeRow0`, as written.
    pub key: String,
    /// The section the operation's keys came from.
    pub section: String,
    pub kind: TableOpKind,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum TableOpKind {
    ChangeRow(RowChange),
    AddRow(RowAdd),
    CopyRow(RowCopy),
    AddColumn(ColumnAdd),
}

/// Which existing row an operation means.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "by", content = "value", rename_all = "snake_case")]
pub enum RowTarget {
    /// `RowIndex=N`: the N-th row.
    Index(u32),
    /// `RowLabel=L`: the row whose label is `L`.
    Label(String),
    /// `LabelIndex=L`: the row whose `label` column is `L`.
    LabelIndex(String),
}

/// A column assignment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cell {
    pub column: String,
    pub value: Value,
}

/// `2DAMEMORY#=SOURCE`: store something of the row in slot `#`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Store {
    pub slot: u32,
    pub source: StoreSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "source", content = "column", rename_all = "snake_case")]
pub enum StoreSource {
    RowIndex,
    RowLabel,
    Column(String),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RowChange {
    pub target: Option<RowTarget>,
    pub cells: Vec<Cell>,
    pub stores: Vec<Store>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RowAdd {
    /// Add only when no row has this column's value; else change that row.
    pub exclusive_column: Option<String>,
    /// `RowLabel=`; the new row's index when absent.
    pub row_label: Option<Value>,
    pub cells: Vec<Cell>,
    pub stores: Vec<Store>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RowCopy {
    pub target: Option<RowTarget>,
    pub exclusive_column: Option<String>,
    pub new_row_label: Option<Value>,
    pub cells: Vec<Cell>,
    pub stores: Vec<Store>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ColumnAdd {
    pub label: String,
    pub default: Option<Value>,
    /// `I#=` and `L#=`: the new column's value in one row.
    pub cells: Vec<ColumnCell>,
    /// `2DAMEMORY#=I#` / `L#`: the new column's value in that row, stored.
    pub stores: Vec<ColumnStore>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ColumnCell {
    pub row: RowRef,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ColumnStore {
    pub slot: u32,
    pub row: RowRef,
}

/// A row named the AddColumn way: `I#` by index, `L#` by label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "by", content = "value", rename_all = "snake_case")]
pub enum RowRef {
    Index(u32),
    Label(String),
}

/// One `File#=NAME` of `[GFFList]` with `[NAME]`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GffChanges {
    pub file: String,
    /// `!Destination`: `override` when absent.
    pub destination: Option<String>,
    /// `!Filename`: the name in the destination when it differs from `file`.
    pub filename: Option<String>,
    pub save_as: Option<String>,
    pub source_file: Option<String>,
    /// `!ReplaceFile=1`: the fresh copy from `tslpatchdata` overwrites before editing.
    pub replace_file: bool,
    /// `!OverrideType`: `ignore`, `warn` or `rename` for a copy also in `override`.
    pub override_type: Option<String>,
    pub edits: Vec<GffEdit>,
    pub adds: Vec<AddField>,
    pub line: u32,
    pub unknown: Vec<Entry>,
}

/// `PATH=VALUE`: set an existing field.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GffEdit {
    pub path: FieldPath,
    pub target: FieldTarget,
    pub value: Value,
    pub line: u32,
}

/// Where the field is: a backslash path, or the path a memory slot holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum FieldPath {
    Literal(String),
    Memory(u32),
}

/// What of the field: its value, a LocString's strref, or one substring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "lang", rename_all = "snake_case")]
pub enum FieldTarget {
    Value,
    StrRef,
    Substring(u32),
}

/// `AddField#=SECTION` with `[SECTION]`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AddField {
    pub section: String,
    pub field_type: FieldType,
    pub label: String,
    /// Where to add it; the file's root when absent, the parent struct for a nested field.
    pub path: Option<String>,
    pub value: Option<Value>,
    /// ExoLocString: the strref, `-1` for none.
    pub strref: Option<Value>,
    /// ExoLocString: `lang#=` substrings by language-and-gender id.
    pub substrings: Vec<(u32, String)>,
    /// Struct: the struct's type id.
    pub type_id: Option<u32>,
    pub children: Vec<AddField>,
    pub stores: Vec<AddStore>,
    /// The line of the `AddField#=` entry that adds it: its place in the order.
    pub line: u32,
    pub unknown: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddStore {
    pub slot: u32,
    pub what: AddStoreWhat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddStoreWhat {
    /// The index the struct was added to its list at.
    ListIndex,
    /// The added field's full path.
    FieldPath,
}

/// GFF field types, by the names `changes.ini` writes and the ids the format uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FieldType {
    Byte,
    Char,
    Word,
    Short,
    Dword,
    Int,
    Dword64,
    Int64,
    Float,
    Double,
    ExoString,
    ResRef,
    ExoLocString,
    Void,
    Struct,
    List,
    Orientation,
    Position,
}

impl FieldType {
    /// By name, case-insensitively, or by the GFF type id.
    pub fn parse(text: &str) -> Option<FieldType> {
        use FieldType::*;
        const NAMES: [(&str, FieldType); 18] = [
            ("byte", Byte),
            ("char", Char),
            ("word", Word),
            ("short", Short),
            ("dword", Dword),
            ("int", Int),
            ("dword64", Dword64),
            ("int64", Int64),
            ("float", Float),
            ("double", Double),
            ("exostring", ExoString),
            ("resref", ResRef),
            ("exolocstring", ExoLocString),
            ("void", Void),
            ("struct", Struct),
            ("list", List),
            ("orientation", Orientation),
            ("position", Position),
        ];
        let t = text.trim();
        if let Ok(n) = t.parse::<usize>() {
            return NAMES.get(n).map(|(_, ty)| *ty);
        }
        NAMES
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(t))
            .map(|(_, ty)| *ty)
    }
}

/// One `File#`/`Replace#` of `[HACKList]` with `[NAME]`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HackChanges {
    pub file: String,
    pub destination: Option<String>,
    pub replace_file: bool,
    pub writes: Vec<HackWrite>,
    pub line: u32,
    pub unknown: Vec<Entry>,
}

/// `OFFSET=VALUE`: a 16-bit value written at a byte offset.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HackWrite {
    pub offset: u32,
    pub value: Value,
    pub line: u32,
}

/// `[CompileList]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CompileList {
    /// `!DefaultDestination`: where every compiled script goes unless its own section says.
    pub default_destination: Option<String>,
    pub scripts: Vec<CompileScript>,
    pub unknown: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompileScript {
    pub file: String,
    /// `Replace#` overwrites an existing `.ncs`; `File#` skips it.
    pub replace: bool,
    pub destination: Option<String>,
    pub save_as: Option<String>,
    pub line: u32,
    pub unknown: Vec<Entry>,
}

/// One `File#`/`Replace#` of `[SSFList]` with `[NAME]`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SsfChanges {
    pub file: String,
    pub replace: bool,
    pub entries: Vec<SsfEntry>,
    pub line: u32,
}

/// `SOUND=VALUE`: the sound's strref, usually a `StrRef#` token.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SsfEntry {
    pub sound: String,
    pub value: Value,
    pub line: u32,
}

/// The parse of one file: the ini, its index for issues, and the findings.
struct Reader<'a> {
    ini: &'a Ini,
    file: usize,
    issues: &'a mut Vec<Issue>,
    /// Lines of repeated sections already reported, so a section two
    /// entries name is reported once.
    reported: HashSet<u32>,
}

impl Changes {
    /// Build the model from a parsed file; `file` is the index issues carry.
    pub fn parse(ini: &Ini, file: usize, issues: &mut Vec<Issue>) -> Changes {
        let mut r = Reader {
            ini,
            file,
            issues,
            reported: HashSet::new(),
        };
        Changes {
            settings: r.settings(),
            tlk: r.tlk_list(),
            install: r.install_list(),
            tables: r.table_list(),
            gff: r.gff_list(),
            hacks: r.hack_list(),
            compile: r.compile_list(),
            ssf: r.ssf_list(),
        }
    }
}

/// `PREFIX<digits>` as the prefix and the number.
pub fn split_numbered(key: &str) -> Option<(&str, u32)> {
    let head = key.trim_end_matches(|c: char| !c.is_ascii_digit());
    // The digits are ASCII, a byte each, so `start` is a char boundary
    // whatever precedes them.
    let start = head.len() - head.bytes().rev().take_while(u8::is_ascii_digit).count();
    if start == key.len() {
        return None;
    }
    let (prefix, n) = key.split_at(start);
    if n.is_empty() {
        return None;
    }
    Some((prefix, n.parse().ok()?))
}

/// `File#` or `Replace#` as the replace flag.
fn file_or_replace(key: &str) -> Option<bool> {
    let (prefix, _) = split_numbered(key)?;
    if prefix.eq_ignore_ascii_case("File") {
        Some(false)
    } else if prefix.eq_ignore_ascii_case("Replace") {
        Some(true)
    } else {
        None
    }
}

fn parse_bool(text: &str) -> Option<bool> {
    match text.trim() {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

impl<'a> Reader<'a> {
    fn warn(&mut self, code: &'static str, msg: String, line: u32) {
        self.issues.push(Issue::new(code, msg, self.file, line));
    }

    /// The section a list entry names, or the warning TSLPatcher's skip
    /// deserves. Repeats of the section are reported once, at the repeat.
    fn referenced(&mut self, name: &str, entry: &Entry, list: &str) -> Option<&'a Section> {
        let named = self.ini.sections_named(name);
        let first = named.first().copied();
        let repeats: Vec<u32> = named.iter().skip(1).map(|s| s.line).collect();
        for line in repeats {
            if self.reported.insert(line) {
                self.warn(
                    codes::DUPLICATE,
                    format!("section `[{name}]` is written again; the first is read"),
                    line,
                );
            }
        }
        if first.is_none() {
            self.warn(
                codes::MISSING_SECTION,
                format!(
                    "`{}={}` in [{list}] names a section `[{name}]` the file does not have; skipped",
                    entry.key, entry.value
                ),
                entry.line,
            );
        }
        first
    }

    /// The entries of a section as the Windows INI functions answer them:
    /// the first of each key, case-insensitively. A repeat is reported.
    fn entries<'s>(&mut self, section: &'s Section) -> Vec<&'s Entry> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut kept = Vec::new();
        for e in &section.entries {
            if seen.insert(e.key.to_ascii_lowercase()) {
                kept.push(e);
            } else {
                self.warn(
                    codes::DUPLICATE,
                    format!(
                        "`{}` is written again in [{}]; the first is read",
                        e.key, section.name
                    ),
                    e.line,
                );
            }
        }
        kept
    }

    fn unknown(&mut self, section: &str, e: &Entry) -> Entry {
        self.warn(
            codes::UNKNOWN_KEY,
            format!(
                "`{}` is not a key [{section}] takes; kept, not applied",
                e.key
            ),
            e.line,
        );
        e.clone()
    }

    fn bad(&mut self, e: &Entry, takes: &str) {
        self.warn(
            codes::BAD_VALUE,
            format!("`{}={}`: the key takes {takes}", e.key, e.value),
            e.line,
        );
    }

    fn settings(&mut self) -> Settings {
        let mut s = Settings::default();
        let Some(sec) = self.ini.section("Settings") else {
            return s;
        };
        let mut messages: Vec<String> = Vec::new();
        let mut files: Vec<String> = Vec::new();
        for e in self.entries(sec) {
            let k = e.key.to_ascii_lowercase();
            let bare = split_numbered(&k).map_or(k.as_str(), |(p, _)| p);
            match bare {
                "windowcaption" => s.window_caption = Some(e.value.clone()),
                "confirmmessage" => s.confirm_message = Some(e.value.clone()),
                "loglevel" => match e.value.parse() {
                    Ok(n) => s.log_level = Some(n),
                    Err(_) => self.bad(e, "a number from 0 to 4"),
                },
                "lookupgamenumber" => match e.value.parse() {
                    Ok(n) => s.lookup_game_number = Some(n),
                    Err(_) => self.bad(e, "1 or 2"),
                },
                "installermode"
                | "backupfiles"
                | "plaintextlog"
                | "lookupgamefolder"
                | "saveprocessedscripts"
                | "fileexists" => match parse_bool(&e.value) {
                    Some(b) => {
                        let slot = match bare {
                            "installermode" => &mut s.installer_mode,
                            "backupfiles" => &mut s.backup_files,
                            "plaintextlog" => &mut s.plaintext_log,
                            "lookupgamefolder" => &mut s.lookup_game_folder,
                            "saveprocessedscripts" => &mut s.save_processed_scripts,
                            _ => &mut s.file_exists,
                        };
                        *slot = Some(b);
                    }
                    None => self.bad(e, "0 or 1"),
                },
                "required" => files.push(e.value.clone()),
                "requiredmsg" => messages.push(e.value.clone()),
                _ => {
                    let u = self.unknown("Settings", e);
                    s.unknown.push(u);
                }
            }
        }
        // The shorter list is padded: a file without a message gets the
        // patcher's own text, a message without a file requires nothing.
        let mut messages = messages.into_iter();
        s.required = files
            .into_iter()
            .map(|file| Requirement {
                file,
                message: messages.next(),
            })
            .filter(|r| !r.file.is_empty())
            .collect();
        s
    }

    fn tlk_list(&mut self) -> TlkList {
        let mut t = TlkList::default();
        let Some(sec) = self.ini.section("TLKList") else {
            return t;
        };
        for e in self.entries(sec) {
            let k = e.key.to_ascii_lowercase();
            if k == "!sourcefile" {
                t.source_file = Some(e.value.clone());
                continue;
            }
            if k == "!sourcefilef" {
                t.source_file_f = Some(e.value.clone());
                continue;
            }
            if let Some(token) = token_number(&e.key, "StrRef") {
                match e.value.parse() {
                    Ok(index) => t.appends.push(TlkAppend {
                        token,
                        index,
                        line: e.line,
                    }),
                    Err(_) => self.bad(e, "the index of an entry in append.tlk"),
                }
                continue;
            }
            match split_numbered(&e.key) {
                Some((p, _))
                    if p.eq_ignore_ascii_case("Replace")
                        || p.eq_ignore_ascii_case("ReplaceFile") =>
                {
                    let entries = match self.referenced(&e.value, e, "TLKList") {
                        Some(fs) => self.tlk_replace_entries(fs),
                        None => Vec::new(),
                    };
                    t.replacements.push(TlkReplaceFile {
                        key: e.key.clone(),
                        file: e.value.clone(),
                        entries,
                        line: e.line,
                    });
                }
                _ => {
                    let u = self.unknown("TLKList", e);
                    t.unknown.push(u);
                }
            }
        }
        t
    }

    fn tlk_replace_entries(&mut self, sec: &Section) -> Vec<TlkReplaceEntry> {
        let mut out = Vec::new();
        for e in self.entries(sec) {
            // Written as `25859=27` or as `StrRef223=0`; both name the
            // dialog.tlk entry on the left.
            let strref = e
                .key
                .parse()
                .ok()
                .or_else(|| token_number(&e.key, "StrRef"));
            match (strref, e.value.parse()) {
                (Some(strref), Ok(index)) => out.push(TlkReplaceEntry {
                    strref,
                    index,
                    line: e.line,
                }),
                _ => self.bad(
                    e,
                    "a dialog.tlk index on the left and the file's entry index on the right",
                ),
            }
        }
        out
    }

    fn install_list(&mut self) -> Vec<InstallFolder> {
        let mut out = Vec::new();
        let Some(sec) = self.ini.section("InstallList") else {
            return out;
        };
        for e in self.entries(sec) {
            let mut folder = InstallFolder {
                key: e.key.clone(),
                destination: e.value.clone(),
                files: Vec::new(),
                line: e.line,
                unknown: Vec::new(),
            };
            // The folder's files are under a section named by the key, not the value.
            let fs = {
                let named = self.ini.sections_named(&e.key);
                let first = named.first().copied();
                let repeats: Vec<u32> = named.iter().skip(1).map(|s| s.line).collect();
                for line in repeats {
                    if self.reported.insert(line) {
                        self.warn(
                            codes::DUPLICATE,
                            format!("section `[{}]` is written again; the first is read", e.key),
                            line,
                        );
                    }
                }
                first
            };
            let Some(fs) = fs else {
                self.warn(
                    codes::MISSING_SECTION,
                    format!(
                        "`{}={}` in [InstallList] has no `[{}]` section listing its files; skipped",
                        e.key, e.value, e.key
                    ),
                    e.line,
                );
                out.push(folder);
                continue;
            };
            for fe in self.entries(fs) {
                match file_or_replace(&fe.key) {
                    Some(replace) => {
                        let mut f = InstallFile {
                            name: fe.value.clone(),
                            replace,
                            save_as: None,
                            source_file: None,
                            line: fe.line,
                        };
                        // A file may have a section of its own, for `!SaveAs`
                        // and `!SourceFile`; it is optional, and a section of
                        // that name may belong to another list (a 2DA or GFF
                        // the mod both installs and edits), so other keys are
                        // that list's business.
                        if let Some(own) = self.ini.section(&fe.value) {
                            f.save_as = own.value("!SaveAs").map(str::to_string);
                            f.source_file = own.value("!SourceFile").map(str::to_string);
                        }
                        folder.files.push(f);
                    }
                    None => {
                        let u = self.unknown(&fs.name, fe);
                        folder.unknown.push(u);
                    }
                }
            }
            out.push(folder);
        }
        out
    }

    fn table_list(&mut self) -> Vec<TableChanges> {
        let mut out = Vec::new();
        let Some(sec) = self.ini.section("2DAList") else {
            return out;
        };
        for e in self.entries(sec) {
            if !split_numbered(&e.key).is_some_and(|(p, _)| p.eq_ignore_ascii_case("Table")) {
                self.unknown("2DAList", e);
                continue;
            }
            let mut table = TableChanges {
                file: e.value.clone(),
                ops: Vec::new(),
                line: e.line,
                unknown: Vec::new(),
            };
            let Some(ts) = self.referenced(&e.value, e, "2DAList") else {
                out.push(table);
                continue;
            };
            for te in self.entries(ts) {
                let Some((prefix, _)) = split_numbered(&te.key) else {
                    let u = self.unknown(&ts.name, te);
                    table.unknown.push(u);
                    continue;
                };
                let p = prefix.to_ascii_lowercase();
                if !matches!(p.as_str(), "changerow" | "addrow" | "copyrow" | "addcolumn") {
                    let u = self.unknown(&ts.name, te);
                    table.unknown.push(u);
                    continue;
                }
                let Some(rs) = self.referenced(&te.value, te, &ts.name) else {
                    continue;
                };
                let kind = match p.as_str() {
                    "changerow" => TableOpKind::ChangeRow(self.row_change(rs)),
                    "addrow" => TableOpKind::AddRow(self.row_add(rs)),
                    "copyrow" => TableOpKind::CopyRow(self.row_copy(rs)),
                    _ => TableOpKind::AddColumn(self.column_add(rs, &mut table.unknown)),
                };
                table.ops.push(TableOp {
                    key: te.key.clone(),
                    section: rs.name.clone(),
                    kind,
                    line: te.line,
                });
            }
            out.push(table);
        }
        out
    }

    /// The keys every row operation shares: a store, or a cell. Returns
    /// false when the key was neither (the caller handles its own specials).
    fn row_common(&mut self, e: &Entry, cells: &mut Vec<Cell>, stores: &mut Vec<Store>) {
        if let Some(slot) = memory_key(&e.key) {
            let source = if e.value.eq_ignore_ascii_case("RowIndex") {
                StoreSource::RowIndex
            } else if e.value.eq_ignore_ascii_case("RowLabel") {
                StoreSource::RowLabel
            } else {
                StoreSource::Column(e.value.clone())
            };
            stores.push(Store { slot, source });
        } else {
            cells.push(Cell {
                column: e.key.clone(),
                value: Value::parse(&e.value),
            });
        }
    }

    fn row_target(&mut self, e: &Entry) -> Option<RowTarget> {
        match e.key.to_ascii_lowercase().as_str() {
            "rowindex" => match e.value.parse() {
                Ok(n) => Some(RowTarget::Index(n)),
                Err(_) => {
                    self.bad(e, "a row number");
                    None
                }
            },
            "rowlabel" => Some(RowTarget::Label(e.value.clone())),
            "labelindex" => Some(RowTarget::LabelIndex(e.value.clone())),
            _ => None,
        }
    }

    fn row_change(&mut self, sec: &Section) -> RowChange {
        let mut op = RowChange {
            target: None,
            cells: Vec::new(),
            stores: Vec::new(),
        };
        for e in self.entries(sec) {
            if matches!(
                e.key.to_ascii_lowercase().as_str(),
                "rowindex" | "rowlabel" | "labelindex"
            ) {
                if let Some(t) = self.row_target(e) {
                    op.target = Some(t);
                }
            } else {
                self.row_common(e, &mut op.cells, &mut op.stores);
            }
        }
        if op.target.is_none() {
            self.warn(
                codes::BAD_VALUE,
                format!(
                    "[{}] changes a row without RowIndex, RowLabel or LabelIndex to find it",
                    sec.name
                ),
                sec.line,
            );
        }
        op
    }

    fn row_add(&mut self, sec: &Section) -> RowAdd {
        let mut op = RowAdd {
            exclusive_column: None,
            row_label: None,
            cells: Vec::new(),
            stores: Vec::new(),
        };
        for e in self.entries(sec) {
            match e.key.to_ascii_lowercase().as_str() {
                "exclusivecolumn" => op.exclusive_column = Some(e.value.clone()),
                "rowlabel" => op.row_label = Some(Value::parse(&e.value)),
                _ => self.row_common(e, &mut op.cells, &mut op.stores),
            }
        }
        op
    }

    fn row_copy(&mut self, sec: &Section) -> RowCopy {
        let mut op = RowCopy {
            target: None,
            exclusive_column: None,
            new_row_label: None,
            cells: Vec::new(),
            stores: Vec::new(),
        };
        for e in self.entries(sec) {
            match e.key.to_ascii_lowercase().as_str() {
                "exclusivecolumn" => op.exclusive_column = Some(e.value.clone()),
                "newrowlabel" => op.new_row_label = Some(Value::parse(&e.value)),
                "rowindex" | "rowlabel" | "labelindex" => {
                    if let Some(t) = self.row_target(e) {
                        op.target = Some(t);
                    }
                }
                _ => self.row_common(e, &mut op.cells, &mut op.stores),
            }
        }
        if op.target.is_none() {
            self.warn(
                codes::BAD_VALUE,
                format!(
                    "[{}] copies a row without RowIndex, RowLabel or LabelIndex to find it",
                    sec.name
                ),
                sec.line,
            );
        }
        op
    }

    fn column_add(&mut self, sec: &Section, unknown: &mut Vec<Entry>) -> ColumnAdd {
        let mut op = ColumnAdd {
            label: String::new(),
            default: None,
            cells: Vec::new(),
            stores: Vec::new(),
        };
        for e in self.entries(sec) {
            match e.key.to_ascii_lowercase().as_str() {
                "columnlabel" => op.label = e.value.clone(),
                "defaultvalue" => op.default = Some(Value::parse(&e.value)),
                _ => {
                    if let Some(slot) = memory_key(&e.key) {
                        match row_ref(&e.value) {
                            Some(row) => op.stores.push(ColumnStore { slot, row }),
                            None => self.bad(e, "I<row index> or L<row label>"),
                        }
                    } else if let Some(row) = row_ref(&e.key) {
                        op.cells.push(ColumnCell {
                            row,
                            value: Value::parse(&e.value),
                        });
                    } else {
                        let u = self.unknown(&sec.name, e);
                        unknown.push(u);
                    }
                }
            }
        }
        if op.label.is_empty() {
            self.warn(
                codes::BAD_VALUE,
                format!("[{}] adds a column without a ColumnLabel", sec.name),
                sec.line,
            );
        }
        op
    }

    fn gff_list(&mut self) -> Vec<GffChanges> {
        let mut out = Vec::new();
        let Some(sec) = self.ini.section("GFFList") else {
            return out;
        };
        for e in self.entries(sec) {
            if file_or_replace(&e.key).is_none() {
                self.unknown("GFFList", e);
                continue;
            }
            let mut g = GffChanges {
                file: e.value.clone(),
                destination: None,
                filename: None,
                save_as: None,
                source_file: None,
                replace_file: false,
                override_type: None,
                edits: Vec::new(),
                adds: Vec::new(),
                line: e.line,
                unknown: Vec::new(),
            };
            let Some(gs) = self.referenced(&e.value, e, "GFFList") else {
                out.push(g);
                continue;
            };
            let mut visiting: Vec<String> = vec![gs.name.to_ascii_lowercase()];
            for ge in self.entries(gs) {
                let k = ge.key.to_ascii_lowercase();
                match k.as_str() {
                    "!destination" => g.destination = Some(ge.value.clone()),
                    "!filename" => g.filename = Some(ge.value.clone()),
                    "!saveas" => g.save_as = Some(ge.value.clone()),
                    "!sourcefile" => g.source_file = Some(ge.value.clone()),
                    "!overridetype" => g.override_type = Some(ge.value.clone()),
                    "!replacefile" => match parse_bool(&ge.value) {
                        Some(b) => g.replace_file = b,
                        None => self.bad(ge, "0 or 1"),
                    },
                    _ if k.starts_with('!') => {
                        let u = self.unknown(&gs.name, ge);
                        g.unknown.push(u);
                    }
                    _ if split_numbered(&ge.key)
                        .is_some_and(|(p, _)| p.eq_ignore_ascii_case("AddField")) =>
                    {
                        if let Some(add) = self.add_field(ge, &gs.name, &mut visiting) {
                            g.adds.push(add);
                        }
                    }
                    _ => g.edits.push(gff_edit(ge)),
                }
            }
            out.push(g);
        }
        out
    }

    fn add_field(
        &mut self,
        e: &Entry,
        parent: &str,
        visiting: &mut Vec<String>,
    ) -> Option<AddField> {
        let sec = self.referenced(&e.value, e, parent)?;
        let lower = sec.name.to_ascii_lowercase();
        if visiting.contains(&lower) {
            self.warn(
                codes::BAD_VALUE,
                format!("`{}={}` adds a field inside itself", e.key, e.value),
                e.line,
            );
            return None;
        }
        visiting.push(lower);
        let mut add = AddField {
            section: sec.name.clone(),
            field_type: FieldType::Void,
            label: String::new(),
            path: None,
            value: None,
            strref: None,
            substrings: Vec::new(),
            type_id: None,
            children: Vec::new(),
            stores: Vec::new(),
            line: e.line,
            unknown: Vec::new(),
        };
        let mut typed = false;
        for fe in self.entries(sec) {
            let k = fe.key.to_ascii_lowercase();
            match k.as_str() {
                "fieldtype" => match FieldType::parse(&fe.value) {
                    Some(t) => {
                        add.field_type = t;
                        typed = true;
                    }
                    None => self.bad(fe, "a GFF field type name or id"),
                },
                "label" => add.label = fe.value.clone(),
                "path" => add.path = Some(fe.value.clone()).filter(|p| !p.is_empty()),
                "value" => add.value = Some(Value::parse(&fe.value)),
                "strref" => add.strref = Some(Value::parse(&fe.value)),
                "typeid" => match fe.value.parse() {
                    Ok(n) => add.type_id = Some(n),
                    Err(_) => self.bad(fe, "a struct type id, 0 or larger"),
                },
                _ => {
                    if let Some(lang) = token_number(&fe.key, "lang") {
                        add.substrings.push((lang, fe.value.clone()));
                    } else if let Some(slot) = memory_key(&fe.key) {
                        let what = if fe.value.eq_ignore_ascii_case("ListIndex") {
                            Some(AddStoreWhat::ListIndex)
                        } else if fe.value.eq_ignore_ascii_case("!FieldPath") {
                            Some(AddStoreWhat::FieldPath)
                        } else {
                            None
                        };
                        match what {
                            Some(what) => add.stores.push(AddStore { slot, what }),
                            None => self.bad(fe, "ListIndex or !FieldPath"),
                        }
                    } else if split_numbered(&fe.key)
                        .is_some_and(|(p, _)| p.eq_ignore_ascii_case("AddField"))
                    {
                        if let Some(child) = self.add_field(fe, &sec.name, visiting) {
                            add.children.push(child);
                        }
                    } else {
                        let u = self.unknown(&sec.name, fe);
                        add.unknown.push(u);
                    }
                }
            }
        }
        visiting.pop();
        if !typed {
            self.warn(
                codes::BAD_VALUE,
                format!("[{}] adds a field without a FieldType", sec.name),
                sec.line,
            );
        }
        Some(add)
    }

    fn hack_list(&mut self) -> Vec<HackChanges> {
        let mut out = Vec::new();
        let Some(sec) = self.ini.section("HACKList") else {
            return out;
        };
        for e in self.entries(sec) {
            let Some(replace) = file_or_replace(&e.key) else {
                self.unknown("HACKList", e);
                continue;
            };
            let mut h = HackChanges {
                file: e.value.clone(),
                destination: None,
                replace_file: replace,
                writes: Vec::new(),
                line: e.line,
                unknown: Vec::new(),
            };
            let Some(hs) = self.referenced(&e.value, e, "HACKList") else {
                out.push(h);
                continue;
            };
            for he in self.entries(hs) {
                let k = he.key.to_ascii_lowercase();
                match k.as_str() {
                    "!destination" => h.destination = Some(he.value.clone()),
                    "replacefile" | "!replacefile" => match parse_bool(&he.value) {
                        Some(b) => h.replace_file = b,
                        None => self.bad(he, "0 or 1"),
                    },
                    _ => match parse_offset(&he.key) {
                        Some(offset) => h.writes.push(HackWrite {
                            offset,
                            value: Value::parse(&he.value),
                            line: he.line,
                        }),
                        None => {
                            let u = self.unknown(&hs.name, he);
                            h.unknown.push(u);
                        }
                    },
                }
            }
            out.push(h);
        }
        out
    }

    fn compile_list(&mut self) -> CompileList {
        let mut c = CompileList::default();
        let Some(sec) = self.ini.section("CompileList") else {
            return c;
        };
        for e in self.entries(sec) {
            if e.key.eq_ignore_ascii_case("!DefaultDestination") {
                c.default_destination = Some(e.value.clone());
                continue;
            }
            let Some(replace) = file_or_replace(&e.key) else {
                let u = self.unknown("CompileList", e);
                c.unknown.push(u);
                continue;
            };
            let mut s = CompileScript {
                file: e.value.clone(),
                replace,
                destination: None,
                save_as: None,
                line: e.line,
                unknown: Vec::new(),
            };
            // The script's own section is optional.
            if let Some(own) = self.ini.section(&e.value) {
                for oe in self.entries(own) {
                    match oe.key.to_ascii_lowercase().as_str() {
                        "!destination" => s.destination = Some(oe.value.clone()),
                        "!saveas" => s.save_as = Some(oe.value.clone()),
                        _ => {
                            let u = self.unknown(&own.name, oe);
                            s.unknown.push(u);
                        }
                    }
                }
            }
            c.scripts.push(s);
        }
        c
    }

    fn ssf_list(&mut self) -> Vec<SsfChanges> {
        let mut out = Vec::new();
        let Some(sec) = self.ini.section("SSFList") else {
            return out;
        };
        for e in self.entries(sec) {
            let Some(replace) = file_or_replace(&e.key) else {
                self.unknown("SSFList", e);
                continue;
            };
            let mut s = SsfChanges {
                file: e.value.clone(),
                replace,
                entries: Vec::new(),
                line: e.line,
            };
            let Some(ss) = self.referenced(&e.value, e, "SSFList") else {
                out.push(s);
                continue;
            };
            for se in self.entries(ss) {
                s.entries.push(SsfEntry {
                    sound: se.key.clone(),
                    value: Value::parse(&se.value),
                    line: se.line,
                });
            }
            out.push(s);
        }
        out
    }
}

/// `PATH=VALUE`, `PATH(strref)=VALUE`, `PATH(lang#)=VALUE`, or `2DAMEMORY#=VALUE`.
fn gff_edit(e: &Entry) -> GffEdit {
    let (path, target) = match e.key.rsplit_once('(') {
        Some((head, tail)) if tail.ends_with(')') => {
            let inner = &tail[..tail.len() - 1];
            if inner.eq_ignore_ascii_case("strref") {
                (head, FieldTarget::StrRef)
            } else if let Some(lang) = token_number(inner, "lang") {
                (head, FieldTarget::Substring(lang))
            } else {
                (e.key.as_str(), FieldTarget::Value)
            }
        }
        _ => (e.key.as_str(), FieldTarget::Value),
    };
    let path = match memory_key(path) {
        Some(slot) => FieldPath::Memory(slot),
        None => FieldPath::Literal(path.to_string()),
    };
    GffEdit {
        path,
        target,
        value: Value::parse(&e.value),
        line: e.line,
    }
}

/// `I#` or `L#`, the AddColumn way of naming a row.
fn row_ref(text: &str) -> Option<RowRef> {
    let t = text.trim();
    let mut chars = t.chars();
    let first = chars.next()?;
    let rest = chars.as_str();
    if rest.is_empty() {
        return None;
    }
    match first {
        'I' | 'i' => rest.parse().ok().map(RowRef::Index),
        'L' | 'l' => Some(RowRef::Label(rest.to_string())),
        _ => None,
    }
}

/// A byte offset, decimal or `0x` hex.
fn parse_offset(text: &str) -> Option<u32> {
    let t = text.trim();
    match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => t.parse().ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbered_keys_split_into_prefix_and_number() {
        assert_eq!(split_numbered("File12"), Some(("File", 12)));
        assert_eq!(
            split_numbered("install_folder0"),
            Some(("install_folder", 0))
        );
        assert_eq!(split_numbered("2DAMEMORY3"), Some(("2DAMEMORY", 3)));
        assert_eq!(split_numbered("Label"), None);
        assert_eq!(split_numbered("123"), Some(("", 123)));
    }

    #[test]
    fn field_types_by_name_or_id() {
        assert_eq!(
            FieldType::parse("exolocstring"),
            Some(FieldType::ExoLocString)
        );
        assert_eq!(FieldType::parse("12"), Some(FieldType::ExoLocString));
        assert_eq!(FieldType::parse("17"), Some(FieldType::Position));
        assert_eq!(FieldType::parse("Blob"), None);
    }

    #[test]
    fn gff_edit_keys_split_path_and_target() {
        let e = |k: &str| Entry {
            key: k.into(),
            value: "1".into(),
            line: 1,
        };
        let g = gff_edit(&e("EntryList\\3\\Text(strref)"));
        assert_eq!(g.path, FieldPath::Literal("EntryList\\3\\Text".into()));
        assert_eq!(g.target, FieldTarget::StrRef);
        let g = gff_edit(&e("FirstName(lang0)"));
        assert_eq!(g.target, FieldTarget::Substring(0));
        let g = gff_edit(&e("2DAMEMORY5"));
        assert_eq!(g.path, FieldPath::Memory(5));
        let g = gff_edit(&e("Name(odd)"));
        assert_eq!(g.path, FieldPath::Literal("Name(odd)".into()));
    }

    #[test]
    fn row_refs_and_offsets() {
        assert_eq!(row_ref("I12"), Some(RowRef::Index(12)));
        assert_eq!(row_ref("Lfoo"), Some(RowRef::Label("foo".into())));
        assert_eq!(row_ref("X1"), None);
        assert_eq!(parse_offset("0x1A"), Some(26));
        assert_eq!(parse_offset("26"), Some(26));
    }
}
