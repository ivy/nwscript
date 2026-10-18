//! `refs`: every naming of a symbol, or every symbol a resource names, read
//! from the `refs` table, with where the game loads each resource from.
//!
//! Toward a symbol, the answer has the direct references and two kinds of
//! indirect one, each marked with what it goes through: a blueprint named
//! by a resref is referred to by its tag too (a script finds a placeable
//! with `GetNearestObjectByTag`, never by its resref), and a script that
//! names a symbol is run by the dialogue links, nodes and events that name
//! the script. A weaker match follows them: a script that holds the symbol
//! as a string constant (`script_strings`) when no `refs` row of that
//! script names it, as when the string reaches the call through a
//! variable.

use std::collections::{BTreeSet, HashMap};

use rakata_res::RefKind;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::error::{Error, Result};
use crate::store;

/// Which way a [`refs`] query reads the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// What refers to the symbol.
    To,
    /// What the resource refers to.
    From,
}

/// A [`refs`] query.
#[derive(Debug, Clone)]
pub struct RefsQuery<'a> {
    /// The symbol, or with [`Direction::From`] the resource (`name` or
    /// `name.ext`).
    pub thing: &'a str,
    /// The kind to look for; `None` tries every kind the symbol can be.
    pub kind: Option<RefKind>,
    pub direction: Direction,
    /// Only this tree's locations; `None` for every tree.
    pub tree: Option<&'a str>,
    /// Only direct references: no blueprint tags, no scripts' users.
    pub direct: bool,
}

/// One place a resource lies, and whether the game loads that copy.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Location {
    pub tree: String,
    pub path: String,
    /// The archive member, empty for a loose file.
    pub member: String,
    /// The copy the game loads instead: an `Override/` file of the same
    /// name and type holding other bytes, or for a member of a module's
    /// `STEM.rim` or `STEM_s.rim`, the `STEM.mod` that replaces both.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadowed_by: Option<String>,
}

/// One reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hit {
    /// The kind and symbol of the `refs` row.
    pub kind: String,
    pub value: String,
    /// The resource that holds the reference, `name.type`.
    pub resource: String,
    pub hash: String,
    /// The site as the `refs` table holds it: a field path, `row N COLUMN`,
    /// `strref N`, or `@OFFSET Routine.param`.
    pub site: String,
    /// For a script, the instruction that pushes the literal: `CONSTS` or
    /// `CONSTI`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operand: Option<&'static str>,
    /// For an indirect reference, what it goes through: `tag NAME of
    /// RESOURCE` or `script NAME`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
    /// For a weaker match, a script holding the symbol as a string
    /// constant that no resolved call names: how many `CONSTS` push it.
    /// The kind is `string` and the site the instructions' offsets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unresolved: Option<usize>,
    pub locations: Vec<Location>,
}

/// The answer to a [`refs`] query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Answer {
    pub thing: String,
    pub direction: Direction,
    /// The kinds looked for, and those with at least one direct hit.
    pub tried: Vec<String>,
    pub matched: Vec<String>,
    pub hits: Vec<Hit>,
}

/// The blueprint types: a resref naming one of these names an object that
/// bears a tag.
const BLUEPRINTS: &[&str] = &[
    "utc", "utd", "ute", "uti", "utm", "utp", "uts", "utt", "utw",
];

/// Whether a field or parameter names a script that runs: a dialogue's
/// `Script` and `Active`, an object's `OnUsed` and `ScriptOnEnter`, a
/// module's `Mod_OnHeartbeat`, `ExecuteScript`'s `sScript`.
fn runs_script(site: &str) -> bool {
    let last = site.rsplit(['.', ' ']).next().unwrap_or(site);
    let l = last.to_ascii_lowercase();
    l.contains("script") || l.starts_with("on") || l.contains("_on") || l.starts_with("active")
}

/// Whether a site is an object's own `Tag` field: the object bearing the
/// tag, not a reference to it.
fn is_tag_field(site: &str) -> bool {
    site.rsplit('.').next() == Some("Tag")
}

/// The kinds `thing` can be, in [`RefKind::ALL`]'s order.
fn kinds_for(thing: &str) -> Vec<RefKind> {
    RefKind::ALL
        .into_iter()
        .filter(|k| valid(*k, thing).is_ok())
        .collect()
}

