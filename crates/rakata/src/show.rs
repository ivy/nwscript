//! One resource rendered for a reader, with what it names resolved through
//! the install: what `rakata show` and `rakata decompile` print.
//!
//! An item (`.uti`) shows its name and description through the talk table,
//! its tag, its base item through `baseitems.2da`, and each property
//! through `itempropdef.2da`, its subtype table, its cost table and its
//! parameter table; which tables and columns those are is the measured
//! table `show/items.toml` beside this file. A creature, placeable or door
//! (`.utc`, `.utp`, `.utd`) shows its name, tag, conversation, event
//! scripts and inventory. An area's instances (`.git`) show each instance
//! with its template's tag, name and event scripts, and, from an index,
//! the objects the module's scripts create in it. Any other GFF is not
//! shown here ([`show`] answers `None`).
//!
//! A script is decompiled as a dialogue's conditions are
//! ([`decompile`]): through [`Session`], its include libraries folded and
//! every function verified by recompiling.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use nwscript::diag::Diagnostic;
use rakata_2da::TwoDa;
use rakata_dialog::Lookup as _;
use rakata_game::{Install, Location, ModuleName, Profile};
use rakata_gff::{Gff, LocString, Struct, Value};
use rakata_res::{Gender, ResId, ResRef, ResType};
use rakata_tlk::{Tlk, codepage};
use serde::{Deserialize, Serialize};

use crate::codes;
use crate::corpus::find_ci;
use crate::dialog::Session;

pub use rakata_dialog::{Script, ScriptStatus};

/// The codes this module reports.
pub mod ids {
    /// A name `rakata decompile` takes that is not `NAME.ncs`.
    pub const NOT_SCRIPT: &str = "RE2001";
    /// A script several modules hold, with no module named.
    pub const MODULES: &str = "RE2002";
    /// A script the decompiler could not recover.
    pub const UNRECOVERED: &str = "RE2003";
    /// An index holding no tree built from the install.
    pub const NO_TREE: &str = "RE2004";
    /// A decompiled script some of whose functions did not verify.
    pub const UNVERIFIED: &str = "RW2001";
    /// A 2DA a shown value names that the install lacks or that does not read.
    pub const NO_TABLE: &str = "RW2002";
    /// A blueprint an instance or an inventory names that no layer holds.
    pub const NO_BLUEPRINT: &str = "RW2003";
}
use ids as c;

fn diagnostic(id: &str, message: impl Into<String>) -> Diagnostic {
    let code =
        codes::lookup(id).unwrap_or_else(|| panic!("diagnostic code {id} is not registered"));
    Diagnostic::new(code, message)
}

/// The measured table of the 2DAs and columns that resolve an item.
#[derive(Debug, Deserialize)]
struct ItemTables {
    properties: PropertyTables,
    costs: Indirect,
    params: Params,
    values: Columns,
    base_items: Table,
}

#[derive(Debug, Deserialize)]
struct PropertyTables {
    table: String,
    /// The column naming the subtype 2DA.
    subtype: String,
}

/// A 2DA whose column names another 2DA.
#[derive(Debug, Deserialize)]
struct Indirect {
    table: String,
    resref: String,
}

#[derive(Debug, Deserialize)]
struct Params {
    table: String,
    resref: String,
    /// The `Param1` that names no parameter table.
    none: u8,
}

#[derive(Debug, Deserialize)]
struct Columns {
    /// The column holding a row's strref.
    name: String,
    /// The column shown where the strref names no text.
    label: String,
}

#[derive(Debug, Deserialize)]
struct Table {
    table: String,
}

static TABLES: LazyLock<ItemTables> = LazyLock::new(|| {
    toml::from_str(include_str!("show/items.toml")).expect("show/items.toml parses")
});

/// A row of a 2DA as a reader sees it: the table, the row, its label and
/// the text its strref names.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Row {
    pub table: String,
    pub row: i64,
    /// The row's label column, as written; `None` when blank or absent.
    pub label: Option<String>,
    /// The talk table's text for the row's strref.
    pub name: Option<String>,
    /// Whether the table holds the row.
    pub found: bool,
}

impl Row {
    /// The name, else the label, else the row number.
    pub fn shown(&self) -> String {
        match (&self.name, &self.label, self.found) {
            (Some(n), _, _) => n.clone(),
            (None, Some(l), _) => l.clone(),
            (None, None, true) => format!("row {}", self.row),
            (None, None, false) => format!("{} has no row {}", self.table, self.row),
        }
    }
}

