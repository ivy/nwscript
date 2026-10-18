//! `rakata refs`: every reference to a strref, resref, tag or global, or
//! every reference a resource makes, through [`rakata::index::refs`].

use std::time::Instant;

use clap::{Args, ValueEnum};
use serde_json::json;

use super::Ctx;
use super::index::{DbArg, db_path};
use crate::Exit;
use rakata::diag::SourceMap;
use rakata::index::{self, Answer, Direction, Hit, RefKind, RefsQuery};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum KindArg {
    Strref,
    Resref,
    Tag,
    Global,
    /// Every kind THING can be.
    Auto,
}

#[derive(Args, Debug)]
#[command(after_help = "\
--to (the default) lists what refers to THING. Each hit is the resource
that holds the reference, the site in it (a GFF field path; `row N COLUMN`
of a 2DA; `strref N` of a talk table; `@OFFSET Routine.param` of a script,
with the CONSTS or CONSTI that pushes the literal), and every place the
index holds that resource, marked when an Override/ file of the same name
with other bytes shadows it. Indirect hits follow, each saying what it goes
through: a resref naming a blueprint also finds what refers to the
blueprint's tag, and a script that names THING is followed to the dialogue
links, nodes and events that run it. --direct leaves them out.

--from lists what the resource THING (name or name.ext) refers to: its
rows of the refs table, of the kind --kind names or of every kind.

--kind auto (the default) tries every kind THING can be, a strref only
when it is a number, and says which matched. An empty answer is qualified
by the index's coverage: how many resources had no reader or failed, so a
reference in one of them cannot have been found (RW1505).

The database is --db FILE, or index.sqlite under $XDG_CACHE_HOME/rakata/,
as `rakata index build` wrote it.

Examples:
  rakata refs 42724 --kind strref --db idx.sqlite
  rakata refs k_transabort --kind resref --db idx.sqlite
  rakata refs k_trans_abort --kind resref --db idx.sqlite
  rakata refs OAJ_DONE --kind global --db idx.sqlite
  rakata refs k_trg_transfail.ncs --from --db idx.sqlite
  rakata --format json refs Cand --db idx.sqlite")]
pub struct RefsArgs {
    /// The symbol: a strref, a resref, a tag or a global's name; with --from, a resource's name.
    pub thing: String,
    /// The kind of symbol THING is.
    #[arg(long, value_enum, default_value_t = KindArg::Auto)]
    pub kind: KindArg,
    /// List what refers to THING (the default).
    #[arg(long, conflicts_with = "from")]
    pub to: bool,
    /// List what the resource THING refers to.
    #[arg(long)]
    pub from: bool,
    /// Only direct references: no blueprint tags, no scripts' users.
    #[arg(long)]
    pub direct: bool,
    /// Only this tree's locations; without it, every tree the database holds.
    #[arg(long, value_name = "LABEL")]
    pub tree: Option<String>,
    #[command(flatten)]
    pub db: DbArg,
}

