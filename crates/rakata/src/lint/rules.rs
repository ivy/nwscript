//! The content rules: what one resource names, checked against the
//! install. The walk and the index both reduce a resource to a
//! [`Content`], so the two answer alike.

use std::collections::HashMap;

use nwscript::ncs::facts::Literal;
use nwscript::sema::{ConstValue, EngineTable};
use rakata_res::{RefKind, ResType};

use super::sites::TABLE;
use super::universe::{Scope, Universe, Want};
use super::{Finding, codes};

/// A literal argument, as the bytecode or the index's `calls.args` holds it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Lit {
    Int(i64),
    Str(String),
    Other,
}

/// One engine call: the instruction's offset, the routine's number, and
/// each argument's literal (`None` where it is computed).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Call {
    pub offset: u32,
    pub routine: u16,
    pub args: Vec<Option<Lit>>,
}

/// What one resource names.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Content {
    /// A GFF's references: `(kind, value, site)`, resrefs lowercase and
    /// strrefs in decimal, as `rakata_gff::facts::refs` gives them.
    Gff(Vec<(RefKind, String, String)>),
    /// A table's cells that are not blank: row position, column, value.
    TwoDa(Vec<(usize, String, String)>),
    /// A script's engine calls.
    Script(Vec<Call>),
    /// A model's or a layout's references: `(value, site)`, resrefs
    /// lowercase, as `rakata_mdl::facts::refs` and `rakata_lyt::facts::refs`
    /// give them.
    Asset(Vec<(String, String)>),
}

/// One resource to check: where it lies and what it names.
pub(crate) struct Item {
    /// The path relative to the tree, `PATH!MEMBER` for an archive member.
    pub path: String,
    /// The resource's name and type extension, lowercase.
    pub name: String,
    pub ext: String,
    pub scope: Scope,
    pub content: Content,
}

/// What the rules consult.
pub(crate) struct Rules<'a> {
    pub universe: &'a Universe,
    /// The talk table's length; `None` skips the strref rule.
    pub talk_table: Option<u32>,
    /// The engine's routines and constants; `None` skips the script rule.
    pub engine: Option<&'a EngineTable>,
    /// Blueprint types by the value of each `OBJECT_TYPE_*` constant.
    pub object_types: HashMap<i64, ResType>,
}

/// The blueprint type each `OBJECT_TYPE_*` constant of `table` creates.
pub(crate) fn object_types(table: &EngineTable) -> HashMap<i64, ResType> {
    let mut out = HashMap::new();
    for c in &table.constants {
        let Some(kind) = c.name.strip_prefix("OBJECT_TYPE_") else {
            continue;
        };
        let ty = match kind {
            "CREATURE" => ResType::Utc,
            "ITEM" => ResType::Uti,
            "TRIGGER" => ResType::Utt,
            "DOOR" => ResType::Utd,
            "WAYPOINT" => ResType::Utw,
            "PLACEABLE" => ResType::Utp,
            "STORE" => ResType::Utm,
            "ENCOUNTER" => ResType::Ute,
            "SOUND" => ResType::Uts,
            _ => continue,
        };
        if let ConstValue::Int(v) = c.value {
            out.insert(i64::from(v), ty);
        }
    }
    out
}

/// What a model's or a layout's site names: a layout's rooms, tracks,
/// obstacles and hooks, a model's supermodel and a reference node's model
/// name models; a mesh's texture and lightmap and a light's flares name
/// textures.
pub(crate) fn asset_wants(ext: &str, site: &str) -> Want {
    let label = site.rsplit('.').next().unwrap_or(site);
    if ext == "mdl" && (label == "texture" || label == "lightmap" || label.starts_with("flare[")) {
        Want::Type(ResType::Tpc)
    } else {
        Want::Type(ResType::Mdl)
    }
}

