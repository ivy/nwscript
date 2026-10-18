//! `namespaces.ini`: the options an installer offers, each naming its own
//! changes file, info file and data folder under `tslpatchdata`.

use crate::changes::split_numbered;
use crate::ini::{Entry, Ini};
use crate::issue::{Issue, codes};
use serde::Serialize;

/// The `[Namespaces]` list, in file order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Namespaces {
    pub options: Vec<Namespace>,
    pub unknown: Vec<Entry>,
}

/// One option: `Namespace#=SECTION` and `[SECTION]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Namespace {
    /// The number in `Namespace#`, what `--option` selects by.
    pub number: u32,
    pub section: String,
    pub name: Option<String>,
    pub description: Option<String>,
    /// `IniName`; `changes.ini` when absent.
    pub ini_name: String,
    /// `InfoName`; `info.rtf` when absent.
    pub info_name: String,
    /// `DataPath`: the folder under `tslpatchdata` holding this option's files; `tslpatchdata` itself when absent.
    pub data_path: Option<String>,
    pub line: u32,
    pub unknown: Vec<Entry>,
}

impl Namespaces {
    pub fn parse(ini: &Ini, file: usize, issues: &mut Vec<Issue>) -> Namespaces {
        let mut ns = Namespaces::default();
        let Some(list) = ini.section("Namespaces") else {
            issues.push(Issue::new(
                codes::NO_NAMESPACES,
                "namespaces.ini has no [Namespaces] section",
                file,
                0,
            ));
            return ns;
        };
        for e in &list.entries {
            let Some((prefix, number)) = split_numbered(&e.key) else {
                issues.push(Issue::new(
                    codes::UNKNOWN_KEY,
                    format!(
                        "`{}` is not a key [Namespaces] takes; kept, not applied",
                        e.key
                    ),
                    file,
                    e.line,
                ));
                ns.unknown.push(e.clone());
                continue;
            };
            if !prefix.eq_ignore_ascii_case("Namespace") {
                issues.push(Issue::new(
                    codes::UNKNOWN_KEY,
                    format!(
                        "`{}` is not a key [Namespaces] takes; kept, not applied",
                        e.key
                    ),
                    file,
                    e.line,
                ));
                ns.unknown.push(e.clone());
                continue;
            }
            let mut n = Namespace {
                number,
                section: e.value.clone(),
                name: None,
                description: None,
                ini_name: "changes.ini".to_string(),
                info_name: "info.rtf".to_string(),
                data_path: None,
                line: e.line,
                unknown: Vec::new(),
            };
            let Some(sec) = ini.section(&e.value) else {
                issues.push(Issue::new(
                    codes::MISSING_SECTION,
                    format!(
                        "`{}={}` in [Namespaces] names a section `[{}]` the file does not have; skipped",
                        e.key, e.value, e.value
                    ),
                    file,
                    e.line,
                ));
                continue;
            };
            for se in &sec.entries {
                match se.key.to_ascii_lowercase().as_str() {
                    "name" => n.name = Some(se.value.clone()),
                    "description" => n.description = Some(se.value.clone()),
                    "ininame" if !se.value.is_empty() => n.ini_name = se.value.clone(),
                    "infoname" if !se.value.is_empty() => n.info_name = se.value.clone(),
                    "ininame" | "infoname" => {}
                    "datapath" => n.data_path = Some(se.value.clone()).filter(|p| !p.is_empty()),
                    _ => {
                        issues.push(Issue::new(
                            codes::UNKNOWN_KEY,
                            format!(
                                "`{}` is not a key [{}] takes; kept, not applied",
                                se.key, sec.name
                            ),
                            file,
                            se.line,
                        ));
                        n.unknown.push(se.clone());
                    }
                }
            }
            ns.options.push(n);
        }
        if ns.options.is_empty() {
            issues.push(Issue::new(
                codes::NO_NAMESPACES,
                "namespaces.ini lists no option",
                file,
                list.line,
            ));
        }
        ns
    }

    /// The option numbered `n` in `Namespace#`.
    pub fn option(&self, n: u32) -> Option<&Namespace> {
        self.options.iter().find(|o| o.number == n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_namespace_without_ininame_or_infoname_takes_the_defaults() {
        let mut issues = Vec::new();
        let ini = Ini::parse(
            b"[Namespaces]\nNamespace1=a\nNamespace2=b\n[a]\nDataPath=a\nName=A\n[b]\nIniName=b.ini\nInfoName=\nName=B\n",
            0,
            &mut issues,
        );
        let ns = Namespaces::parse(&ini, 0, &mut issues);
        assert!(issues.is_empty(), "{issues:?}");
        let a = ns.option(1).unwrap();
        assert_eq!(
            (a.ini_name.as_str(), a.info_name.as_str()),
            ("changes.ini", "info.rtf")
        );
        assert_eq!(a.data_path.as_deref(), Some("a"));
        let b = ns.option(2).unwrap();
        assert_eq!(
            (b.ini_name.as_str(), b.info_name.as_str()),
            ("b.ini", "info.rtf")
        );
        assert_eq!(b.data_path, None);
    }

    #[test]
    fn a_missing_list_is_an_error() {
        let mut issues = Vec::new();
        let ini = Ini::parse(b"[a]\nName=A\n", 0, &mut issues);
        let ns = Namespaces::parse(&ini, 0, &mut issues);
        assert!(ns.options.is_empty());
        assert_eq!(issues[0].code, codes::NO_NAMESPACES);
    }
}