pub fn run(ctx: &Ctx, args: &RefsArgs) -> Exit {
    let map = SourceMap::new();
    let db = match db_path(&args.db) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let mut env = ctx.envelope("refs");
    let started = Instant::now();
    let q = RefsQuery {
        thing: &args.thing,
        kind: match args.kind {
            KindArg::Strref => Some(RefKind::StrRef),
            KindArg::Resref => Some(RefKind::ResRef),
            KindArg::Tag => Some(RefKind::Tag),
            KindArg::Global => Some(RefKind::Global),
            KindArg::Auto => None,
        },
        direction: if args.from {
            Direction::From
        } else {
            Direction::To
        },
        tree: args.tree.as_deref(),
        direct: args.direct,
    };
    let conn = match index::open(&db) {
        Ok(c) => c,
        Err(e) => return ctx.finish(env, "", vec![index::error(&e)], &map, Exit::Ok),
    };
    let answer = match index::refs(&conn, &q) {
        Ok(a) => a,
        Err(e) => return ctx.finish(env, "", vec![index::error(&e)], &map, Exit::Ok),
    };
    let mut diags = Vec::new();
    let mut coverage = None;
    if answer.hits.is_empty() {
        match index::coverage(&conn, args.tree.as_deref(), false) {
            Ok(trees) => {
                let no_reader: usize = trees
                    .iter()
                    .map(|t| t.by_status.get("no_reader").copied().unwrap_or(0))
                    .sum();
                let failed: usize = trees
                    .iter()
                    .map(|t| t.by_status.get("failed").copied().unwrap_or(0))
                    .sum();
                let mut by_type = std::collections::BTreeMap::<String, usize>::new();
                for t in &trees {
                    for (k, v) in &t.no_reader {
                        *by_type.entry(k.clone()).or_default() += v;
                    }
                }
                let mut sorted: Vec<(&String, &usize)> = by_type.iter().collect();
                sorted.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
                let types = sorted
                    .iter()
                    .map(|(k, v)| format!("{k} {v}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                diags.push(index::diagnostic(
                    index::codes::NO_REFS,
                    format!(
                        "no reference {} `{}` (tried {}); the index could not search {} resources: no reader for {no_reader} ({types}), {failed} failed",
                        if args.from { "from" } else { "to" },
                        answer.thing,
                        answer.tried.join(", "),
                        no_reader + failed,
                    ),
                ));
                coverage = Some(json!({
                    "no_reader": no_reader,
                    "failed": failed,
                    "no_reader_by_type": by_type,
                }));
            }
            Err(e) => diags.push(index::error(&e)),
        }
    }
    let seconds = started.elapsed().as_secs_f64();
    let mut text = render(&answer, ctx.out.compact);
    if let Some(c) = &coverage {
        text.push_str(&format!(
            "the index could not search {} resources: no reader for {}, {} failed\n",
            c["no_reader"].as_u64().unwrap_or(0) + c["failed"].as_u64().unwrap_or(0),
            c["no_reader"],
            c["failed"]
        ));
    }
    env.result = json!({
        "thing": answer.thing,
        "direction": answer.direction,
        "tried": answer.tried,
        "matched": answer.matched,
        "hits": answer.hits,
        "coverage": coverage,
        "seconds": seconds,
    });
    ctx.finish(env, &text, diags, &map, Exit::Ok)
}

/// Where a hit lies: each location, `path!member`, with its tree when the
/// database holds more than one and the Override file that shadows it.
fn places(h: &Hit, trees: bool) -> String {
    h.locations
        .iter()
        .map(|l| {
            let mut s = String::new();
            if trees {
                s.push_str(&format!("{}:", l.tree));
            }
            s.push_str(&l.path);
            if !l.member.is_empty() {
                s.push('!');
                s.push_str(&l.member);
            }
            if let Some(o) = &l.shadowed_by {
                s.push_str(&format!(" (shadowed by {o})"));
            }
            s
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn site(h: &Hit) -> String {
    match h.operand {
        Some(op) => format!("{} {op}", h.site),
        None => h.site.clone(),
    }
}

/// The answer as text: a heading, then the hits as columns, aligned unless
/// `compact`, the indirect ones under what they go through.
fn render(a: &Answer, compact: bool) -> String {
    let mut out = String::new();
    let direct = a.hits.iter().filter(|h| h.via.is_none()).count();
    let what = match a.direction {
        Direction::To => format!("references to `{}`", a.thing),
        Direction::From => format!("references from `{}`", a.thing),
    };
    out.push_str(&format!(
        "{what}: {direct} direct, {} indirect; tried {}; matched {}\n",
        a.hits.len() - direct,
        a.tried.join(", "),
        if a.matched.is_empty() {
            "none".to_string()
        } else {
            a.matched.join(", ")
        }
    ));
    let trees = {
        let mut t: Vec<&str> = a
            .hits
            .iter()
            .flat_map(|h| h.locations.iter().map(|l| l.tree.as_str()))
            .collect();
        t.sort_unstable();
        t.dedup();
        t.len() > 1
    };
    let rows: Vec<(Option<&str>, [String; 3])> = a
        .hits
        .iter()
        .map(|h| {
            let first = match a.direction {
                Direction::To => h.resource.clone(),
                Direction::From => format!("{} {}", h.kind, h.value),
            };
            (h.via.as_deref(), [first, site(h), places(h, trees)])
        })
        .collect();
    let mut width = [0usize; 2];
    for (_, r) in rows.iter().filter(|_| !compact) {
        for i in 0..2 {
            width[i] = width[i].max(r[i].chars().count());
        }
    }
    let mut group: Option<&str> = None;
    for (via, r) in &rows {
        if *via != group {
            if let Some(v) = via {
                out.push_str(&format!("through the {v}:\n"));
            }
            group = *via;
        }
        out.push_str(&format!(
            "  {:<w0$}  {:<w1$}  {}\n",
            r[0],
            r[1],
            r[2],
            w0 = width[0],
            w1 = width[1]
        ));
    }
    out
}
