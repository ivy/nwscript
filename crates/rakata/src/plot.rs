//! Plot state read from the index: who writes and reads an object's local
//! flags or a global ([`flags`]), and a quest's journal entries with what
//! writes each ([`quest`]). What `rakata flags` and `rakata quest` print.
//!
//! Calls come from the index's `calls` and `resolved_calls`
//! (`docs/index.md`): a call the index resolves through the script's own
//! functions (`UT_SetPlotBooleanFlag` reaching `SetLocalBoolean`) is read
//! from `resolved_calls`, and stands for the plain row of its `ACTION`.
//! Every call has its site, the script and offset, and what runs the
//! script: a dialogue node's `Script`, a link's `Active` condition, an
//! object's event field (`OnUsed`, `ScriptUserDefine`), or an
//! `ExecuteScript` call naming it. `OBJECT_SELF` is the object that runs
//! the script: the owner of the dialogue (the creature, placeable or door
//! whose `Conversation` names it), the object whose event field names the
//! script, or the object `ExecuteScript` runs it on; where nothing in the
//! index runs the script, it is unresolved.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::rc::Rc;

use nwscript::diag::Diagnostic;
use rakata_index::Connection;
use serde::Serialize;
use serde_json::Value;

use crate::codes;

/// The codes this module reports.
pub mod ids {
    /// An index holding several trees, none of them named.
    pub const TREES: &str = "RE2101";
    /// A quest `global.jrl` has no category for.
    pub const NO_QUEST: &str = "RE2102";
    /// A journal entry nothing in the index writes.
    pub const UNWRITTEN: &str = "RW2101";
    /// A write naming an entry the journal category lacks.
    pub const NO_ENTRY: &str = "RW2102";
    /// A player reply that does what a sibling reply writing the quest
    /// does, and writes no entry.
    pub const SIBLING: &str = "RW2103";
    /// A `flags` symbol no object in scope bears and no global declares.
    pub const NOTHING: &str = "RW2104";
}
use ids as c;

fn diagnostic(id: &str, message: impl Into<String>) -> Diagnostic {
    let code =
        codes::lookup(id).unwrap_or_else(|| panic!("diagnostic code {id} is not registered"));
    Diagnostic::new(code, message)
}

/// The routines that set and read an object's local variables; each
/// takes the object, the index and, to set, the value.
const LOCAL_SETS: &[&str] = &["SetLocalBoolean", "SetLocalNumber"];
const LOCAL_GETS: &[&str] = &["GetLocalBoolean", "GetLocalNumber"];

/// The prefixes of the include constants that name a local's index:
/// `SW_PLOT_BOOLEAN_01`, `SW_FLAG_AI_OFF` for booleans, `SW_NUMBER_*` for
/// numbers, as `k_inc_utility`, `k_inc_generic` and `k_inc_gensupport`
/// declare them.
const BOOLEAN_NAMES: &[&str] = &["SW_PLOT_", "SW_FLAG_"];
const NUMBER_NAMES: &[&str] = &["SW_NUMBER_"];

/// SQL text for a string literal.
fn lit(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn list<'a>(items: impl IntoIterator<Item = &'a str>) -> String {
    let v: Vec<String> = items.into_iter().map(lit).collect();
    if v.is_empty() {
        "NULL".to_string()
    } else {
        v.join(", ")
    }
}

fn s(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        v => v.to_string(),
    }
}

/// The tree and module a query reads.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Scope {
    #[serde(skip)]
    pub tree: i64,
    /// The tree's label.
    pub label: String,
    pub module: Option<String>,
    /// The module's capsules as the index spells their paths.
    pub capsules: Vec<String>,
}

/// The index read through one tree.
struct Reader<'c> {
    conn: &'c Connection,
    pub scope: Scope,
    owners_of: BTreeMap<String, Vec<Object>>,
    runners_of: BTreeMap<String, Vec<Runner>>,
    selves_of: BTreeMap<(String, u8), Rc<Vec<RunBy>>>,
    loaded: bool,
}