impl Rules<'_> {
    /// Every finding of `item`.
    pub(crate) fn check(&self, item: &Item, out: &mut Vec<Finding>) {
        match &item.content {
            Content::Gff(refs) => self.gff(item, refs, out),
            Content::TwoDa(cells) => self.twoda(item, cells, out),
            Content::Script(calls) => self.script(item, calls, out),
            Content::Asset(refs) => {
                for (value, site) in refs {
                    let want = asset_wants(&item.ext, site);
                    self.unresolved(codes::ASSET, item, site.clone(), value, want, out);
                }
            }
        }
    }

    fn dangling(&self, item: &Item, site: String, value: &str, want: Want, out: &mut Vec<Finding>) {
        self.unresolved(codes::DANGLING, item, site, value, want, out);
    }

    fn unresolved(
        &self,
        code: &'static str,
        item: &Item,
        site: String,
        value: &str,
        want: Want,
        out: &mut Vec<Finding>,
    ) {
        let value = value.to_ascii_lowercase();
        if value.trim().is_empty() || self.universe.resolves(&value, want, &item.scope) {
            return;
        }
        out.push(Finding {
            code,
            path: item.path.clone(),
            site,
            value,
            want: Some(want.name()),
            related: Vec::new(),
        });
    }

    fn strref(&self, item: &Item, site: String, value: i64, out: &mut Vec<Finding>) {
        let Some(len) = self.talk_table else { return };
        if value == -1 || value == i64::from(u32::MAX) || value < i64::from(len) {
            return;
        }
        out.push(Finding {
            code: codes::STRREF,
            path: item.path.clone(),
            site,
            value: value.to_string(),
            want: None,
            related: Vec::new(),
        });
    }

    fn gff(&self, item: &Item, refs: &[(RefKind, String, String)], out: &mut Vec<Finding>) {
        for (kind, value, site) in refs {
            match kind {
                RefKind::ResRef => {
                    if let Some(want) = TABLE.gff(&item.ext, site) {
                        self.dangling(item, site.clone(), value, want, out);
                    }
                }
                RefKind::StrRef => {
                    if let Ok(v) = value.parse::<i64>() {
                        self.strref(item, site.clone(), v, out);
                    }
                }
                _ => {}
            }
        }
    }

    fn twoda(&self, item: &Item, cells: &[(usize, String, String)], out: &mut Vec<Finding>) {
        for (row, column, value) in cells {
            if value.is_empty() || value == "****" {
                continue;
            }
            let site = || format!("row {row}, column {column}");
            let numeric = value.parse::<f64>().is_ok();
            if let Some(want) = TABLE.twoda(&item.name, column)
                && !numeric
            {
                self.dangling(item, site(), value, want, out);
            }
            if TABLE.is_strref(&item.name, column)
                && let Ok(v) = value.parse::<i64>()
            {
                self.strref(item, site(), v, out);
            }
        }
    }

    fn script(&self, item: &Item, calls: &[Call], out: &mut Vec<Finding>) {
        let Some(table) = self.engine else { return };
        for c in calls {
            let Some(f) = table.functions.get(usize::from(c.routine)) else {
                continue;
            };
            for (k, a) in c.args.iter().enumerate() {
                let (Some(Lit::Str(s)), Some(p)) = (a, f.params.get(k)) else {
                    continue;
                };
                if rakata_index::ref_kind(&f.name, &p.name, &Literal::String(Vec::new()))
                    != Some(RefKind::ResRef)
                {
                    continue;
                }
                let want = self.param_wants(f, c, &p.name);
                let site = format!("@{} {}.{}", c.offset, f.name, p.name);
                self.dangling(item, site, s, want, out);
            }
        }
    }

    /// What a resref parameter names, by the words of its name in
    /// `nwscript.nss`: a script, a dialogue, a sound, a movie, a module, or
    /// a template, whose kind is an item's for an item template and else
    /// the one the call's literal `nObjectType` creates.
    fn param_wants(&self, f: &nwscript::sema::Function, c: &Call, param: &str) -> Want {
        let p = param.to_ascii_lowercase();
        if p.contains("script") {
            Want::Type(ResType::Ncs)
        } else if p.contains("dialog") || p.contains("conversation") {
            Want::Type(ResType::Dlg)
        } else if p.contains("sound") {
            Want::Type(ResType::Wav)
        } else if p.contains("movie") {
            Want::Type(ResType::Bik)
        } else if p.contains("module") {
            Want::Module
        } else if p.contains("template") {
            if p.contains("item") {
                return Want::Type(ResType::Uti);
            }
            let kind = f
                .params
                .iter()
                .position(|q| q.name.to_ascii_lowercase().contains("objecttype"))
                .and_then(|j| c.args.get(j))
                .and_then(|a| match a {
                    Some(Lit::Int(v)) => self.object_types.get(v).copied(),
                    _ => None,
                });
            kind.map_or(Want::Blueprint, Want::Type)
        } else {
            Want::Any
        }
    }
}