/// An item property: the stored numbers and the rows they name.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Property {
    pub property: Row,
    pub subtype: Option<Row>,
    /// The value, from the property's cost table; `None` when that table
    /// has no rows.
    pub value: Option<Row>,
    /// The parameter's name and value; `None` for `Param1` 255.
    pub param: Option<(Row, Row)>,
    /// The property as one line: `Damage Bonus (Fire): 2`.
    pub text: String,
}

/// An item blueprint.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Item {
    pub resref: Option<String>,
    pub name: Option<String>,
    pub tag: Option<String>,
    pub description: Option<String>,
    pub base_item: Option<Row>,
    pub properties: Vec<Property>,
    pub stack_size: Option<i64>,
    pub charges: Option<i64>,
    pub plot: Option<i64>,
}

/// A script a field names.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Event {
    pub field: String,
    pub script: String,
}

/// An item an inventory names, with the name its blueprint gives it.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Held {
    pub resref: String,
    /// `InventoryRes` or `EquippedRes`.
    pub field: String,
    pub name: Option<String>,
    pub tag: Option<String>,
    /// Whether a layer holds the blueprint.
    pub found: bool,
}

/// A creature, placeable or door blueprint, or an instance of one.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Object {
    /// `creature`, `placeable`, `door` and so on.
    pub kind: String,
    pub resref: Option<String>,
    pub name: Option<String>,
    pub tag: Option<String>,
    pub conversation: Option<String>,
    pub events: Vec<Event>,
    pub inventory: Vec<Held>,
}

/// One instance of an area.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Instance {
    /// The list and index: `Placeable List[3]`.
    pub at: String,
    pub template: Option<String>,
    /// Where the template the engine loads lies; `None` when no layer
    /// holds it.
    pub location: Option<String>,
    #[serde(flatten)]
    pub object: Object,
}

/// An object a script creates.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Spawn {
    pub script: String,
    /// Where the script lies, relative to the install.
    pub location: String,
    /// `creature`, `placeable` and so on, from the call's object type;
    /// `None` when the type is not a literal.
    pub kind: Option<String>,
    pub template: String,
    pub tag: Option<String>,
    pub name: Option<String>,
}

/// An area's instances.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Area {
    pub instances: Vec<Instance>,
    /// From the index, when one is given: the objects scripts create.
    pub spawned: Option<Spawned>,
}

/// What the module's scripts create, from the index.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Spawned {
    /// Each call with a literal template, once per script and template.
    pub objects: Vec<Spawn>,
    /// The scripts that call `CreateObject` with a template the index does
    /// not record as a literal.
    pub computed: Vec<String>,
}

/// A GFF as [`show`] renders it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "shown", rename_all = "lowercase")]
pub enum Shown {
    Item(Item),
    Object(Object),
    Area(Area),
}

impl Shown {
    /// The rendering for a reader.
    pub fn text(&self) -> String {
        let mut out = String::new();
        match self {
            Shown::Item(i) => item_text(i, &mut out),
            Shown::Object(o) => object_text(o, "", &mut out),
            Shown::Area(a) => area_text(a, &mut out),
        }
        out
    }
}

fn opt(v: &Option<String>) -> &str {
    v.as_deref().unwrap_or("(none)")
}

fn item_text(i: &Item, out: &mut String) {
    let _ = writeln!(out, "name: {}", opt(&i.name));
    let _ = writeln!(out, "tag: {}", opt(&i.tag));
    if let Some(b) = &i.base_item {
        let _ = writeln!(out, "base item: {} [{} {}]", b.shown(), b.table, b.row);
    }
    for (label, v) in [
        ("stack size", i.stack_size),
        ("charges", i.charges),
        ("plot", i.plot),
    ] {
        if let Some(v) = v {
            let _ = writeln!(out, "{label}: {v}");
        }
    }
    let _ = writeln!(out, "properties: {}", i.properties.len());
    for p in &i.properties {
        let mut rows = vec![format!("{} {}", p.property.table, p.property.row)];
        rows.extend(p.subtype.iter().map(|r| format!("{} {}", r.table, r.row)));
        rows.extend(p.value.iter().map(|r| format!("{} {}", r.table, r.row)));
        if let Some((_, v)) = &p.param {
            rows.push(format!("{} {}", v.table, v.row));
        }
        let _ = writeln!(out, "  {} [{}]", p.text, rows.join(", "));
    }
    if let Some(d) = &i.description {
        let _ = writeln!(out, "description: {d}");
    }
}