impl<'c> Reader<'c> {
    /// The tree `tree` names, or the index's one tree; the capsules of
    /// `module` in it when one is named.
    fn open(
        conn: &'c Connection,
        tree: Option<&str>,
        module: Option<&str>,
    ) -> Result<Self, Diagnostic> {
        let q = |sql: &str| rakata_index::query(conn, sql).map_err(|e| crate::index::error(&e));
        let trees = q("SELECT id, label FROM trees ORDER BY id")?;
        let labels = || {
            trees
                .rows
                .iter()
                .map(|r| format!("`{}`", s(&r[1])))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let (id, label) = match (tree, trees.rows.as_slice()) {
            (Some(t), rows) => rows
                .iter()
                .find(|r| r[1].as_str() == Some(t))
                .map(|r| (r[0].as_i64().unwrap_or(0), t.to_string()))
                .ok_or_else(|| {
                    crate::index::diagnostic(
                        rakata_index::codes::NO_SUCH_TREE,
                        format!("the index holds no tree `{t}`; it holds {}", labels()),
                    )
                })?,
            (None, [r]) => (r[0].as_i64().unwrap_or(0), s(&r[1])),
            (None, []) => {
                return Err(diagnostic(c::TREES, "the index holds no tree").help(
                    "build one with `rakata index build --game DIR --tree LABEL --db FILE`",
                ));
            }
            (None, _) => {
                return Err(diagnostic(
                    c::TREES,
                    format!(
                        "the index holds {} trees ({}); name one with --tree",
                        trees.rows.len(),
                        labels()
                    ),
                ));
            }
        };
        let mut capsules = Vec::new();
        if let Some(m) = module {
            let m = m.to_ascii_lowercase();
            let want = [
                format!("modules/{m}.mod"),
                format!("modules/{m}.rim"),
                format!("modules/{m}_s.rim"),
            ];
            let rows = q(&format!(
                "SELECT DISTINCT path FROM locations WHERE tree = {id} AND member = ''
                   AND lower(path) IN ({}) ORDER BY path",
                list(want.iter().map(String::as_str))
            ))?;
            capsules = rows.rows.iter().map(|r| s(&r[0])).collect();
            if capsules.is_empty() {
                return Err(diagnostic(
                    "RE0403",
                    format!("the tree `{label}` holds no capsule of the module `{m}`"),
                )
                .help("name the module by the stem of modules/STEM.mod, STEM.rim or STEM_s.rim"));
            }
        }
        Ok(Reader {
            conn,
            scope: Scope {
                tree: id,
                label,
                module: module.map(str::to_ascii_lowercase),
                capsules,
            },
            owners_of: BTreeMap::new(),
            runners_of: BTreeMap::new(),
            selves_of: BTreeMap::new(),
            loaded: false,
        })
    }

    fn q(&self, sql: &str) -> Result<rakata_index::Rows, Diagnostic> {
        rakata_index::query(self.conn, sql).map_err(|e| crate::index::error(&e))
    }

    /// Where a resource lies: the archive's file name for a member
    /// (`korr_m37aa_s.rim`, `scripts.bif`), the path for a loose file.
    fn place(path: &str, member: &str) -> String {
        if member.is_empty() {
            path.to_string()
        } else {
            path.rsplit('/').next().unwrap_or(path).to_string()
        }
    }

    /// The calls to `routines` in the tree's scripts, each `ACTION` once
    /// per path: the rows of `resolved_calls` in place of the plain row of
    /// the same `ACTION`. `filter` is SQL over `c.args`.
    fn calls(&self, routines: &[&str], filter: &str) -> Result<Vec<Call>, Diagnostic> {
        let tree = self.scope.tree;
        let names = list(routines.iter().copied());
        let resolved = self.q(&format!(
            "SELECT l.name, l.path, l.member, c.hash, c.site, c.offset, c.routine, c.args, c.path
             FROM resolved_calls c JOIN locations l ON l.hash = c.hash
             WHERE l.tree = {tree} AND l.type = 'ncs' AND c.routine IN ({names}) AND ({filter})
             ORDER BY l.name, c.site, c.offset"
        ))?;
        let plain = self.q(&format!(
            "SELECT l.name, l.path, l.member, c.hash, c.offset, c.offset, c.routine, c.args, NULL
             FROM calls c JOIN locations l ON l.hash = c.hash
             WHERE l.tree = {tree} AND l.type = 'ncs' AND c.routine IN ({names})
               AND ({filter})
               AND NOT EXISTS (SELECT 1 FROM resolved_calls r
                               WHERE r.hash = c.hash AND r.offset = c.offset)
             ORDER BY l.name, c.offset"
        ))?;
        let mut by_key: BTreeMap<(String, String, i64, String), Call> = BTreeMap::new();
        for r in resolved.rows.iter().chain(&plain.rows) {
            let script = s(&r[0]);
            let hash = s(&r[3]);
            let site = r[4].as_i64().unwrap_or(0);
            let args: Vec<Value> = serde_json::from_str(&s(&r[7])).unwrap_or_default();
            let path: Vec<i64> = match &r[8] {
                Value::String(p) => serde_json::from_str(p).unwrap_or_default(),
                _ => vec![site],
            };
            let key = (script.clone(), hash.clone(), site, s(&r[7]));
            let place = Self::place(&s(&r[1]), &s(&r[2]));
            by_key
                .entry(key)
                .and_modify(|c| {
                    if !c.locations.contains(&place) {
                        c.locations.push(place.clone());
                    }
                })
                .or_insert(Call {
                    script,
                    hash,
                    site,
                    path,
                    routine: s(&r[6]),
                    args,
                    locations: vec![place],
                    run_by: Vec::new(),
                });
        }
        let mut out: Vec<Call> = by_key.into_values().collect();
        out.sort_by(|a, b| (&a.script, a.site).cmp(&(&b.script, b.site)));
        Ok(out)
    }

    /// What runs the script `name`: dialogue nodes and conditions, event
    /// fields, `ExecuteScript` calls.
    fn runners(&mut self, name: &str) -> Result<Vec<Runner>, Diagnostic> {
        if !self.loaded {
            self.load()?;
        }
        Ok(self
            .runners_of
            .get(&name.to_ascii_lowercase())
            .cloned()
            .unwrap_or_default())
    }

    /// The objects whose `Conversation` names `dialog`, in scope.
    fn dialog_owners(&mut self, dialog: &str) -> Result<Vec<Object>, Diagnostic> {
        if !self.loaded {
            self.load()?;
        }
        Ok(self
            .owners_of
            .get(&dialog.to_ascii_lowercase())
            .cloned()
            .unwrap_or_default())
    }

    /// Every runner of every script in the tree, and every owner of every
    /// dialogue in scope, read once.
    fn load(&mut self) -> Result<(), Diagnostic> {
        let tree = self.scope.tree;
        let mut runners: BTreeMap<String, Vec<Runner>> = BTreeMap::new();
        for r in self
            .q(&format!(
                "SELECT DISTINCT n.script, l.name, l.path, l.member, n.node FROM dialog_nodes n
                 JOIN locations l ON l.hash = n.hash
                 WHERE l.tree = {tree} AND n.script IS NOT NULL AND n.script <> ''
                 ORDER BY 2, 5"
            ))?
            .rows
        {
            runners.entry(s(&r[0])).or_default().push(Runner::Node {
                dialog: s(&r[1]),
                node: s(&r[4]),
                location: Self::place(&s(&r[2]), &s(&r[3])),
            });
        }
        for r in self
            .q(&format!(
                "SELECT DISTINCT k.condition, l.name, l.path, l.member, k.source, k.target
                 FROM dialog_links k JOIN locations l ON l.hash = k.hash
                 WHERE l.tree = {tree} AND k.condition IS NOT NULL AND k.condition <> ''
                 ORDER BY 2, 5, 6"
            ))?
            .rows
        {
            runners
                .entry(s(&r[0]))
                .or_default()
                .push(Runner::Condition {
                    dialog: s(&r[1]),
                    source: s(&r[4]),
                    target: s(&r[5]),
                    location: Self::place(&s(&r[2]), &s(&r[3])),
                });
        }
        for r in self
            .q(&format!(
                "SELECT DISTINCT lower(f.value), l.name, l.type, l.path, l.member, f.path,
                        (SELECT t.value FROM gff_fields t WHERE t.hash = f.hash AND t.path = 'Tag')
                 FROM locations l JOIN gff_fields f ON f.hash = l.hash
                 WHERE l.tree = {tree} AND l.type NOT IN ('dlg', 'uti', 'jrl', 'gui', 'fac', 'pth')
                   AND f.type = 'CResRef' AND f.value <> ''
                   AND (f.label GLOB 'On*' OR f.label GLOB 'Script*' OR f.label GLOB 'Mod_On*')
                 ORDER BY 2, 3, 6"
            ))?
            .rows
        {
            let ty = s(&r[2]);
            let tag = match &r[6] {
                Value::String(t) if !t.is_empty() && ty != "ifo" => Some(t.clone()),
                _ => None,
            };
            runners.entry(s(&r[0])).or_default().push(Runner::Event {
                resource: format!("{}.{ty}", s(&r[1])),
                field: s(&r[5]),
                tag,
                location: Self::place(&s(&r[3]), &s(&r[4])),
            });
        }
        for e in self.calls(&["ExecuteScript"], "json_type(c.args, '$[0]') = 'text'")? {
            let Some(target) = e.args.first().and_then(Value::as_str) else {
                continue;
            };
            runners
                .entry(target.to_ascii_lowercase())
                .or_default()
                .push(Runner::Execute {
                    script: e.script.clone(),
                    site: e.site,
                    target: e
                        .args
                        .get(1)
                        .map(object_arg)
                        .unwrap_or(ObjectArg::Unresolved("computed".to_string())),
                });
        }
        let mut owners: BTreeMap<String, Vec<Object>> = BTreeMap::new();
        for r in self
            .q(&format!(
                "SELECT DISTINCT lower(f.value), l.name, l.type, l.path, l.member,
                        (SELECT t.value FROM gff_fields t WHERE t.hash = f.hash AND t.path = 'Tag')
                 FROM locations l JOIN gff_fields f ON f.hash = l.hash
                 WHERE l.tree = {tree} AND l.type IN ('utc', 'utp', 'utd')
                   AND f.label = 'Conversation' AND f.path = 'Conversation' AND f.value <> ''
                   AND {}
                 ORDER BY 2, 3",
                self.blueprint_scope("l")
            ))?
            .rows
        {
            let o = Object {
                resource: format!("{}.{}", s(&r[1]), s(&r[2])),
                tag: s(&r[5]),
                location: Self::place(&s(&r[3]), &s(&r[4])),
            };
            let list = owners.entry(s(&r[0])).or_default();
            if !list.contains(&o) {
                list.push(o);
            }
        }
        self.runners_of = runners;
        self.owners_of = owners;
        self.loaded = true;
        Ok(())
    }

    /// SQL true for a blueprint location in scope: in the module's
    /// capsules, or named by a template of the module's instances.
    fn blueprint_scope(&self, l: &str) -> String {
        if self.scope.capsules.is_empty() {
            return "1".to_string();
        }
        let caps = list(self.scope.capsules.iter().map(String::as_str));
        format!(
            "({l}.path IN ({caps}) OR {l}.name IN (
                SELECT lower(g.value) FROM gff_fields g JOIN locations m ON m.hash = g.hash
                WHERE m.tree = {} AND m.path IN ({caps}) AND g.label = 'TemplateResRef'))",
            self.scope.tree
        )
    }

    /// Who `OBJECT_SELF` is for each runner of `script`, as far as the
    /// index shows it.
    fn selves(&mut self, script: &str, depth: u8) -> Result<Rc<Vec<RunBy>>, Diagnostic> {
        let key = (script.to_ascii_lowercase(), depth);
        if let Some(v) = self.selves_of.get(&key) {
            return Ok(v.clone());
        }
        let mut out = Vec::new();
        for r in self.runners(script)? {
            let objects = match &r {
                Runner::Node { dialog, .. } | Runner::Condition { dialog, .. } => {
                    self.dialog_owners(dialog)?
                }
                Runner::Event {
                    resource,
                    tag,
                    location,
                    ..
                } => tag
                    .iter()
                    .map(|t| Object {
                        resource: resource.clone(),
                        tag: t.clone(),
                        location: location.clone(),
                    })
                    .collect(),
                Runner::Execute { script, target, .. } => match target {
                    ObjectArg::Tag(t) => vec![Object {
                        resource: String::new(),
                        tag: t.clone(),
                        location: String::new(),
                    }],
                    ObjectArg::SelfObject if depth > 0 => self
                        .selves(script, depth - 1)?
                        .iter()
                        .flat_map(|r| r.objects.iter().cloned())
                        .collect(),
                    _ => Vec::new(),
                },
            };
            out.push(RunBy { runner: r, objects });
        }
        let out = Rc::new(out);
        self.selves_of.insert(key, out.clone());
        Ok(out)
    }

    /// Whether a runner lies in scope: in the module's capsules, or
    /// anywhere without a module.
    fn runner_in_scope(&self, r: &RunBy) -> bool {
        if self.scope.capsules.is_empty() {
            return true;
        }
        let caps: Vec<&str> = self
            .scope
            .capsules
            .iter()
            .map(|p| p.rsplit('/').next().unwrap_or(p))
            .collect();
        match &r.runner {
            Runner::Node { location, .. }
            | Runner::Condition { location, .. }
            | Runner::Event { location, .. } => caps.contains(&location.as_str()),
            Runner::Execute { .. } => true,
        }
    }

    /// Whether the call's script lies in scope: in the module's capsules,
    /// or run by something in them.
    fn call_in_scope(&self, c: &Call) -> bool {
        self.in_scope(&c.locations, &c.run_by)
    }

    /// Whether a script held at `locations` and run by `run_by` lies in
    /// scope.
    fn in_scope(&self, locations: &[String], run_by: &[RunBy]) -> bool {
        if self.scope.capsules.is_empty() {
            return true;
        }
        let caps: Vec<&str> = self
            .scope
            .capsules
            .iter()
            .map(|p| p.rsplit('/').next().unwrap_or(p))
            .collect();
        locations.iter().any(|l| caps.contains(&l.as_str()))
            || run_by.iter().any(|r| self.runner_in_scope(r))
    }

    /// The include constants naming each local's index: (kind, index) to
    /// names.
    fn index_names(&self) -> Result<BTreeMap<(&'static str, i64), Vec<String>>, Diagnostic> {
        let rows = self.q(&format!(
            "SELECT l.name, s.body FROM search s JOIN locations l ON l.hash = s.hash
             WHERE l.tree = {} AND l.type = 'nss' AND s.kind = 'source'
             GROUP BY l.name ORDER BY l.name",
            self.scope.tree
        ))?;
        let mut out: BTreeMap<(&'static str, i64), Vec<String>> = BTreeMap::new();
        for r in rows.rows {
            let body = s(&r[1]);
            if body.contains("void main(") || body.contains("int StartingConditional(") {
                continue;
            }
            for line in body.lines() {
                let Some((name, n)) = int_constant(line) else {
                    continue;
                };
                let kind = if BOOLEAN_NAMES.iter().any(|p| name.starts_with(p)) {
                    "boolean"
                } else if NUMBER_NAMES.iter().any(|p| name.starts_with(p)) {
                    "number"
                } else {
                    continue;
                };
                let names = out.entry((kind, n)).or_default();
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
        Ok(out)
    }

    /// The `globalcat.2da` row naming `name`, with its type.
    fn global(&self, name: &str) -> Result<Option<(String, String)>, Diagnostic> {
        let rows = self.q(&format!(
            "SELECT n.value, t.value FROM locations l
             JOIN cells n ON n.hash = l.hash AND n.column = 'name'
             LEFT JOIN cells t ON t.hash = l.hash AND t.row = n.row AND t.column = 'type'
             WHERE l.tree = {} AND l.name = 'globalcat' AND l.type = '2da'
               AND lower(n.value) = lower({})
             ORDER BY lower(substr(l.path, 1, 9)) = 'override/' DESC LIMIT 1",
            self.scope.tree,
            lit(name)
        ))?;
        Ok(rows.rows.first().map(|r| (s(&r[0]), s(&r[1]))))
    }

    /// The objects in scope bearing `tag`.
    fn tagged(&self, tag: &str) -> Result<Vec<Object>, Diagnostic> {
        let rows = self.q(&format!(
            "SELECT DISTINCT l.name, l.type, l.path, l.member, f.value FROM gff_fields f
             JOIN locations l ON l.hash = f.hash
             WHERE l.tree = {} AND f.label = 'Tag' AND f.path = 'Tag' AND lower(f.value) = lower({})
               AND l.type IN ('utc', 'utp', 'utd', 'utt', 'ute', 'utw', 'uts', 'utm')
               AND {}
             ORDER BY 1, 2",
            self.scope.tree,
            lit(tag),
            self.blueprint_scope("l")
        ))?;
        Ok(rows
            .rows
            .iter()
            .map(|r| Object {
                resource: format!("{}.{}", s(&r[0]), s(&r[1])),
                tag: s(&r[4]),
                location: Self::place(&s(&r[2]), &s(&r[3])),
            })
            .collect())
    }
}

/// `int NAME = N;` or `const int NAME = N;` at the start of a line.
fn int_constant(line: &str) -> Option<(String, i64)> {
    if line.starts_with([' ', '\t']) {
        return None;
    }
    let rest = line.strip_prefix("const ").unwrap_or(line);
    let rest = rest.strip_prefix("int")?.trim_start();
    let (name, rest) = rest.split_once('=')?;
    let name = name.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let value = rest
        .trim()
        .strip_suffix(';')
        .or_else(|| rest.split_once(';').map(|(v, _)| v))?;
    Some((name.to_string(), value.trim().parse().ok()?))
}

/// An object a query names or a script acts on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Object {
    /// The blueprint, `name.type`; empty for an object known only by tag.
    pub resource: String,
    pub tag: String,
    /// The capsule or path holding the blueprint.
    pub location: String,
}

/// What runs a script.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Runner {
    /// A dialogue node's `Script`.
    Node {
        dialog: String,
        node: String,
        location: String,
    },
    /// A dialogue link's `Active` condition.
    Condition {
        dialog: String,
        source: String,
        target: String,
        location: String,
    },
    /// An object's event field.
    Event {
        resource: String,
        field: String,
        tag: Option<String>,
        location: String,
    },
    /// An `ExecuteScript` call.
    Execute {
        script: String,
        site: i64,
        target: ObjectArg,
    },
}

