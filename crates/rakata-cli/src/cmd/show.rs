//! `rakata show`: one GFF rendered for a reader through [`rakata::show`],
//! names and 2DA values resolved; a type it does not render prints as
//! `gff show` prints it.

use std::path::PathBuf;

use clap::Args;
use serde_json::json;

use super::Ctx;
use super::gff::{struct_json, tree, version};
use super::inspect::{self, LookupArgs, profile, resource_forms};
use crate::Exit;
use rakata::diag::SourceMap;
use rakata::gff::Gff;
use rakata::index;
use rakata::inspect::diagnostic;
use rakata::show::{self, Shown};

const HELP: &str = concat!(
    "\
What each type shows:
  .uti   the name and description through dialog.tlk, the tag, the base
         item through baseitems.2da, and each property through itempropdef,
         its subtype table, its cost table (iprp_costtable) and its
         parameter table (iprp_paramtable): `Damage Bonus (Fire): 2`, the
         rows it was read from in brackets
  .utc, .utp, .utd
         the name, tag, conversation, event scripts (the On… and Script…
         fields) and inventory, each item with its blueprint's name
  .git   each instance by list (creatures, placeables, doors, triggers,
         encounters, waypoints, sounds, stores) with its template and the
         template's tag, name, conversation, event scripts and inventory; an
         instance's own fields win over its template's. With --db, an index
         of the install (`rakata index build`), the objects the module's
         scripts create with CreateObject and a literal template, each with
         the script that creates it
Any other GFF prints its tree as `rakata gff show` does. Blueprints and
tables are looked up as the engine loads them, through --module; a module's
area needs its module. --format json gives the same, structured, with every
row's table and number.

Which 2DA tables and columns resolve a property is measured on the vanilla
tables (crates/rakata/src/show/items.toml, written by
scripts/measure-item-properties.py). A table the install lacks is RW2002; a
blueprint no layer holds, RW2003.

",
    resource_forms!(),
    "

Examples:
  rakata show k37_itm_ajunta.uti --module korr_m37aa --game DIR
  rakata show kor37_freedon.utc --module korr_m37aa --game DIR
  rakata show m37aa.git --module korr_m37aa --game DIR --db index.sqlite
  rakata --format json show ./edited.uti --game DIR"
);

#[derive(Args, Debug)]
#[command(after_help = HELP)]
pub struct ShowArgs {
    /// The GFF: NAME.EXT, PATH or ARCHIVE!MEMBER.
    #[arg(value_name = "NAME.EXT")]
    pub name: String,
    /// An index of the install, for a .git: list the objects the module's scripts create. Without --game, the root of its one tree is the install.
    #[arg(long, value_name = "FILE")]
    pub db: Option<PathBuf>,
    #[command(flatten)]
    pub lookup: LookupArgs,
}

pub fn run(ctx: &Ctx, args: &ShowArgs) -> Exit {
    let map = SourceMap::new();
    let game = match ctx.require_game("show") {
        Ok(g) => g.to_path_buf(),
        Err(e) => return e,
    };
    let mut env = ctx.envelope("show");
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let loaded = match inspect::load(&mut env, "resource", &args.name, &args.lookup.lookup(ctx)) {
        Ok(l) => l,
        Err(d) => return fail(env, d),
    };
    let gff = match Gff::parse(&loaded.bytes) {
        Ok(g) => g,
        Err(e) => {
            let d = diagnostic(e.code(), &e).note(format!("while reading `{}`", loaded.source));
            return fail(env, d);
        }
    };
    let mut reader = match show::open(
        &game,
        profile(args.lookup.profile),
        args.lookup.module.as_deref(),
    ) {
        Ok(r) => r,
        Err(d) => return fail(env, d),
    };
    let ext = loaded.extension().unwrap_or_default();
    let mut text = format!(
        "{} ({} {})\n",
        loaded.source,
        gff.file_type_str(),
        version(&gff)
    );
    let mut result = json!({
        "source": loaded.source,
        "file_type": gff.file_type_str(),
    });
    match show::show(&mut reader, &ext, &gff) {
        Some(mut shown) => {
            if let (Shown::Area(area), Some(db)) = (&mut shown, &args.db) {
                let conn = match index::open(db) {
                    Ok(c) => c,
                    Err(e) => return fail(env, index::error(&e)),
                };
                match reader.spawned(&conn) {
                    Ok(s) => area.spawned = Some(s),
                    Err(d) => return fail(env, d),
                }
            }
            text.push_str(&shown.text());
            result["shown"] = json!(shown);
        }
        None => {
            tree(&gff.root, 0, &mut text);
            result["root"] = struct_json(&gff.root);
        }
    }
    let diags = reader.take_warnings();
    env.result = result;
    ctx.finish(env, &text, diags, &map, Exit::Ok)
}