/// `thing` as a value of `kind`, or why it cannot be one.
fn valid(kind: RefKind, thing: &str) -> std::result::Result<String, &'static str> {
    match kind {
        RefKind::StrRef => thing
            .parse::<u32>()
            .map(|n| n.to_string())
            .map_err(|_| "a strref is a decimal number"),
        RefKind::ResRef => {
            let stem = stem(thing).0;
            if stem.is_empty() || stem.len() > 16 {
                Err("a resref is one to sixteen characters")
            } else {
                Ok(stem.to_ascii_lowercase())
            }
        }
        RefKind::Tag | RefKind::Global => {
            if thing.is_empty() {
                Err("the symbol is empty")
            } else {
                Ok(thing.to_string())
            }
        }
    }
}

/// `name.ext` as the name and the extension, lowercase.
fn stem(thing: &str) -> (String, Option<String>) {
    match thing.rsplit_once('.') {
        Some((n, e)) if !n.is_empty() && !e.is_empty() => {
            (n.to_ascii_lowercase(), Some(e.to_ascii_lowercase()))
        }
        _ => (thing.to_ascii_lowercase(), None),
    }
}

/// The reader: the connection, the tree it is limited to, and the
/// locations already looked up.
struct Reader<'c> {
    conn: &'c Connection,
    tree: Option<i64>,
    located: HashMap<String, Vec<(String, Location)>>,
}

const FAIL: &str = "reading references";