impl Runner {
    pub fn text(&self) -> String {
        match self {
            Runner::Node { dialog, node, .. } => format!("{dialog}.dlg {node}"),
            Runner::Condition {
                dialog,
                source,
                target,
                ..
            } => format!("{dialog}.dlg {source}->{target} condition"),
            Runner::Event {
                resource, field, ..
            } => format!("{resource} {field}"),
            Runner::Execute { script, site, .. } => format!("ExecuteScript in {script} @{site}"),
        }
    }
}

/// A runner and who `OBJECT_SELF` is under it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunBy {
    pub runner: Runner,
    pub objects: Vec<Object>,
}

/// An object argument as the index resolves it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ObjectArg {
    /// `OBJECT_SELF`: the object running the script.
    SelfObject,
    /// An object found by tag (`GetObjectByTag("k37_inv_statue")`).
    Tag(String),
    /// Anything else, described.
    Unresolved(String),
}

fn object_arg(v: &Value) -> ObjectArg {
    match v {
        Value::Object(o) if o.get("object").and_then(Value::as_str) == Some("OBJECT_SELF") => {
            ObjectArg::SelfObject
        }
        Value::Object(o) if o.contains_key("call") => {
            let routine = o.get("call").and_then(Value::as_str).unwrap_or_default();
            let first = o
                .get("args")
                .and_then(Value::as_array)
                .and_then(|a| a.first())
                .and_then(Value::as_str);
            match first {
                Some(t) if routine.contains("ByTag") => ObjectArg::Tag(t.to_string()),
                _ => ObjectArg::Unresolved(format!("{routine}()")),
            }
        }
        Value::Object(o) => ObjectArg::Unresolved(o.get("object").map_or_else(|| v.to_string(), s)),
        _ => ObjectArg::Unresolved("computed".to_string()),
    }
}

