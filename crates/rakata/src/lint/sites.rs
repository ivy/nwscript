//! The measured table of resref and strref sites, `sites.toml` beside this
//! file: which type each GFF field and 2DA column names, and which 2DA
//! columns hold strrefs. The file states its method and how to regenerate
//! it.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

use serde::Deserialize;

use super::universe::Want;

const SITES: &str = include_str!("sites.toml");

#[derive(Deserialize)]
struct Raw {
    gff: BTreeMap<String, String>,
    twoda: BTreeMap<String, String>,
    strref: RawStrref,
}

#[derive(Deserialize)]
struct RawStrref {
    columns: Vec<String>,
}

/// The table, read once.
pub(crate) struct Sites {
    /// By file extension and field label.
    gff: HashMap<(String, String), Want>,
    /// By table name and column name, both lowercase.
    twoda: HashMap<(String, String), Want>,
    strref: HashSet<(String, String)>,
}

/// The fields whose value the engine does without when nothing holds it:
/// a dialogue node's `VO_ResRef` names a recording BioWare planned for
/// every line, made for some (a node with none is shown unvoiced), and
/// `rakata dialog lint` checks voices per dialogue (RW1104).
const UNCHECKED: &[(&str, &str)] = &[("dlg", "VO_ResRef")];

fn split(key: &str) -> (String, String) {
    let (a, b) = key
        .split_once('.')
        .expect("sites.toml keys are TABLE.COLUMN");
    (a.to_string(), b.to_string())
}

fn want(t: &str) -> Want {
    Want::parse(t).unwrap_or_else(|| panic!("sites.toml names the unknown type {t}"))
}

pub(crate) static TABLE: LazyLock<Sites> = LazyLock::new(|| {
    let raw: Raw = toml::from_str(SITES).expect("sites.toml parses");
    let lower = |(a, b): (String, String)| (a.to_ascii_lowercase(), b.to_ascii_lowercase());
    Sites {
        gff: raw.gff.iter().map(|(k, v)| (split(k), want(v))).collect(),
        twoda: raw
            .twoda
            .iter()
            .map(|(k, v)| (lower(split(k)), want(v)))
            .collect(),
        strref: raw.strref.columns.iter().map(|k| lower(split(k))).collect(),
    }
});

impl Sites {
    /// What a GFF field names, `None` for a field the lint leaves alone;
    /// a field the table does not type names a resource of any type.
    pub(crate) fn gff(&self, ext: &str, label: &str) -> Option<Want> {
        if UNCHECKED.iter().any(|(e, l)| *e == ext && *l == label) {
            return None;
        }
        Some(
            self.gff
                .get(&(ext.to_string(), label.to_string()))
                .copied()
                .unwrap_or(Want::Any),
        )
    }

    /// What a 2DA column names; `None` when it holds no resrefs.
    pub(crate) fn twoda(&self, table: &str, column: &str) -> Option<Want> {
        self.twoda
            .get(&(table.to_ascii_lowercase(), column.to_ascii_lowercase()))
            .copied()
    }

    /// Whether a 2DA column holds strrefs.
    pub(crate) fn is_strref(&self, table: &str, column: &str) -> bool {
        self.strref
            .contains(&(table.to_ascii_lowercase(), column.to_ascii_lowercase()))
    }

    /// The tables either kind of column lies in, lowercase.
    pub(crate) fn tables(&self) -> Vec<String> {
        let mut t: Vec<String> = self
            .twoda
            .keys()
            .chain(self.strref.iter())
            .map(|(t, _)| t.clone())
            .collect();
        t.sort();
        t.dedup();
        t
    }

    #[cfg(test)]
    /// How many sites each kind lists: GFF fields, 2DA resref columns,
    /// 2DA strref columns.
    pub(crate) fn sizes(&self) -> (usize, usize, usize) {
        (self.gff.len(), self.twoda.len(), self.strref.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rakata_res::ResType;

    #[test]
    fn the_table_reads_and_types_the_sites_the_game_fills() {
        let t = &*TABLE;
        assert_eq!(t.gff("utc", "Conversation"), Some(Want::Type(ResType::Dlg)));
        assert_eq!(t.gff("dlg", "Script"), Some(Want::Type(ResType::Ncs)));
        assert_eq!(t.gff("git", "LinkedToModule"), Some(Want::Module));
        assert_eq!(t.gff("dlg", "VO_ResRef"), None);
        assert_eq!(t.gff("utc", "NoSuchLabel"), Some(Want::Any));
        assert_eq!(
            t.twoda("Appearance", "ModelA"),
            Some(Want::Type(ResType::Mdl))
        );
        assert_eq!(t.twoda("appearance", "texa"), None, "a texture prefix");
        assert!(t.is_strref("baseitems", "name"));
        assert!(!t.is_strref("baseitems", "itemmultiplier"));
        let (g, r, s) = t.sizes();
        assert!(g > 50 && r > 100 && s > 50, "{g} {r} {s}");
    }
}