impl Reader<'_> {
    /// Every location of `hash`, with the resource name each has there.
    fn locations(&mut self, hash: &str) -> Result<Vec<(String, Location)>> {
        if let Some(l) = self.located.get(hash) {
            return Ok(l.clone());
        }
        let mut st = self
            .conn
            .prepare_cached(
                "SELECT l.tree, t.label, l.path, l.member, l.name, l.type
                 FROM locations l JOIN trees t ON t.id = l.tree
                 WHERE l.hash = ?1 AND (?2 IS NULL OR l.tree = ?2)
                 ORDER BY l.tree, l.path, l.member",
            )
            .map_err(Error::db(FAIL))?;
        let rows: Vec<(i64, String, String, String, String, String)> = st
            .query_map(params![hash, self.tree], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })
            .map_err(Error::db(FAIL))?
            .collect::<rusqlite::Result<_>>()
            .map_err(Error::db(FAIL))?;
        let mut out = Vec::with_capacity(rows.len());
        for (tree, label, path, member, name, ty) in rows {
            let shadowed_by = if is_override(&path) {
                None
            } else {
                match self.override_copy(tree, &name, &ty, hash)? {
                    Some(o) => Some(o),
                    None if member.is_empty() => None,
                    None => self.module_mod(tree, &path)?,
                }
            };
            out.push((
                format!("{name}.{ty}"),
                Location {
                    tree: label,
                    path,
                    member,
                    shadowed_by,
                },
            ));
        }
        self.located.insert(hash.to_string(), out.clone());
        Ok(out)
    }

    /// The `Override/` file of tree `tree` named `name.ty` whose content is
    /// not `hash`.
    fn override_copy(&self, tree: i64, name: &str, ty: &str, hash: &str) -> Result<Option<String>> {
        let mut st = self
            .conn
            .prepare_cached(
                "SELECT path FROM locations
                 WHERE name = ?1 AND type = ?2 AND tree = ?3 AND member = ''
                   AND lower(substr(path, 1, 9)) = 'override/' AND hash <> ?4
                 ORDER BY path LIMIT 1",
            )
            .map_err(Error::db(FAIL))?;
        st.query_row(params![name, ty, tree, hash], |r| r.get(0))
            .optional()
            .map_err(Error::db(FAIL))
    }

    /// For `modules/STEM.rim` or `modules/STEM_s.rim`, the tree's
    /// `modules/STEM.mod`, which the game loads in place of both.
    fn module_mod(&self, tree: i64, path: &str) -> Result<Option<String>> {
        let lower = path.to_ascii_lowercase();
        let Some(file) = lower.strip_prefix("modules/") else {
            return Ok(None);
        };
        let Some(stem) = file
            .strip_suffix("_s.rim")
            .or_else(|| file.strip_suffix(".rim"))
        else {
            return Ok(None);
        };
        let mut st = self
            .conn
            .prepare_cached(
                "SELECT path FROM locations
                 WHERE name = ?1 AND type = 'mod' AND tree = ?2 AND member = ''
                   AND lower(path) = ?3 LIMIT 1",
            )
            .map_err(Error::db(FAIL))?;
        st.query_row(params![stem, tree, format!("modules/{stem}.mod")], |r| {
            r.get(0)
        })
        .optional()
        .map_err(Error::db(FAIL))
    }

    /// The `refs` rows of `kind` naming `value`.
    fn naming(&self, kind: RefKind, value: &str) -> Result<Vec<(String, String, String)>> {
        let mut st = self
            .conn
            .prepare_cached(
                "SELECT from_hash, value, site FROM refs WHERE kind = ?1 AND value = ?2
                 ORDER BY from_hash, site",
            )
            .map_err(Error::db(FAIL))?;
        st.query_map(params![kind.name(), value], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(Error::db(FAIL))?
        .collect::<rusqlite::Result<_>>()
        .map_err(Error::db(FAIL))
    }

    /// One hit per resource name the content `hash` has in the tree.
    fn hits(
        &mut self,
        hash: &str,
        kind: &str,
        value: &str,
        site: &str,
        via: Option<&str>,
    ) -> Result<Vec<Hit>> {
        let mut by_name: Vec<(String, Vec<Location>)> = Vec::new();
        for (name, loc) in self.locations(hash)? {
            match by_name.iter_mut().find(|(n, _)| *n == name) {
                Some((_, v)) => v.push(loc),
                None => by_name.push((name, vec![loc])),
            }
        }
        Ok(by_name
            .into_iter()
            .map(|(resource, locations)| Hit {
                kind: kind.to_string(),
                value: value.to_string(),
                operand: if resource.ends_with(".ncs") && site.starts_with('@') {
                    Some(if kind == RefKind::StrRef.name() {
                        "CONSTI"
                    } else {
                        "CONSTS"
                    })
                } else {
                    None
                },
                resource,
                hash: hash.to_string(),
                site: site.to_string(),
                via: via.map(str::to_string),
                unresolved: None,
                locations,
            })
            .collect())
    }

    /// The scripts holding `value` as a string constant, each with the
    /// offsets of the `CONSTS` instructions that push it.
    fn constants(&self, value: &str) -> Result<Vec<(String, Vec<i64>)>> {
        let mut st = self
            .conn
            .prepare_cached(
                "SELECT hash, offset FROM script_strings WHERE value = ?1 ORDER BY hash, offset",
            )
            .map_err(Error::db(FAIL))?;
        let rows: Vec<(String, i64)> = st
            .query_map(params![value], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(Error::db(FAIL))?
            .collect::<rusqlite::Result<_>>()
            .map_err(Error::db(FAIL))?;
        let mut out: Vec<(String, Vec<i64>)> = Vec::new();
        for (hash, offset) in rows {
            match out.last_mut() {
                Some((h, v)) if *h == hash => v.push(offset),
                _ => out.push((hash, vec![offset])),
            }
        }
        Ok(out)
    }

    /// The tags of the blueprints named `name`, with the blueprint each
    /// belongs to and its hash.
    fn blueprint_tags(&self, name: &str) -> Result<Vec<(String, String, String)>> {
        let types = BLUEPRINTS
            .iter()
            .map(|t| format!("'{t}'"))
            .collect::<Vec<_>>()
            .join(",");
        let mut st = self
            .conn
            .prepare_cached(&format!(
                "SELECT DISTINCT g.value, l.name || '.' || l.type, l.hash
                 FROM locations l JOIN gff_fields g ON g.hash = l.hash
                 WHERE l.name = ?1 AND l.type IN ({types}) AND (?2 IS NULL OR l.tree = ?2)
                   AND g.path = 'Tag' AND g.value <> ''
                 ORDER BY 2, 1"
            ))
            .map_err(Error::db(FAIL))?;
        st.query_map(params![name, self.tree], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(Error::db(FAIL))?
        .collect::<rusqlite::Result<_>>()
        .map_err(Error::db(FAIL))
    }
}

fn is_override(path: &str) -> bool {
    path.len() >= 9 && path[..9].eq_ignore_ascii_case("override/")
}

/// The answer to `q` over the index `conn`.
pub fn refs(conn: &Connection, q: &RefsQuery) -> Result<Answer> {
    let tree = match q.tree {
        Some(t) => Some(store::tree_id(conn, t)?),
        None => None,
    };
    let kinds = match q.kind {
        Some(k) => {
            valid(k, q.thing).map_err(|why| Error::Symbol {
                thing: q.thing.to_string(),
                kind: k.name(),
                why,
            })?;
            vec![k]
        }
        None => kinds_for(q.thing),
    };
    let mut r = Reader {
        conn,
        tree,
        located: HashMap::new(),
    };
    let mut answer = Answer {
        thing: q.thing.to_string(),
        direction: q.direction,
        tried: kinds.iter().map(|k| k.name().to_string()).collect(),
        matched: Vec::new(),
        hits: Vec::new(),
    };
    match q.direction {
        Direction::To => to(&mut r, q, &kinds, &mut answer)?,
        Direction::From => from(&mut r, q, &kinds, &mut answer)?,
    }
    Ok(answer)
}

fn to(r: &mut Reader, q: &RefsQuery, kinds: &[RefKind], answer: &mut Answer) -> Result<()> {
    let mut direct = Vec::new();
    let mut naming = BTreeSet::new();
    for &kind in kinds {
        let value = valid(kind, q.thing).expect("the kinds are valid for the symbol");
        let before = direct.len();
        for (hash, v, site) in r.naming(kind, &value)? {
            direct.extend(r.hits(&hash, kind.name(), &v, &site, None)?);
            naming.insert(hash);
        }
        if direct.len() > before {
            answer.matched.push(kind.name().to_string());
        }
    }
    let mut indirect = Vec::new();
    if !q.direct {
        // A blueprint is found by its tag.
        if kinds.contains(&RefKind::ResRef) {
            let name = valid(RefKind::ResRef, q.thing).expect("valid");
            let tags = r.blueprint_tags(&name)?;
            let own: BTreeSet<&str> = tags.iter().map(|(_, _, h)| h.as_str()).collect();
            let mut seen = BTreeSet::new();
            for (tag, blueprint, _) in &tags {
                if !seen.insert(tag.to_ascii_lowercase()) {
                    continue;
                }
                let via = format!("tag {tag} of {blueprint}");
                for (hash, v, site) in r.naming(RefKind::Tag, tag)? {
                    if own.contains(hash.as_str()) || is_tag_field(&site) {
                        continue;
                    }
                    indirect.extend(r.hits(&hash, RefKind::Tag.name(), &v, &site, Some(&via))?);
                }
            }
        }
        // A script runs where something names it.
        let scripts: BTreeSet<String> = direct
            .iter()
            .chain(&indirect)
            .filter_map(|h| h.resource.strip_suffix(".ncs").map(str::to_string))
            .collect();
        for script in scripts {
            let via = format!("script {script}");
            for (hash, v, site) in r.naming(RefKind::ResRef, &script)? {
                if runs_script(&site) {
                    indirect.extend(r.hits(
                        &hash,
                        RefKind::ResRef.name(),
                        &v,
                        &site,
                        Some(&via),
                    )?);
                }
            }
        }
    }
    // A string constant no resolved call names: the weaker match.
    let mut weak = Vec::new();
    if kinds.iter().any(|k| *k != RefKind::StrRef) {
        for (hash, offsets) in r.constants(q.thing)? {
            if naming.contains(&hash) {
                continue;
            }
            let site = offsets
                .iter()
                .map(|o| format!("@{o}"))
                .collect::<Vec<_>>()
                .join(" ");
            for mut h in r.hits(&hash, "string", q.thing, &site, None)? {
                h.unresolved = Some(offsets.len());
                weak.push(h);
            }
        }
    }
    sort(&mut direct);
    sort(&mut indirect);
    sort(&mut weak);
    answer.hits = direct;
    answer.hits.extend(indirect);
    answer.hits.extend(weak);
    Ok(())
}

fn from(r: &mut Reader, q: &RefsQuery, kinds: &[RefKind], answer: &mut Answer) -> Result<()> {
    let (name, ty) = stem(q.thing);
    let mut st = r
        .conn
        .prepare(
            "SELECT DISTINCT hash FROM locations
             WHERE name = ?1 AND (?2 IS NULL OR type = ?2) AND (?3 IS NULL OR tree = ?3)
             ORDER BY hash",
        )
        .map_err(Error::db(FAIL))?;
    let hashes: Vec<String> = st
        .query_map(params![name, ty, r.tree], |row| row.get(0))
        .map_err(Error::db(FAIL))?
        .collect::<rusqlite::Result<_>>()
        .map_err(Error::db(FAIL))?;
    if hashes.is_empty() {
        return Err(Error::NoSuchResource {
            name: q.thing.to_string(),
        });
    }
    // Every kind, unless one was asked for.
    let kinds: Vec<RefKind> = if q.kind.is_some() {
        kinds.to_vec()
    } else {
        RefKind::ALL.to_vec()
    };
    answer.tried = kinds.iter().map(|k| k.name().to_string()).collect();
    let mut st = r
        .conn
        .prepare("SELECT kind, value, site FROM refs WHERE from_hash = ?1 ORDER BY rowid")
        .map_err(Error::db(FAIL))?;
    let mut hits = Vec::new();
    let mut matched = BTreeSet::new();
    for hash in &hashes {
        let rows: Vec<(String, String, String)> = st
            .query_map([hash], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(Error::db(FAIL))?
            .collect::<rusqlite::Result<_>>()
            .map_err(Error::db(FAIL))?;
        for (kind, value, site) in rows {
            if !kinds.iter().any(|k| k.name() == kind) {
                continue;
            }
            let found = r.hits(hash, &kind, &value, &site, None)?;
            // Only the names asked for: a content may lie under other names.
            for h in found {
                let (n, t) = stem(&h.resource);
                if n == name && ty.as_ref().is_none_or(|ty| t.as_ref() == Some(ty)) {
                    matched.insert(kind.clone());
                    hits.push(h);
                }
            }
        }
    }
    answer.matched = RefKind::ALL
        .iter()
        .map(|k| k.name().to_string())
        .filter(|k| matched.contains(k))
        .collect();
    answer.hits = hits;
    Ok(())
}

fn sort(hits: &mut [Hit]) {
    hits.sort_by(|a, b| {
        (&a.via, &a.resource, &a.locations, &a.site).cmp(&(
            &b.via,
            &b.resource,
            &b.locations,
            &b.site,
        ))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_symbol_is_tried_as_every_kind_it_can_be() {
        assert_eq!(
            kinds_for("42724"),
            [
                RefKind::ResRef,
                RefKind::StrRef,
                RefKind::Tag,
                RefKind::Global
            ]
        );
        assert_eq!(
            kinds_for("OAJ_DONE"),
            [RefKind::ResRef, RefKind::Tag, RefKind::Global]
        );
        assert_eq!(
            kinds_for("a_name_longer_than_sixteen"),
            [RefKind::Tag, RefKind::Global]
        );
        assert_eq!(
            valid(RefKind::ResRef, "K_Trans_Abort.utp"),
            Ok("k_trans_abort".into())
        );
        assert!(valid(RefKind::StrRef, "-1").is_err());
    }

    #[test]
    fn script_sites_and_tag_fields() {
        assert!(runs_script("EntryList[3].RepliesList[0].Active"));
        assert!(runs_script("ReplyList[0].Script"));
        assert!(runs_script("OnUsed"));
        assert!(runs_script("ScriptOnEnter"));
        assert!(runs_script("Mod_OnHeartbeat"));
        assert!(runs_script("@4793 ExecuteScript.sScript"));
        assert!(!runs_script("Placeable List[3].TemplateResRef"));
        assert!(!runs_script("Conversation"));
        assert!(is_tag_field("Placeable List[3].Tag"));
        assert!(is_tag_field("Tag"));
        assert!(!is_tag_field("@136 GetNearestObjectByTag.sTag"));
    }
}