/// One call of a script.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Call {
    pub script: String,
    #[serde(skip)]
    pub hash: String,
    /// The offset where the arguments are known: the call in the script's
    /// code, or the `JSR` into the include function that makes it.
    pub site: i64,
    /// The offsets from the site to the `ACTION`.
    pub path: Vec<i64>,
    pub routine: String,
    pub args: Vec<Value>,
    /// The capsules or paths holding the script.
    pub locations: Vec<String>,
    /// What runs the script, with who `OBJECT_SELF` is.
    pub run_by: Vec<RunBy>,
}

impl Call {
    fn at(&self) -> String {
        format!("{} @{}", self.script, self.site)
    }

    fn runners_text(&self) -> String {
        if self.run_by.is_empty() {
            return "run by nothing the index holds".to_string();
        }
        self.run_by
            .iter()
            .map(|r| r.runner.text())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// A literal argument as text: a number, a quoted string, or `computed`.
fn arg_text(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => "computed".to_string(),
        Some(Value::String(s)) => format!("\"{s}\""),
        Some(v) => v.to_string(),
    }
}

/// What a `flags` query asks.
#[derive(Debug, Clone)]
pub struct FlagsQuery<'a> {
    /// An object's tag or a global's name.
    pub thing: &'a str,
    pub module: Option<&'a str>,
    pub tree: Option<&'a str>,
}

/// One write or read of a flag.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Access {
    /// The value written, as a literal; `None` for a read or a computed
    /// value.
    pub value: Option<Value>,
    /// How the object is named: `OBJECT_SELF` or the tag.
    pub object: ObjectArg,
    pub call: Call,
}

/// One flag: a local's kind and index, or a global.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Flag {
    /// `boolean` or `number` for a local; the global's type for a global.
    pub kind: String,
    /// The local's index, as the call passes it; `None` for a global.
    pub index: Option<Value>,
    /// The include constants naming the index.
    pub names: Vec<String>,
    pub writes: Vec<Access>,
    pub reads: Vec<Access>,
}

/// The answer to a [`flags`] query.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Flags {
    pub thing: String,
    /// `object` or `global`.
    pub of: &'static str,
    pub scope: Scope,
    /// The objects in scope bearing the tag.
    pub objects: Vec<Object>,
    pub flags: Vec<Flag>,
    /// Calls on `OBJECT_SELF` in scripts in scope that nothing the index
    /// holds runs, so whose object is unknown, with the tag's flags among
    /// what they could set.
    pub unresolved_self: Vec<Call>,
    /// Local-variable calls in scope whose object the index cannot follow.
    pub unresolved_objects: usize,
}