fn object_text(o: &Object, pad: &str, out: &mut String) {
    let _ = writeln!(out, "{pad}name: {}", opt(&o.name));
    let _ = writeln!(out, "{pad}tag: {}", opt(&o.tag));
    if let Some(c) = &o.conversation {
        let _ = writeln!(out, "{pad}conversation: {c}");
    }
    for e in &o.events {
        let _ = writeln!(out, "{pad}{}: {}", e.field, e.script);
    }
    inventory_text(&o.inventory, pad, out);
}

fn inventory_text(inventory: &[Held], pad: &str, out: &mut String) {
    if inventory.is_empty() {
        return;
    }
    let _ = writeln!(out, "{pad}inventory: {}", inventory.len());
    for h in inventory {
        let what = match (&h.name, h.found) {
            (Some(n), _) => format!(" {n:?}"),
            (None, true) => String::new(),
            (None, false) => " (no layer holds it)".to_string(),
        };
        let equipped = if h.field == "EquippedRes" {
            " (equipped)"
        } else {
            ""
        };
        let _ = writeln!(out, "{pad}  {}{what}{}{equipped}", h.resref, tagged(&h.tag));
    }
}

/// ` tag TAG`, or nothing.
fn tagged(tag: &Option<String>) -> String {
    tag.as_deref()
        .map(|t| format!(" tag {t}"))
        .unwrap_or_default()
}

/// ` "NAME"`, or nothing.
fn named(name: &Option<String>) -> String {
    name.as_deref()
        .map(|n| format!(" {n:?}"))
        .unwrap_or_default()
}

fn area_text(a: &Area, out: &mut String) {
    let mut kinds: Vec<&str> = Vec::new();
    for i in &a.instances {
        if !kinds.contains(&i.object.kind.as_str()) {
            kinds.push(&i.object.kind);
        }
    }
    for kind in kinds {
        let of: Vec<&Instance> = a
            .instances
            .iter()
            .filter(|i| i.object.kind == kind)
            .collect();
        let _ = writeln!(out, "{kind}s: {}", of.len());
        for i in of {
            let o = &i.object;
            let missing = if i.location.is_none() && i.template.is_some() {
                " (no layer holds the template)"
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "  {} {}{}{}{missing}",
                i.at,
                i.template.as_deref().unwrap_or("(no template)"),
                named(&o.name),
                tagged(&o.tag),
            );
            if let Some(c) = &o.conversation {
                let _ = writeln!(out, "    conversation: {c}");
            }
            if !o.events.is_empty() {
                let events: Vec<String> = o
                    .events
                    .iter()
                    .map(|e| format!("{} {}", e.field, e.script))
                    .collect();
                let _ = writeln!(out, "    scripts: {}", events.join(", "));
            }
            inventory_text(&o.inventory, "    ", out);
        }
    }
    match &a.spawned {
        None => {
            let _ = writeln!(
                out,
                "spawned by scripts: not listed; pass --db FILE, an index of the install"
            );
        }
        Some(s) => {
            let _ = writeln!(out, "spawned by scripts: {}", s.objects.len());
            for o in &s.objects {
                let kind = o.kind.as_deref().unwrap_or("object of a computed type");
                let _ = writeln!(
                    out,
                    "  {} ({kind}){}{} by {} ({})",
                    o.template,
                    named(&o.name),
                    tagged(&o.tag),
                    o.script,
                    o.location
                );
            }
            if !s.computed.is_empty() {
                let _ = writeln!(
                    out,
                    "  and by {}, a template the index does not know: computed, or passed in a variable",
                    s.computed.join(", ")
                );
            }
        }
    }
}

/// The kind of object a blueprint type is, as the text names it.
fn kind_of(ty: ResType) -> &'static str {
    match ty {
        ResType::Utc => "creature",
        ResType::Utp => "placeable",
        ResType::Utd => "door",
        ResType::Utt => "trigger",
        ResType::Utw => "waypoint",
        ResType::Ute => "encounter",
        ResType::Uts => "sound",
        ResType::Utm => "store",
        ResType::Uti => "item",
        _ => "object",
    }
}

/// The lists of a `.git` and the blueprint type each instance's template
/// is, in the order the text shows them.
const AREA_LISTS: &[(&str, ResType)] = &[
    ("Creature List", ResType::Utc),
    ("Placeable List", ResType::Utp),
    ("Door List", ResType::Utd),
    ("TriggerList", ResType::Utt),
    ("Encounter List", ResType::Ute),
    ("WaypointList", ResType::Utw),
    ("SoundList", ResType::Uts),
    ("StoreList", ResType::Utm),
];

