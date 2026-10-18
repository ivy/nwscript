//! Names for numbers a save stores bare: the scripts' names for a
//! creature's local booleans and numbers, read from the game's include
//! scripts, and the few engine action ids a save has shown the meaning of.

use std::collections::BTreeMap;
use std::path::Path;

/// The scripts' names for local variable indices: `SW_FLAG_*` for the
/// booleans of `SWVarTable.BitArray`, `SW_NUMBER_*` and `WALKWAYS_*` for the
/// numbers of `SWVarTable.ByteArray`. The game defines them as `int`
/// constants in its include scripts (`k_inc_generic.nss` and its kin); this
/// table holds whatever the scripts it was given define, so none are
/// carried in this crate.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalNames {
    booleans: BTreeMap<u32, String>,
    numbers: BTreeMap<u32, String>,
}

/// The prefixes of the constants that index local booleans.
const BOOLEAN_PREFIXES: &[&str] = &["SW_FLAG_"];
/// The prefixes of the constants that index local numbers.
const NUMBER_PREFIXES: &[&str] = &["SW_NUMBER_", "WALKWAYS_"];

impl LocalNames {
    /// An empty table: every local is shown by index alone.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add the constants one script defines: each line of the form
    /// `int SW_FLAG_NAME = 62;` at the start of a line. The first
    /// definition of a name, and the first name for an index, stand.
    pub fn add_nss(&mut self, source: &str) {
        for line in source.lines() {
            let Some((name, value)) = constant(line) else {
                continue;
            };
            let table = if BOOLEAN_PREFIXES.iter().any(|p| name.starts_with(p)) {
                &mut self.booleans
            } else if NUMBER_PREFIXES.iter().any(|p| name.starts_with(p)) {
                &mut self.numbers
            } else {
                continue;
            };
            if table.values().any(|n| n == name) {
                continue;
            }
            table.entry(value).or_insert_with(|| name.to_owned());
        }
    }

    /// The constants of every include script (`k_inc_*.nss`) in `dir`, the
    /// `nss/` directory of a corpus. Fails only when `dir` cannot be listed.
    pub fn from_include_dir(dir: &Path) -> std::io::Result<Self> {
        let mut paths: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.to_ascii_lowercase())
                    .is_some_and(|n| n.starts_with("k_inc_") && n.ends_with(".nss"))
            })
            .collect();
        paths.sort();
        let mut names = Self::new();
        for p in paths {
            if let Ok(bytes) = std::fs::read(&p) {
                names.add_nss(&String::from_utf8_lossy(&bytes));
            }
        }
        Ok(names)
    }

    /// The name of local boolean `index`, if the scripts name it.
    pub fn boolean(&self, index: u32) -> Option<&str> {
        self.booleans.get(&index).map(String::as_str)
    }

    /// The name of local number `index`, if the scripts name it.
    pub fn number(&self, index: u32) -> Option<&str> {
        self.numbers.get(&index).map(String::as_str)
    }

    /// Whether the table names nothing.
    pub fn is_empty(&self) -> bool {
        self.booleans.is_empty() && self.numbers.is_empty()
    }
}

/// `int NAME = DIGITS;`, with any spacing and tabs, at the start of a line.
fn constant(line: &str) -> Option<(&str, u32)> {
    let rest = line.trim_start().strip_prefix("int")?;
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let (name, rest) = rest.trim_start().split_once('=')?;
    let name = name.trim();
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    let digits: String = rest
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let after = rest.trim_start()[digits.len()..].trim_start();
    if digits.is_empty() || !after.starts_with(';') {
        return None;
    }
    Some((name, digits.parse().ok()?))
}

/// The scripting call that queues engine action `id`, where a save has shown
/// it: `docs/formats/save.md`, "Action queues", gives the evidence for each.
/// Every other action id's meaning lives in the engine.
pub fn action_name(id: u32) -> Option<&'static str> {
    match id {
        24 => Some("ActionStartConversation"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_read_from_include_lines() {
        let mut n = LocalNames::new();
        n.add_nss(
            "int SW_FLAG_AI_OFF               = 62;\n\
             int\tSW_FLAG_AMBIENT_ANIMATIONS\t=\t29;\n\
             int SW_NUMBER_COMBO_ROUTINE = 3;\n\
             int WALKWAYS_END_POINT = 1;\n\
             int SW_FLAG_AI_OFF = 99; // a second definition loses\n\
             int nLocal = 4;\n\
             // int SW_FLAG_COMMENTED = 5;\n\
             int SW_FLAG_EXPR = 2 + 3;\n",
        );
        assert_eq!(n.boolean(62), Some("SW_FLAG_AI_OFF"));
        assert_eq!(n.boolean(29), Some("SW_FLAG_AMBIENT_ANIMATIONS"));
        assert_eq!(n.boolean(99), None);
        assert_eq!(n.boolean(5), None);
        assert_eq!(n.boolean(2), None);
        assert_eq!(n.number(3), Some("SW_NUMBER_COMBO_ROUTINE"));
        assert_eq!(n.number(1), Some("WALKWAYS_END_POINT"));
        assert_eq!(n.number(4), None);
    }
}