/// Who writes and reads the local flags of the object tagged
/// `q.thing`, or the global it names.
pub fn flags(conn: &Connection, q: &FlagsQuery) -> Result<(Flags, Vec<Diagnostic>), Diagnostic> {
    let mut r = Reader::open(conn, q.tree, q.module)?;
    let mut diags = Vec::new();
    let global = r.global(q.thing)?;
    let mut out = Flags {
        thing: q.thing.to_string(),
        of: if global.is_some() { "global" } else { "object" },
        scope: r.scope.clone(),
        objects: Vec::new(),
        flags: Vec::new(),
        unresolved_self: Vec::new(),
        unresolved_objects: 0,
    };
    if let Some((name, ty)) = global {
        let calls = r.calls(
            &[
                "SetGlobalBoolean",
                "SetGlobalNumber",
                "SetGlobalString",
                "GetGlobalBoolean",
                "GetGlobalNumber",
                "GetGlobalString",
            ],
            &format!(
                "lower(json_extract(c.args, '$[0]')) = lower({})",
                lit(&name)
            ),
        )?;
        let mut flag = Flag {
            kind: ty.to_ascii_lowercase(),
            index: None,
            names: vec![name.clone()],
            writes: Vec::new(),
            reads: Vec::new(),
        };
        for mut c in calls {
            c.run_by = (*r.selves(&c.script, 1)?).clone();
            if !r.call_in_scope(&c) {
                continue;
            }
            let write = c.routine.starts_with("Set");
            let a = Access {
                value: write.then(|| c.args.get(1).cloned()).flatten(),
                object: ObjectArg::Unresolved("global".to_string()),
                call: c,
            };
            if write {
                flag.writes.push(a);
            } else {
                flag.reads.push(a);
            }
        }
        out.thing = name;
        out.flags.push(flag);
        return Ok((out, diags));
    }
    out.objects = r.tagged(q.thing)?;
    let names = r.index_names()?;
    let routines: Vec<&str> = LOCAL_SETS.iter().chain(LOCAL_GETS).copied().collect();
    let calls = r.calls(&routines, "1")?;
    let tag = q.thing.to_ascii_lowercase();
    let mut by_flag: BTreeMap<(String, String), Flag> = BTreeMap::new();
    let mut unresolved_self: Vec<Call> = Vec::new();
    for mut c in calls {
        let object = c
            .args
            .first()
            .map(object_arg)
            .unwrap_or(ObjectArg::Unresolved("computed".to_string()));
        let hit = match &object {
            ObjectArg::Tag(t) => t.eq_ignore_ascii_case(&tag),
            ObjectArg::SelfObject => {
                let rb = r.selves(&c.script, 1)?;
                let owned = rb
                    .iter()
                    .any(|rb| rb.objects.iter().any(|o| o.tag.eq_ignore_ascii_case(&tag)));
                if !owned && rb.iter().all(|rb| rb.objects.is_empty()) {
                    if r.in_scope(&c.locations, &rb) {
                        c.run_by = (*rb).clone();
                        unresolved_self.push(c);
                    }
                    continue;
                }
                owned
            }
            ObjectArg::Unresolved(_) => {
                let rb = r.selves(&c.script, 1)?;
                if r.in_scope(&c.locations, &rb) {
                    out.unresolved_objects += 1;
                }
                continue;
            }
        };
        if !hit {
            continue;
        }
        if c.run_by.is_empty() {
            c.run_by = (*r.selves(&c.script, 1)?).clone();
        }
        if !r.call_in_scope(&c) {
            continue;
        }
        let kind = if c.routine.ends_with("Boolean") {
            "boolean"
        } else {
            "number"
        };
        let index = c.args.get(1).cloned().unwrap_or(Value::Null);
        let key = (kind.to_string(), index_key(&index));
        let flag = by_flag.entry(key).or_insert_with(|| Flag {
            kind: kind.to_string(),
            names: index
                .as_i64()
                .and_then(|n| names.get(&(kind, n)))
                .cloned()
                .unwrap_or_default(),
            index: (!index.is_null()).then(|| index.clone()),
            writes: Vec::new(),
            reads: Vec::new(),
        });
        let write = c.routine.starts_with("Set");
        let a = Access {
            value: write.then(|| c.args.get(2).cloned()).flatten(),
            object,
            call: c,
        };
        if write {
            flag.writes.push(a);
        } else {
            flag.reads.push(a);
        }
    }
    out.flags = by_flag.into_values().collect();
    out.unresolved_self = unresolved_self;
    if out.objects.is_empty() && out.flags.is_empty() {
        diags.push(diagnostic(
            c::NOTHING,
            format!(
                "no object {} bears the tag `{}`, no script sets or reads a local on it, and globalcat.2da declares no global of that name",
                match &r.scope.module {
                    Some(m) => format!("of the module `{m}`"),
                    None => format!("of the tree `{}`", r.scope.label),
                },
                q.thing
            ),
        ));
    }
    Ok((out, diags))
}

/// A sort key for an index: numbers in order, then the rest.
fn index_key(v: &Value) -> String {
    match v.as_i64() {
        Some(n) => format!("0{:012}", n + 1_000_000),
        None => format!("1{v}"),
    }
}

impl Flags {
    /// The answer as text.
    pub fn text(&self) -> String {
        let mut out = String::new();
        let scope = match &self.scope.module {
            Some(m) => format!("module {m}"),
            None => format!("tree {}", self.scope.label),
        };
        if self.of == "global" {
            let _ = writeln!(
                out,
                "global {} ({}), {scope}",
                self.thing, self.flags[0].kind
            );
        } else {
            let _ = writeln!(out, "{} in {scope}", self.thing);
            for o in &self.objects {
                let _ = writeln!(out, "  {} ({})", o.resource, o.location);
            }
            if self.objects.is_empty() {
                let _ = writeln!(out, "  no blueprint in scope bears the tag");
            }
        }
        for f in &self.flags {
            if self.of == "object" {
                let index = f
                    .index
                    .as_ref()
                    .map_or_else(|| "computed".to_string(), |v| v.to_string());
                let names = if f.names.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", f.names.join(" / "))
                };
                let _ = writeln!(out, "{} {index}{names}", f.kind);
            }
            let rows = |out: &mut String, title: &str, list: &[Access], write: bool| {
                if list.is_empty() {
                    return;
                }
                let _ = writeln!(out, "  {title}:");
                for a in list {
                    let value = if write {
                        format!("= {}  ", arg_text(a.value.as_ref()))
                    } else {
                        String::new()
                    };
                    let how = match &a.object {
                        ObjectArg::SelfObject => "OBJECT_SELF, ".to_string(),
                        ObjectArg::Tag(_) => "by tag, ".to_string(),
                        ObjectArg::Unresolved(_) => String::new(),
                    };
                    let _ = writeln!(
                        out,
                        "    {value}{}  ({how}run by {})",
                        a.call.at(),
                        a.call.runners_text().trim_start_matches("run by ")
                    );
                }
            };
            rows(&mut out, "writes", &f.writes, true);
            rows(&mut out, "reads", &f.reads, false);
        }
        if !self.unresolved_self.is_empty() {
            let mut scripts: Vec<&str> = self
                .unresolved_self
                .iter()
                .map(|c| c.script.as_str())
                .collect();
            scripts.dedup();
            const SHOWN: usize = 12;
            let more = if scripts.len() > SHOWN {
                format!(
                    ", and {} more (--format json lists every call)",
                    scripts.len() - SHOWN
                )
            } else {
                String::new()
            };
            let _ = writeln!(
                out,
                "OBJECT_SELF unresolved: {} calls in {} scripts nothing in the index runs: {}{more}",
                self.unresolved_self.len(),
                scripts.len(),
                scripts[..scripts.len().min(SHOWN)].join(", ")
            );
        }
        if self.unresolved_objects > 0 {
            let _ = writeln!(
                out,
                "objects not followed: {} local-variable calls in scope name an object the index cannot resolve",
                self.unresolved_objects
            );
        }
        out
    }
}