/// The blueprint type `CreateObject`'s object type names: the
/// `OBJECT_TYPE_*` constants of `nwscript.nss`.
fn created(object_type: i64) -> Option<ResType> {
    Some(match object_type {
        1 => ResType::Utc,
        2 => ResType::Uti,
        4 => ResType::Utt,
        8 => ResType::Utd,
        32 => ResType::Utw,
        64 => ResType::Utp,
        128 => ResType::Utm,
        256 => ResType::Ute,
        512 => ResType::Uts,
        _ => return None,
    })
}

/// An install opened to show its resources.
pub struct Reader {
    install: Install,
    module: Option<ModuleName>,
    tlk: Option<Tlk>,
    tables: HashMap<String, Option<TwoDa>>,
    blueprints: HashMap<ResId, Option<(String, Gff)>>,
    warnings: Vec<Diagnostic>,
}

/// Open the install at `game` and its talk table, searching `module`'s
/// capsules too. An install without `dialog.tlk` shows strrefs unresolved.
pub fn open(game: &Path, profile: Profile, module: Option<&str>) -> Result<Reader, Diagnostic> {
    let install = Install::open(game, profile).map_err(|e| diagnostic(e.code(), e.message()))?;
    let tlk = match find_ci(install.root(), "dialog.tlk") {
        Some(p) => Some(Tlk::open(&p).map_err(|e| crate::inspect::diagnostic(e.code(), &e))?),
        None => None,
    };
    Ok(Reader {
        install,
        module: module.map(ModuleName::new),
        tlk,
        tables: HashMap::new(),
        blueprints: HashMap::new(),
        warnings: Vec::new(),
    })
}

/// The integer a field of any integer width holds.
fn number(s: &Struct, label: &str) -> Option<i64> {
    Some(match s.get(label)? {
        Value::Byte(v) => i64::from(*v),
        Value::Char(v) => i64::from(*v),
        Value::Word(v) => i64::from(*v),
        Value::Short(v) => i64::from(*v),
        Value::Dword(v) => i64::from(*v),
        Value::Int(v) => i64::from(*v),
        Value::Dword64(v) => i64::try_from(*v).ok()?,
        Value::Int64(v) => *v,
        _ => return None,
    })
}

/// A `CExoString`'s text, `None` when empty.
fn string(s: &Struct, label: &str) -> Option<String> {
    match s.get(label)? {
        Value::String(b) if !b.is_empty() => Some(codepage::decode(b)),
        _ => None,
    }
}

/// A `CResRef`, `None` when empty.
fn resref(s: &Struct, label: &str) -> Option<ResRef> {
    match s.get(label)? {
        Value::ResRef(r) if !r.is_empty() => Some(*r),
        _ => None,
    }
}

/// Whether a field label names an event script: BioWare's blueprints name
/// them `On…` and `Script…`.
fn is_event(label: &str) -> bool {
    label.starts_with("On") || label.starts_with("Script")
}

/// The event scripts a struct names, in field order.
fn events(s: &Struct) -> Vec<Event> {
    s.fields
        .iter()
        .filter_map(|f| {
            let label = f.label.to_string();
            match &f.value {
                Value::ResRef(r) if !r.is_empty() && is_event(&label) => Some(Event {
                    field: label,
                    script: r.to_string(),
                }),
                _ => None,
            }
        })
        .collect()
}

