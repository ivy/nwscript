//! The known list: findings a tree is accepted with, by code, path and
//! site, so only a new one fails the run.
//!
//! ```toml
//! [RW1901]
//! "modules/danm13.rim!module.ifo" = ["Mod_OnPlrDying", "Mod_OnSpawnBtnDn"]
//! ```
//!
//! Paths compare without regard to case; sites as written.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;

use super::Finding;

/// A known list, read.
#[derive(Debug, Clone, Default)]
pub struct Known {
    /// By code, then lowercased path: the sites.
    entries: HashMap<String, HashMap<String, HashSet<String>>>,
}

impl Known {
    /// Read a known list's text; the error says what does not parse.
    pub fn parse(text: &str) -> Result<Known, String> {
        let raw: BTreeMap<String, BTreeMap<String, Vec<String>>> =
            toml::from_str(text).map_err(|e| e.to_string())?;
        let mut entries: HashMap<String, HashMap<String, HashSet<String>>> = HashMap::new();
        for (code, paths) in raw {
            let by_path = entries.entry(code.to_ascii_uppercase()).or_default();
            for (path, sites) in paths {
                by_path
                    .entry(path.to_ascii_lowercase())
                    .or_default()
                    .extend(sites);
            }
        }
        Ok(Known { entries })
    }

    /// Whether the list holds `f`.
    pub fn contains(&self, f: &Finding) -> bool {
        self.entries
            .get(f.code)
            .and_then(|p| p.get(&f.path.to_ascii_lowercase()))
            .is_some_and(|s| s.contains(&f.site))
    }

    /// The entries listed, by code.
    pub fn len(&self) -> usize {
        self.entries
            .values()
            .flat_map(|p| p.values())
            .map(HashSet::len)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `findings` as a known list, under `header` (comment lines).
    pub fn render(header: &str, findings: &[Finding]) -> String {
        let mut by: BTreeMap<&str, BTreeMap<&str, Vec<&str>>> = BTreeMap::new();
        for f in findings {
            by.entry(f.code)
                .or_default()
                .entry(&f.path)
                .or_default()
                .push(&f.site);
        }
        let mut out = String::new();
        for line in header.lines() {
            let _ = writeln!(out, "# {line}");
        }
        for (code, paths) in by {
            let _ = write!(out, "\n[{code}]\n");
            for (path, mut sites) in paths {
                sites.sort_unstable();
                sites.dedup();
                let list: Vec<String> = sites.iter().map(|s| quote(s)).collect();
                let _ = writeln!(out, "{} = [{}]", quote(path), list.join(", "));
            }
        }
        out
    }
}

/// A TOML basic string.
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(code: &'static str, path: &str, site: &str) -> Finding {
        Finding {
            code,
            path: path.into(),
            site: site.into(),
            value: String::new(),
            want: None,
            related: Vec::new(),
        }
    }

    #[test]
    fn a_rendered_list_reads_back_and_matches_paths_in_any_case() {
        let all = [
            f("RW1901", "modules/danm13.rim!module.ifo", "Mod_OnPlrDying"),
            f(
                "RW1901",
                "modules/danm13.rim!module.ifo",
                "Mod_OnSpawnBtnDn",
            ),
            f("RW1903", "Override/Foo.utc", ""),
        ];
        let text = Known::render("the vanilla game's own", &all);
        assert!(text.starts_with("# the vanilla game's own\n"), "{text}");
        let k = Known::parse(&text).unwrap();
        assert_eq!(k.len(), 3);
        assert!(k.contains(&f(
            "RW1901",
            "MODULES/danm13.rim!module.ifo",
            "Mod_OnPlrDying"
        )));
        assert!(!k.contains(&f(
            "RW1901",
            "modules/danm13.rim!module.ifo",
            "Mod_OnPlrDeath"
        )));
        assert!(!k.contains(&f("RW1902", "Override/Foo.utc", "")));
        assert!(Known::parse("[RW1901]\nx = 3").is_err());
    }
}