/// What a `quest` query asks.
#[derive(Debug, Clone)]
pub struct QuestQuery<'a> {
    /// The category's `Tag`; `None` for every category (lints only).
    pub tag: Option<&'a str>,
    pub tree: Option<&'a str>,
    /// The entries' and the category's text from `dialog.tlk`.
    pub text: bool,
}

/// One entry of a journal category.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    pub id: i64,
    pub end: bool,
    pub strref: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub xp_percentage: Option<f64>,
    pub writers: Vec<Writer>,
}

/// What writes a quest entry.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Writer {
    /// A dialogue node's `Quest` and `QuestEntry`.
    Node {
        dialog: String,
        node: String,
        location: String,
        /// The node's own script.
        script: Option<String>,
        /// The conditions of the links into the node: (source, condition).
        conditions: Vec<(String, Option<String>)>,
        /// The globals the node's script sets: (name, value).
        globals: Vec<(String, Value)>,
    },
    /// A script's `AddJournalQuestEntry`.
    Script {
        call: Call,
        globals: Vec<(String, Value)>,
    },
}

impl Writer {
    fn text(&self) -> String {
        match self {
            Writer::Node {
                dialog,
                node,
                location,
                script,
                conditions,
                globals,
            } => {
                let mut s = format!("{dialog}.dlg {node} ({location})");
                if let Some(sc) = script {
                    let _ = write!(s, ", runs {sc}");
                }
                let conds: Vec<String> = conditions
                    .iter()
                    .filter_map(|(src, c)| c.as_ref().map(|c| format!("{src}->{node} if {c}")))
                    .collect();
                if !conds.is_empty() {
                    let _ = write!(s, "; {}", conds.join(", "));
                }
                if !globals.is_empty() {
                    let _ = write!(s, "; sets {}", globals_text(globals));
                }
                s
            }
            Writer::Script { call, globals } => {
                let mut s = format!("{} ({})", call.at(), call.runners_text());
                if !globals.is_empty() {
                    let _ = write!(s, "; sets {}", globals_text(globals));
                }
                s
            }
        }
    }
}

fn globals_text(g: &[(String, Value)]) -> String {
    g.iter()
        .map(|(n, v)| format!("{n} = {}", arg_text(Some(v))))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A journal category with its entries and what touches it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Quest {
    pub tag: String,
    /// The category's index in `global.jrl`'s `Categories`.
    pub category: i64,
    pub name_strref: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub comment: String,
    pub priority: Option<i64>,
    pub plot_index: Option<i64>,
    pub planet_id: Option<i64>,
    /// The entries, by id.
    pub entries: Vec<Entry>,
    /// Writes naming an entry the category lacks, or one computed.
    pub other_writes: Vec<(Option<i64>, Writer)>,
    /// What removes the quest: `RemoveJournalQuestEntry`.
    pub removed_by: Vec<Call>,
    /// What reads the quest's state: `GetJournalEntry`.
    pub read_by: Vec<Call>,
}

/// The answer to a [`quest`] query.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Quests {
    pub scope: Scope,
    /// Where `global.jrl` was read from.
    pub journal: String,
    pub quests: Vec<Quest>,
}