impl Reader {
    /// Warnings gathered since the last call: tables and blueprints the
    /// install lacks.
    pub fn take_warnings(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.warnings)
    }

    pub fn install(&self) -> &Install {
        &self.install
    }

    fn strref_text(&self, strref: u32) -> Option<String> {
        let tlk = self.tlk.as_ref()?;
        tlk.text(rakata_res::StrRef::new(strref))
            .filter(|t| !t.trim().is_empty())
            .map(str::to_owned)
    }

    fn locstring(&self, l: &LocString) -> Option<String> {
        let l = rakata_dialog::localized(l);
        let text = match &self.tlk {
            Some(t) => t.resolve(&l, t.language, Gender::Male),
            None => l.resolve_with(rakata_res::Language::English, Gender::Male, |_| None),
        };
        text.filter(|t| !t.trim().is_empty()).map(str::to_owned)
    }

    fn loc(&self, s: &Struct, label: &str) -> Option<String> {
        match s.get(label)? {
            Value::LocString(l) => self.locstring(l),
            _ => None,
        }
    }

    /// Where an install's location lies, relative to the install.
    fn relative(&self, l: &Location) -> String {
        let rel = |p: &Path| {
            p.strip_prefix(self.install.root())
                .unwrap_or(p)
                .display()
                .to_string()
        };
        match l {
            Location::File { path } => rel(path),
            Location::Capsule { path, member } => format!("{}:{member}", rel(path)),
            Location::Bif { path, index, .. } => format!("{}[{index}]", rel(path)),
        }
    }

    /// The 2DA the engine loads under this name, read once; a warning
    /// when the install lacks it or it does not read.
    fn table(&mut self, name: &str) -> Option<&TwoDa> {
        let key = name.to_ascii_lowercase();
        if !self.tables.contains_key(&key) {
            let read = ResId::parse(&format!("{key}.2da"))
                .ok()
                .map(|id| self.install.read(id, self.module.as_ref()));
            let table = match read {
                Some(Ok(Some((_, bytes)))) => match TwoDa::parse(&bytes) {
                    Ok(t) => Some(t),
                    Err(e) => {
                        self.warnings.push(diagnostic(
                            c::NO_TABLE,
                            format!(
                                "{key}.2da does not read ({e}), so its rows are shown by number"
                            ),
                        ));
                        None
                    }
                },
                _ => {
                    self.warnings.push(diagnostic(
                        c::NO_TABLE,
                        format!("no layer of the install holds {key}.2da, so its rows are shown by number"),
                    ));
                    None
                }
            };
            self.tables.insert(key.clone(), table);
        }
        self.tables.get(&key).and_then(Option::as_ref)
    }

    /// A cell, `None` when blank (`****` or empty) or absent.
    fn cell(&mut self, table: &str, row: i64, column: &str) -> Option<String> {
        let t = self.table(table)?;
        let row = usize::try_from(row).ok()?;
        let v = t.get(row, column).ok()?;
        (!v.is_empty() && v != "****").then(|| v.to_string())
    }

    /// A row of a 2DA with its label and the text its strref names.
    fn row(&mut self, table: &str, row: i64) -> Row {
        let cols = &TABLES.values;
        let found = self
            .table(table)
            .is_some_and(|t| usize::try_from(row).is_ok_and(|r| t.row_index(r).is_some()));
        let label = self.cell(table, row, &cols.label);
        let name = self
            .cell(table, row, &cols.name)
            .and_then(|s| s.parse::<u32>().ok())
            .and_then(|s| self.strref_text(s));
        Row {
            table: table.to_ascii_lowercase(),
            row,
            label,
            name,
            found,
        }
    }

    /// The blueprint the engine loads for this resref and type, read and
    /// parsed once, with where it lies.
    fn blueprint(&mut self, name: ResRef, ty: ResType) -> Option<(String, Gff)> {
        let id = ResId::new(name, ty);
        if !self.blueprints.contains_key(&id) {
            let found = match self.install.read(id, self.module.as_ref()) {
                Ok(Some((loc, bytes))) => Gff::parse(&bytes).ok().map(|g| (self.relative(&loc), g)),
                _ => None,
            };
            if found.is_none() {
                self.warnings.push(diagnostic(
                    c::NO_BLUEPRINT,
                    format!("no layer of the install holds {id}, or it does not read"),
                ));
            }
            self.blueprints.insert(id, found);
        }
        self.blueprints.get(&id).cloned().flatten()
    }

    /// An item blueprint's reading.
    pub fn item(&mut self, gff: &Gff) -> Item {
        let s = &gff.root;
        let t = &*TABLES;
        let base_item = number(s, "BaseItem").map(|b| self.row(&t.base_items.table, b));
        let mut properties = Vec::new();
        for p in s
            .get("PropertiesList")
            .and_then(Value::as_list)
            .unwrap_or_default()
        {
            properties.push(self.property(p));
        }
        let description = self
            .loc(s, "DescIdentified")
            .or_else(|| self.loc(s, "Description"));
        Item {
            resref: resref(s, "TemplateResRef").map(|r| r.to_string()),
            name: self.loc(s, "LocalizedName"),
            tag: string(s, "Tag"),
            description,
            base_item,
            properties,
            stack_size: number(s, "StackSize"),
            charges: number(s, "Charges"),
            plot: number(s, "Plot"),
        }
    }

    fn property(&mut self, p: &Struct) -> Property {
        let t = &*TABLES;
        let id = number(p, "PropertyName").unwrap_or(-1);
        let property = self.row(&t.properties.table, id);
        let subtype = self
            .cell(&t.properties.table, id, &t.properties.subtype)
            .zip(number(p, "Subtype"))
            .map(|(table, row)| self.row(&table, row));
        let value = number(p, "CostTable")
            .and_then(|ct| self.cell(&t.costs.table, ct, &t.costs.resref))
            .filter(|table| self.table(table).is_some_and(|t| !t.rows.is_empty()))
            .zip(number(p, "CostValue"))
            .map(|(table, row)| self.row(&table, row));
        let param = number(p, "Param1")
            .filter(|&n| n != i64::from(t.params.none))
            .map(|n| {
                let name = self.row(&t.params.table, n);
                let table = self
                    .cell(&t.params.table, n, &t.params.resref)
                    .unwrap_or_default();
                let v = self.row(&table, number(p, "Param1Value").unwrap_or(0));
                (name, v)
            });
        let mut text = property.shown();
        if let Some(s) = &subtype {
            let _ = write!(text, " ({})", s.shown());
        }
        if let Some(v) = &value {
            let _ = write!(text, ": {}", v.shown());
        }
        if let Some((n, v)) = &param {
            let _ = write!(text, "; {} {}", n.shown(), v.shown());
        }
        Property {
            property,
            subtype,
            value,
            param,
            text,
        }
    }

    /// The name a blueprint or instance struct gives: a creature's first
    /// and last names, else `LocName`, else `LocalizedName`.
    fn name(&self, s: &Struct) -> Option<String> {
        if let Some(first) = self.loc(s, "FirstName") {
            return Some(match self.loc(s, "LastName") {
                Some(last) => format!("{first} {last}"),
                None => first,
            });
        }
        self.loc(s, "LocName")
            .or_else(|| self.loc(s, "LocalizedName"))
    }

    /// The items an inventory names, with their blueprints' names.
    fn inventory(&mut self, s: &Struct) -> Vec<Held> {
        let mut out = Vec::new();
        for (list, field) in [
            ("Equip_ItemList", "EquippedRes"),
            ("ItemList", "InventoryRes"),
        ] {
            for e in s.get(list).and_then(Value::as_list).unwrap_or_default() {
                let Some(r) = resref(e, field) else { continue };
                let bp = self.blueprint(r, ResType::Uti);
                out.push(Held {
                    resref: r.to_string(),
                    field: field.to_string(),
                    name: bp
                        .as_ref()
                        .and_then(|(_, g)| self.loc(&g.root, "LocalizedName")),
                    tag: bp.as_ref().and_then(|(_, g)| string(&g.root, "Tag")),
                    found: bp.is_some(),
                });
            }
        }
        out
    }

    /// A creature, placeable or door blueprint's reading.
    pub fn object(&mut self, gff: &Gff, ty: ResType) -> Object {
        let s = &gff.root;
        Object {
            kind: kind_of(ty).to_string(),
            resref: resref(s, "TemplateResRef").map(|r| r.to_string()),
            name: self.name(s),
            tag: string(s, "Tag"),
            conversation: resref(s, "Conversation").map(|r| r.to_string()),
            events: events(s),
            inventory: self.inventory(s),
        }
    }

    /// An area's instances, each read with its template.
    pub fn area(&mut self, gff: &Gff) -> Area {
        let mut instances = Vec::new();
        for &(list, ty) in AREA_LISTS {
            for (n, s) in gff
                .root
                .get(list)
                .and_then(Value::as_list)
                .unwrap_or_default()
                .iter()
                .enumerate()
            {
                let template = resref(s, "TemplateResRef");
                let bp = template.and_then(|t| self.blueprint(t, ty));
                let (location, base) = match &bp {
                    Some((l, g)) => (Some(l.clone()), Some(&g.root)),
                    None => (None, None),
                };
                // The instance's own fields first, then its template's.
                let mut ev = base.map(events).unwrap_or_default();
                for e in events(s) {
                    match ev.iter_mut().find(|x| x.field == e.field) {
                        Some(x) => *x = e,
                        None => ev.push(e),
                    }
                }
                let object = Object {
                    kind: kind_of(ty).to_string(),
                    resref: template.map(|t| t.to_string()),
                    name: self.name(s).or_else(|| base.and_then(|b| self.name(b))),
                    tag: string(s, "Tag").or_else(|| base.and_then(|b| string(b, "Tag"))),
                    conversation: resref(s, "Conversation")
                        .or_else(|| base.and_then(|b| resref(b, "Conversation")))
                        .map(|r| r.to_string()),
                    events: ev,
                    inventory: match base {
                        Some(b) => self.inventory(b),
                        None => self.inventory(s),
                    },
                };
                instances.push(Instance {
                    at: format!("{list}[{n}]"),
                    template: template.map(|t| t.to_string()),
                    location,
                    object,
                });
            }
        }
        Area {
            instances,
            spawned: None,
        }
    }

    /// The objects the module's scripts create, from the index's
    /// `CreateObject` calls whose template is a literal: every script the
    /// module's capsules hold, in the tree of `conn` built from this
    /// install.
    pub fn spawned(&mut self, conn: &rakata_index::Connection) -> Result<Spawned, Diagnostic> {
        let q = |sql: &str| rakata_index::query(conn, sql).map_err(|e| crate::index::error(&e));
        let lit = |s: &str| format!("'{}'", s.replace('\'', "''"));
        let want = canonical(self.install.root());
        let trees = q("SELECT id, root FROM trees ORDER BY id")?;
        let tree = trees
            .rows
            .iter()
            .rev()
            .find(|r| {
                r[1].as_str()
                    .is_some_and(|root| canonical(Path::new(root)) == want)
            })
            .and_then(|r| r[0].as_i64())
            .ok_or_else(|| {
                diagnostic(
                    c::NO_TREE,
                    format!(
                        "the index holds no tree built from `{}`",
                        self.install.root().display()
                    ),
                )
                .help("build one with `rakata index build --game DIR --tree LABEL --db FILE`")
            })?;
        let Some(module) = self.module.clone() else {
            return Ok(Spawned {
                objects: Vec::new(),
                computed: Vec::new(),
            });
        };
        let capsules: Vec<String> = self
            .install
            .module_capsules(&module)
            .map_err(|e| diagnostic(e.code(), e.message()))?
            .iter()
            .map(|p| {
                lit(&p
                    .strip_prefix(self.install.root())
                    .unwrap_or(p)
                    .to_string_lossy()
                    .to_ascii_lowercase())
            })
            .collect();
        if capsules.is_empty() {
            return Ok(Spawned {
                objects: Vec::new(),
                computed: Vec::new(),
            });
        }
        let rows = q(&format!(
            "SELECT DISTINCT l.name, l.path, l.member, c.args FROM calls c
             JOIN locations l ON l.hash = c.hash
             WHERE l.tree = {tree} AND l.type = 'ncs' AND c.routine = 'CreateObject'
               AND lower(l.path) IN ({})
             ORDER BY l.name, c.offset",
            capsules.join(", ")
        ))?;
        let mut out: Vec<Spawn> = Vec::new();
        let mut computed: Vec<String> = Vec::new();
        for r in &rows.rows {
            let args: Vec<serde_json::Value> = r[3]
                .as_str()
                .and_then(|a| serde_json::from_str(a).ok())
                .unwrap_or_default();
            let script = r[0].as_str().unwrap_or_default().to_string();
            let Some(template) = args.get(1).and_then(|v| v.as_str()) else {
                if !computed.contains(&script) {
                    computed.push(script);
                }
                continue;
            };
            let ty = args
                .first()
                .and_then(serde_json::Value::as_i64)
                .and_then(created);
            if out
                .iter()
                .any(|s| s.script == script && s.template.eq_ignore_ascii_case(template))
            {
                continue;
            }
            let bp = match (ty, ResRef::new(template)) {
                (Some(ty), Ok(name)) => self.blueprint(name, ty),
                _ => None,
            };
            out.push(Spawn {
                script,
                location: format!(
                    "{}:{}",
                    r[1].as_str().unwrap_or_default(),
                    r[2].as_str().unwrap_or_default()
                ),
                kind: ty.map(|t| kind_of(t).to_string()),
                template: template.to_string(),
                tag: bp.as_ref().and_then(|(_, g)| string(&g.root, "Tag")),
                name: bp.as_ref().and_then(|(_, g)| self.name(&g.root)),
            });
        }
        Ok(Spawned {
            objects: out,
            computed,
        })
    }
}

