//! `rakata flags` and `rakata quest`: plot state from the index, through
//! [`rakata::plot`].

use clap::Args;
use serde_json::json;

use super::Ctx;
use super::index::{DbArg, db_path};
use crate::Exit;
use rakata::diag::SourceMap;
use rakata::index;
use rakata::plot::{self, FlagsQuery, QuestQuery};

#[derive(Args, Debug)]
#[command(after_help = "\
THING is a global when globalcat.2da declares it, otherwise an object's tag.

For an object, each local boolean and number set or read on it
(SetLocalBoolean, GetLocalBoolean, SetLocalNumber, GetLocalNumber,
directly or through an include function such as UT_SetPlotBooleanFlag that
calls one), grouped by index with the include constants naming it
(SW_PLOT_BOOLEAN_01): each write with its value, each read, the script and
offset (@N: the call, or the call into the include function), and what runs
the script (a dialogue node, a link's condition, an object's event field,
an ExecuteScript). A call names the object by tag (GetObjectByTag) or as
OBJECT_SELF, which is the object that runs the script: the creature,
placeable or door whose Conversation names the dialogue, or the blueprint
whose event field names the script. A script nothing in the index runs
leaves OBJECT_SELF unresolved; those calls are counted at the end.

For a global, each SetGlobal* and GetGlobal* call naming it.

--module limits the answer to the module's capsules (STEM.mod, STEM.rim,
STEM_s.rim): its blueprints, the templates its instances name, and the
scripts it holds or runs. Without it, the whole tree. The calls are the
index's calls and resolved_calls (docs/index.md); the database is --db
FILE, or index.sqlite under $XDG_CACHE_HOME/rakata/. RW2104 when nothing
matches.

Examples:
  rakata flags k37_inv_statue --module korr_m37aa --db idx.sqlite
  rakata flags KOR_SWORD_PLACED --db idx.sqlite
  rakata --format json flags k37_inv_statue --module korr_m37aa --db idx.sqlite")]
pub struct FlagsArgs {
    /// An object's tag, or a global's name.
    pub thing: String,
    /// The module whose capsules to read: the stem of modules/STEM.mod, STEM.rim and STEM_s.rim.
    #[arg(long, short, value_name = "STEM")]
    pub module: Option<String>,
    /// The tree of the index to read; needed when it holds several.
    #[arg(long, value_name = "LABEL")]
    pub tree: Option<String>,
    #[command(flatten)]
    pub db: DbArg,
}

#[derive(Args, Debug)]
#[command(after_help = "\
The journal category of global.jrl whose Tag is TAG: its entries in order of
id, each with its end flag and, with --text, its text from dialog.tlk, and
what writes it: dialogue nodes whose Quest and QuestEntry name it (with the
conditions of the links into the node and the globals the node's script
sets) and scripts calling AddJournalQuestEntry with literal arguments,
directly or through a function of their own (with what runs the script and
the globals it sets). Then what removes the quest (RemoveJournalQuestEntry)
and what reads it (GetJournalEntry).

The lints: RW2101 an entry nothing writes; RW2102 a write naming an entry
the category lacks; RW2103 a player reply that runs a script setting the
same global as a sibling reply (another reply the same entry offers) that
writes the quest, and writes no entry. --all lints every category and
prints only the findings.

Examples:
  rakata quest kor37_ajuntapall --db idx.sqlite
  rakata quest kor37_ajuntapall --text --db idx.sqlite
  rakata quest --all --db idx.sqlite
  rakata --format json quest kor37_ajuntapall --db idx.sqlite")]
pub struct QuestArgs {
    /// The journal category's Tag; case does not matter.
    #[arg(required_unless_present = "all")]
    pub tag: Option<String>,
    /// Every category, lints only.
    #[arg(long, conflicts_with = "tag")]
    pub all: bool,
    /// The category's and entries' text from dialog.tlk.
    #[arg(long)]
    pub text: bool,
    /// The tree of the index to read; needed when it holds several.
    #[arg(long, value_name = "LABEL")]
    pub tree: Option<String>,
    #[command(flatten)]
    pub db: DbArg,
}

pub fn flags(ctx: &Ctx, args: &FlagsArgs) -> Exit {
    let map = SourceMap::new();
    let db = match db_path(&args.db) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let mut env = ctx.envelope("flags");
    let conn = match index::open(&db) {
        Ok(c) => c,
        Err(e) => return ctx.finish(env, "", vec![index::error(&e)], &map, Exit::Ok),
    };
    let q = FlagsQuery {
        thing: &args.thing,
        module: args.module.as_deref(),
        tree: args.tree.as_deref(),
    };
    match plot::flags(&conn, &q) {
        Ok((answer, diags)) => {
            let text = answer.text();
            env.result = json!(answer);
            ctx.finish(env, &text, diags, &map, Exit::Ok)
        }
        Err(d) => ctx.finish(env, "", vec![d], &map, Exit::Ok),
    }
}

pub fn quest(ctx: &Ctx, args: &QuestArgs) -> Exit {
    let map = SourceMap::new();
    let db = match db_path(&args.db) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let mut env = ctx.envelope("quest");
    let conn = match index::open(&db) {
        Ok(c) => c,
        Err(e) => return ctx.finish(env, "", vec![index::error(&e)], &map, Exit::Ok),
    };
    let q = QuestQuery {
        tag: args.tag.as_deref(),
        tree: args.tree.as_deref(),
        text: args.text,
    };
    match plot::quest(&conn, &q) {
        Ok((answer, diags)) => {
            let text = answer.text(args.all);
            env.result = if args.all {
                json!({
                    "scope": answer.scope,
                    "journal": answer.journal,
                    "categories": answer.quests.len(),
                })
            } else {
                json!(answer)
            };
            ctx.finish(env, &text, diags, &map, Exit::Ok)
        }
        Err(d) => ctx.finish(env, "", vec![d], &map, Exit::Ok),
    }
}