/// The journal category tagged `q.tag` (or every category) with each
/// entry's writers, what removes and reads the quest, and the lints.
pub fn quest(conn: &Connection, q: &QuestQuery) -> Result<(Quests, Vec<Diagnostic>), Diagnostic> {
    let mut r = Reader::open(conn, q.tree, None)?;
    let tree = r.scope.tree;
    let jrl = r.q(&format!(
        "SELECT hash, path, member FROM locations WHERE tree = {tree} AND name = 'global' AND type = 'jrl'
         ORDER BY lower(substr(path, 1, 9)) = 'override/' DESC, path LIMIT 1"
    ))?;
    let Some(j) = jrl.rows.first() else {
        return Err(diagnostic(
            c::NO_QUEST,
            format!("the tree `{}` holds no global.jrl", r.scope.label),
        ));
    };
    let hash = s(&j[0]);
    let journal = if s(&j[2]).is_empty() {
        s(&j[1])
    } else {
        format!("{}!{}", s(&j[1]), s(&j[2]))
    };
    let fields = r.q(&format!(
        "SELECT path, value, strref FROM gff_fields WHERE hash = {} AND path LIKE 'Categories[%'",
        lit(&hash)
    ))?;
    // Category index to its fields, and to its entries' fields.
    let mut cats: BTreeMap<i64, BTreeMap<String, (Value, Value)>> = BTreeMap::new();
    let mut entries: BTreeMap<(i64, i64), BTreeMap<String, (Value, Value)>> = BTreeMap::new();
    for row in &fields.rows {
        let path = s(&row[0]);
        let Some((cat, rest)) = list_index(&path, "Categories") else {
            continue;
        };
        if let Some((e, field)) = list_index(rest, "EntryList") {
            entries
                .entry((cat, e))
                .or_default()
                .insert(field.to_string(), (row[1].clone(), row[2].clone()));
        } else {
            cats.entry(cat)
                .or_default()
                .insert(rest.to_string(), (row[1].clone(), row[2].clone()));
        }
    }
    let wanted: Vec<i64> = cats
        .iter()
        .filter(|(_, f)| match q.tag {
            Some(t) => f
                .get("Tag")
                .is_some_and(|(v, _)| s(v).eq_ignore_ascii_case(t)),
            None => true,
        })
        .map(|(i, _)| *i)
        .collect();
    if wanted.is_empty() {
        return Err(diagnostic(
            c::NO_QUEST,
            format!(
                "{journal} has no category tagged `{}`",
                q.tag.unwrap_or_default()
            ),
        ));
    }
    let tlk = if q.text {
        r.q(&format!(
            "SELECT hash FROM locations WHERE tree = {tree} AND member = '' AND lower(path) = 'dialog.tlk'"
        ))?
        .rows
        .first()
        .map(|r| s(&r[0]))
    } else {
        None
    };
    let text_of = |r: &Reader, strref: Option<i64>| -> Result<Option<String>, Diagnostic> {
        let (Some(h), Some(n)) = (&tlk, strref) else {
            return Ok(None);
        };
        Ok(r.q(&format!(
            "SELECT text FROM strings WHERE hash = {} AND strref = {n}",
            lit(h)
        ))?
        .rows
        .first()
        .map(|x| s(&x[0])))
    };
    // Every dialogue node and script call writing a quest, by tag.
    let tags: Vec<String> = wanted
        .iter()
        .map(|i| s(&cats[i].get("Tag").map(|f| f.0.clone()).unwrap_or_default()))
        .collect();
    let lowered: Vec<String> = tags.iter().map(|t| t.to_ascii_lowercase()).collect();
    let tag_filter = |col: &str| {
        if q.tag.is_some() {
            format!(
                "lower({col}) IN ({})",
                list(lowered.iter().map(String::as_str))
            )
        } else {
            "1".to_string()
        }
    };
    let node_rows = r.q(&format!(
        "SELECT DISTINCT l.name, l.path, l.member, n.hash, n.node, n.quest, n.quest_entry, n.script
         FROM dialog_nodes n JOIN locations l ON l.hash = n.hash
         WHERE l.tree = {tree} AND n.quest IS NOT NULL AND n.quest <> '' AND {}
         ORDER BY 1, 5",
        tag_filter("n.quest")
    ))?;
    let adds = r.calls(
        &["AddJournalQuestEntry"],
        &tag_filter("json_extract(c.args, '$[0]')"),
    )?;
    let removes = r.calls(
        &["RemoveJournalQuestEntry"],
        &tag_filter("json_extract(c.args, '$[0]')"),
    )?;
    let reads = r.calls(
        &["GetJournalEntry"],
        &tag_filter("json_extract(c.args, '$[0]')"),
    )?;
    let mut sets_memo: BTreeMap<String, Vec<(String, Value)>> = BTreeMap::new();
    let mut sets = |r: &Reader, script: &str| -> Result<Vec<(String, Value)>, Diagnostic> {
        let key = script.to_ascii_lowercase();
        if let Some(v) = sets_memo.get(&key) {
            return Ok(v.clone());
        }
        let rows = r.q(&format!(
            "SELECT DISTINCT json_extract(c.args, '$[0]'), json_extract(c.args, '$[1]'), c.offset
             FROM calls c JOIN locations l ON l.hash = c.hash
             WHERE l.tree = {} AND l.type = 'ncs' AND l.name = {} AND c.routine LIKE 'SetGlobal%'
               AND json_type(c.args, '$[0]') = 'text'
               AND NOT EXISTS (SELECT 1 FROM resolved_calls r
                               WHERE r.hash = c.hash AND r.offset = c.offset)
             UNION
             SELECT DISTINCT json_extract(c.args, '$[0]'), json_extract(c.args, '$[1]'), c.offset
             FROM resolved_calls c JOIN locations l ON l.hash = c.hash
             WHERE l.tree = {} AND l.type = 'ncs' AND l.name = {} AND c.routine LIKE 'SetGlobal%'
               AND json_type(c.args, '$[0]') = 'text'
             ORDER BY 3",
            r.scope.tree,
            lit(&key),
            r.scope.tree,
            lit(&key)
        ))?;
        let mut v: Vec<(String, Value)> = Vec::new();
        for row in rows.rows {
            let g = (s(&row[0]), row[1].clone());
            if !v.contains(&g) {
                v.push(g);
            }
        }
        sets_memo.insert(key, v.clone());
        Ok(v)
    };
    let mut diags = Vec::new();
    let mut quests = Vec::new();
    for &ci in &wanted {
        let cat = &cats[&ci];
        let field = |k: &str| cat.get(k).map(|f| f.0.clone()).unwrap_or(Value::Null);
        let tag = s(&field("Tag"));
        let name_strref = cat.get("Name").and_then(|f| f.1.as_i64());
        let mut list: Vec<Entry> = Vec::new();
        for (_, e) in entries.range((ci, i64::MIN)..=(ci, i64::MAX)) {
            let get = |k: &str| e.get(k).map(|f| f.0.clone()).unwrap_or(Value::Null);
            let strref = e.get("Text").and_then(|f| f.1.as_i64());
            list.push(Entry {
                id: get("ID").as_i64().unwrap_or(-1),
                end: get("End").as_i64().unwrap_or(0) != 0,
                strref,
                text: text_of(&r, strref)?,
                xp_percentage: get("XP_Percentage").as_f64(),
                writers: Vec::new(),
            });
        }
        list.sort_by_key(|e| e.id);
        let mut other = Vec::new();
        for row in &node_rows.rows {
            if !s(&row[5]).eq_ignore_ascii_case(&tag) {
                continue;
            }
            let dialog_hash = s(&row[3]);
            let node = s(&row[4]);
            let links = r.q(&format!(
                "SELECT DISTINCT source, condition FROM dialog_links WHERE hash = {} AND target = {}
                 ORDER BY source",
                lit(&dialog_hash),
                lit(&node)
            ))?;
            let script = match &row[7] {
                Value::String(x) if !x.is_empty() => Some(x.clone()),
                _ => None,
            };
            let globals = match &script {
                Some(sc) => sets(&r, sc)?,
                None => Vec::new(),
            };
            let w = Writer::Node {
                dialog: s(&row[0]),
                node,
                location: Reader::place(&s(&row[1]), &s(&row[2])),
                script,
                conditions: links
                    .rows
                    .iter()
                    .map(|l| {
                        (
                            s(&l[0]),
                            l[1].as_str().filter(|x| !x.is_empty()).map(str::to_string),
                        )
                    })
                    .collect(),
                globals,
            };
            let id = row[6].as_i64();
            push_writer(&mut list, &mut other, id, w);
        }
        for c in adds.iter().filter(|c| {
            c.args
                .first()
                .and_then(Value::as_str)
                .is_some_and(|t| t.eq_ignore_ascii_case(&tag))
        }) {
            let mut c = c.clone();
            c.run_by = (*r.selves(&c.script, 1)?).clone();
            let globals = sets(&r, &c.script)?;
            let id = c.args.get(1).and_then(Value::as_i64);
            push_writer(
                &mut list,
                &mut other,
                id,
                Writer::Script { call: c, globals },
            );
        }
        let pick = |calls: &[Call], r: &mut Reader| -> Result<Vec<Call>, Diagnostic> {
            let mut out = Vec::new();
            for c in calls.iter().filter(|c| {
                c.args
                    .first()
                    .and_then(Value::as_str)
                    .is_some_and(|t| t.eq_ignore_ascii_case(&tag))
            }) {
                let mut c = c.clone();
                c.run_by = (*r.selves(&c.script, 1)?).clone();
                out.push(c);
            }
            Ok(out)
        };
        let removed_by = pick(&removes, &mut r)?;
        let read_by = pick(&reads, &mut r)?;
        // The lints.
        let computed = other.iter().filter(|(id, _)| id.is_none()).count();
        for e in list.iter().filter(|e| e.writers.is_empty()) {
            let mut d = diagnostic(
                c::UNWRITTEN,
                format!(
                    "entry {} of the quest `{tag}` is written by no dialogue node and no script call the index holds",
                    e.id
                ),
            );
            if computed > 0 {
                d = d.note(format!(
                    "{computed} AddJournalQuestEntry call(s) for the quest compute the entry; one may write it"
                ));
            }
            diags.push(d);
        }
        for (id, w) in other.iter().filter(|(id, _)| id.is_some()) {
            diags.push(diagnostic(
                c::NO_ENTRY,
                format!(
                    "{} writes entry {} of the quest `{tag}`, which global.jrl's category lacks",
                    w.text(),
                    id.unwrap_or_default()
                ),
            ));
        }
        for d in sibling_lint(&mut r, &tag, &node_rows.rows, &mut sets)? {
            diags.push(d);
        }
        quests.push(Quest {
            tag,
            category: ci,
            name_strref,
            name: text_of(&r, name_strref)?,
            comment: s(&field("Comment")),
            priority: field("Priority").as_i64(),
            plot_index: field("PlotIndex").as_i64(),
            planet_id: field("PlanetID").as_i64(),
            entries: list,
            other_writes: other,
            removed_by,
            read_by,
        });
    }
    Ok((
        Quests {
            scope: r.scope.clone(),
            journal,
            quests,
        },
        diags,
    ))
}