fn canonical(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

/// The reading of a GFF of type `ext` (`uti`, `utc`, `utp`, `utd`, `git`),
/// `None` for any other type.
pub fn show(reader: &mut Reader, ext: &str, gff: &Gff) -> Option<Shown> {
    let ty = ResType::from_extension(ext)?;
    Some(match ty {
        ResType::Uti => Shown::Item(reader.item(gff)),
        ResType::Utc | ResType::Utp | ResType::Utd => Shown::Object(reader.object(gff, ty)),
        ResType::Git => Shown::Area(reader.area(gff)),
        _ => return None,
    })
}

/// A script decompiled.
#[derive(Debug, Clone, Serialize)]
pub struct Decompiled {
    pub script: Script,
    /// The module whose capsules were searched, named or found.
    pub module: Option<String>,
}

/// The script `name` names (`NAME.ncs`, the extension optional), the copy
/// the engine loads through `session`'s module, decompiled and verified as
/// a dialogue's conditions are. With no module set and no global layer
/// holding it, the one module whose capsules hold it is searched; several
/// are an error naming them.
pub fn decompile(session: &mut Session, name: &str) -> Result<Decompiled, Diagnostic> {
    let not_script = || {
        diagnostic(
            c::NOT_SCRIPT,
            format!("`{name}` is not a script's name, NAME.ncs"),
        )
    };
    let id = match name.rsplit_once('.') {
        Some((_, ext)) if !ext.eq_ignore_ascii_case("ncs") => return Err(not_script()),
        Some(_) => ResId::parse(name).map_err(|_| not_script())?,
        None => ResId::new(ResRef::new(name).map_err(|_| not_script())?, ResType::Ncs),
    };
    let install = session.install();
    let global = install
        .read(id, session.module())
        .map_err(|e| diagnostic(e.code(), e.message()))?;
    if global.is_none() && session.module().is_none() {
        let mut holders = Vec::new();
        for m in install.modules() {
            if install
                .module_ids(m)
                .map_err(|e| diagnostic(e.code(), e.message()))?
                .contains(&id)
            {
                holders.push(m.clone());
            }
        }
        match holders.len() {
            0 => {}
            1 => session.set_module(holders.pop()),
            _ => {
                return Err(diagnostic(
                    c::MODULES,
                    format!(
                        "{id} is in no global layer and in {} modules: {}; name one with --module",
                        holders.len(),
                        holders
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            }
        }
    }
    let script = session.script(&id.name, true);
    let module = session.module().map(ToString::to_string);
    match script.status {
        ScriptStatus::Verified | ScriptStatus::Unverified => Ok(Decompiled { script, module }),
        ScriptStatus::Missing => {
            let r = session
                .install()
                .resolve(id, session.module())
                .map_err(|e| diagnostic(e.code(), e.message()))?;
            Err(r
                .not_found()
                .map(|e| diagnostic(e.code(), e.message()))
                .unwrap_or_else(not_script))
        }
        _ => Err(diagnostic(
            c::UNRECOVERED,
            format!(
                "{id} could not be decompiled: {}",
                script.note.as_deref().unwrap_or("no reason given")
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_measured_table_names_the_tables_the_items_use() {
        let t = &*TABLES;
        assert_eq!(t.properties.table, "itempropdef");
        assert_eq!(t.costs.table, "iprp_costtable");
        assert_eq!(t.params.table, "iprp_paramtable");
        assert_eq!(t.base_items.table, "baseitems");
        assert_eq!(t.params.none, 255);
    }

    #[test]
    fn event_fields_are_the_on_and_script_labels() {
        assert!(is_event("OnUsed"));
        assert!(is_event("ScriptHeartbeat"));
        assert!(!is_event("Conversation"));
        assert!(!is_event("TemplateResRef"));
    }

    #[test]
    fn a_row_shows_its_name_else_its_label_else_its_number() {
        let mut r = Row {
            table: "iprp_damagetype".into(),
            row: 8,
            label: Some("Fire".into()),
            name: Some("Fire damage".into()),
            found: true,
        };
        assert_eq!(r.shown(), "Fire damage");
        r.name = None;
        assert_eq!(r.shown(), "Fire");
        r.label = None;
        assert_eq!(r.shown(), "row 8");
        r.found = false;
        assert_eq!(r.shown(), "iprp_damagetype has no row 8");
    }

    #[test]
    fn create_object_types_are_nwscript_constants() {
        assert_eq!(created(1), Some(ResType::Utc));
        assert_eq!(created(64), Some(ResType::Utp));
        assert_eq!(created(3), None);
    }
}