fn push_writer(
    list: &mut [Entry],
    other: &mut Vec<(Option<i64>, Writer)>,
    id: Option<i64>,
    w: Writer,
) {
    match id.and_then(|id| list.iter_mut().find(|e| e.id == id)) {
        Some(e) => e.writers.push(w),
        None => other.push((id, w)),
    }
}

/// `List[N].rest` as `N` and `rest`.
fn list_index<'p>(path: &'p str, list: &str) -> Option<(i64, &'p str)> {
    let rest = path.strip_prefix(list)?.strip_prefix('[')?;
    let (n, rest) = rest.split_once(']')?;
    Some((n.parse().ok()?, rest.strip_prefix('.').unwrap_or(rest)))
}

/// The globals a script sets, with the values its calls pass: (name,
/// value).
type GlobalsOf<'a> = dyn FnMut(&Reader, &str) -> Result<Vec<(String, Value)>, Diagnostic> + 'a;

/// The replies that do what a sibling writing the quest does and write no
/// entry: for each reply writing `tag` with a script, the other replies
/// the same entry offers, when one runs a script setting a global the
/// writer's script also sets, and neither it nor its script writes the
/// quest. Entries an NPC speaks and the starting list are left out: their
/// conditions choose one, so a sibling there is another situation, not
/// another choice the player makes.
fn sibling_lint(
    r: &mut Reader,
    tag: &str,
    node_rows: &[Vec<Value>],
    sets: &mut GlobalsOf,
) -> Result<Vec<Diagnostic>, Diagnostic> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for row in node_rows {
        if !s(&row[5]).eq_ignore_ascii_case(tag) {
            continue;
        }
        let Some(script) = row[7].as_str().filter(|x| !x.is_empty()) else {
            continue;
        };
        let theirs: BTreeSet<String> = sets(r, script)?.into_iter().map(|(n, _)| n).collect();
        if theirs.is_empty() {
            continue;
        }
        let dialog_hash = s(&row[3]);
        let node = s(&row[4]);
        let siblings = r.q(&format!(
            "SELECT DISTINCT k2.source, k2.target, n.script, n.quest FROM dialog_links k1
             JOIN dialog_links k2 ON k2.hash = k1.hash AND k2.source = k1.source AND k2.target <> k1.target
             JOIN dialog_nodes n ON n.hash = k2.hash AND n.node = k2.target
             WHERE k1.hash = {} AND k1.target = {} AND k1.source <> 'start' AND k2.target LIKE 'R%'
             ORDER BY 2",
            lit(&dialog_hash),
            lit(&node)
        ))?;
        for sib in siblings.rows {
            let target = s(&sib[1]);
            let Some(sib_script) = sib[2].as_str().filter(|x| !x.is_empty()) else {
                continue;
            };
            if s(&sib[3]).eq_ignore_ascii_case(tag) {
                continue;
            }
            if !seen.insert((dialog_hash.clone(), target.clone())) {
                continue;
            }
            let adds = r.calls(
                &["AddJournalQuestEntry"],
                &format!(
                    "lower(json_extract(c.args, '$[0]')) = lower({}) AND c.hash IN (
                       SELECT hash FROM locations WHERE name = {} AND type = 'ncs')",
                    lit(tag),
                    lit(&sib_script.to_ascii_lowercase())
                ),
            )?;
            if !adds.is_empty() {
                continue;
            }
            let mine: BTreeSet<String> = sets(r, sib_script)?.into_iter().map(|(n, _)| n).collect();
            let shared: Vec<&String> = mine.intersection(&theirs).collect();
            if shared.is_empty() {
                continue;
            }
            out.push(
                diagnostic(
                    c::SIBLING,
                    format!(
                        "{}.dlg {target} (runs {sib_script}) writes no entry of the quest `{tag}`, while its sibling {node} (runs {script}) writes one and sets the same {}",
                        s(&row[0]),
                        shared.iter().map(|x| x.as_str()).collect::<Vec<_>>().join(", ")
                    ),
                )
                .note(format!(
                    "both are reached from {}",
                    s(&sib[0])
                )),
            );
        }
    }
    Ok(out)
}

impl Quests {
    /// The answer as text: each category, its entries in order with their
    /// writers, what removes and reads it.
    pub fn text(&self, lints_only: bool) -> String {
        let mut out = String::new();
        if lints_only {
            let _ = writeln!(
                out,
                "{} journal categories of {} linted",
                self.quests.len(),
                self.journal
            );
            return out;
        }
        for q in &self.quests {
            let _ = writeln!(
                out,
                "quest {} ({} Categories[{}]){}",
                q.tag,
                self.journal,
                q.category,
                q.name
                    .as_ref()
                    .map_or_else(String::new, |n| format!(": {n}"))
            );
            if !q.comment.is_empty() {
                let _ = writeln!(out, "  comment: {}", q.comment);
            }
            for e in &q.entries {
                let _ = writeln!(
                    out,
                    "  {}{}{}",
                    e.id,
                    if e.end { " (end)" } else { "" },
                    e.text
                        .as_ref()
                        .map_or_else(String::new, |t| format!(": {t}"))
                );
                if e.writers.is_empty() {
                    let _ = writeln!(out, "    written by nothing the index holds");
                }
                for w in &e.writers {
                    let _ = writeln!(out, "    {}", w.text());
                }
            }
            for (id, w) in &q.other_writes {
                let _ = writeln!(
                    out,
                    "  entry {}: {}",
                    id.map_or_else(
                        || "computed".to_string(),
                        |i| format!("{i}, not in the journal")
                    ),
                    w.text()
                );
            }
            for (title, calls) in [("removed by", &q.removed_by), ("read by", &q.read_by)] {
                if calls.is_empty() {
                    continue;
                }
                let _ = writeln!(out, "  {title}:");
                for c in calls {
                    let _ = writeln!(out, "    {} ({})", c.at(), c.runners_text());
                }
            }
        }
        out
    }
}
