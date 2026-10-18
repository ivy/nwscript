//! `rakata tlk show|append`: one string of a talk table, or every string
//! matching a text, and an entry appended with its strref printed, over
//! `rakata-tlk`.

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use serde_json::{Value as Json, json};

use super::Ctx;
use super::inspect;
use crate::Exit;
use rakata::diag::{Diagnostic, SourceMap};
use rakata::inspect::diagnostic;
use rakata::res::{ContentHash, ResRef, StrRef};
use rakata::tlk::{Entry, Tlk};
use rakata::{Envelope, InputRecord};

#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  rakata tlk show 42724 --game DIR
  rakata tlk show --text \"Juhani\" --game DIR
  rakata tlk append dialog.tlk --text \"Hello there.\" --sound n_juhani_01 -o dialog.tlk")]
pub struct TlkArgs {
    #[command(subcommand)]
    pub action: TlkAction,
}

#[derive(Subcommand, Debug)]
pub enum TlkAction {
    /// Print the string a strref names, or every string containing a text.
    #[command(after_help = "\
The talk table is --from FILE, else the install's dialog.tlk (--game DIR, in
any case). A strref prints its text, then `sound: RESREF` when the entry has a
voice-over. --text prints one `STRREF<TAB>text` line per entry whose text
contains PATTERN, compared without regard to case, with newlines in the text
written as \\n. --format json gives each entry's text, sound resref, sound
length and flags.

Examples:
  rakata tlk show 42724 --game DIR
  rakata --format json tlk show 42724 --game DIR
  rakata tlk show --text \"Dark Jedi\" --game DIR
  rakata tlk show 0 --from ./dialog.tlk")]
    Show {
        /// The strref: the entry's index in the table.
        #[arg(
            value_name = "STRREF",
            required_unless_present = "text",
            conflicts_with = "text"
        )]
        strref: Option<u32>,
        /// List every entry whose text contains PATTERN, in any case.
        #[arg(long, value_name = "PATTERN")]
        text: Option<String>,
        /// The talk table file, in place of the install's dialog.tlk.
        #[arg(long, value_name = "FILE")]
        from: Option<PathBuf>,
    },
    /// Append an entry to a talk table, write the table, and print the new entry's strref.
    #[command(after_help = "\
FILE is a talk table on disk. The entry is flagged as BioWare flags its own
lines (text, sound and sound length present). The strref printed is the
table's entry count before the append; in --format json it is
result.strref. The text must be in the game's code page (Windows-1252).

Examples:
  rakata tlk append ./dialog.tlk --text \"Hello there.\" -o ./dialog.tlk
  rakata tlk append ./dialog.tlk --text \"Hello.\" --sound n_juhani_01 --sound-length 1.5 -o out.tlk
  rakata --format json tlk append ./dialog.tlk --text \"Hello.\" -o out.tlk")]
    Append {
        /// The talk table to append to.
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// The entry's text.
        #[arg(long, value_name = "T")]
        text: String,
        /// The voice-over's resref.
        #[arg(long, value_name = "R")]
        sound: Option<String>,
        /// The voice-over's length in seconds.
        #[arg(long, value_name = "SECONDS", default_value_t = 0.0)]
        sound_length: f32,
        /// Where to write the table; it may be FILE.
        #[arg(short, long, value_name = "OUT")]
        output: PathBuf,
    },
}

pub fn run(ctx: &Ctx, args: &TlkArgs) -> Exit {
    match &args.action {
        TlkAction::Show { strref, text, from } => {
            show(ctx, *strref, text.as_deref(), from.as_deref())
        }
        TlkAction::Append {
            file,
            text,
            sound,
            sound_length,
            output,
        } => append(ctx, file, text, sound.as_deref(), *sound_length, output),
    }
}

/// Read the table at `path`, recorded as an input.
fn open(env: &mut Envelope, path: &Path) -> Result<Tlk, Diagnostic> {
    crate::working_on(path.display().to_string());
    let bytes = std::fs::read(path).map_err(|e| {
        Diagnostic::new(
            &rakata::codes::RE0001,
            format!("cannot read `{}`: {e}", path.display()),
        )
    })?;
    env.inputs.push(InputRecord {
        role: "tlk".to_owned(),
        path: path.display().to_string(),
        sha256: ContentHash::of(&bytes).to_hex(),
    });
    Tlk::parse(&bytes)
        .map_err(|e| diagnostic(e.code(), &e).note(format!("while reading `{}`", path.display())))
}

fn entry_json(strref: usize, e: &Entry) -> Json {
    json!({
        "strref": strref,
        "text": e.shown_text(),
        "sound": e.sound.as_ref().map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned()),
        "sound_length": e.sound_length,
        "flags": e.flags,
    })
}

fn show(ctx: &Ctx, strref: Option<u32>, pattern: Option<&str>, from: Option<&Path>) -> Exit {
    let mut env = ctx.envelope("tlk show");
    let map = SourceMap::new();
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let path = match (from, ctx.game.as_deref()) {
        (Some(f), _) => f.to_path_buf(),
        (None, Some(g)) => match rakata::inspect::dialog_tlk(g) {
            Ok(p) => p,
            Err(d) => return fail(env, d),
        },
        (None, None) => {
            eprintln!(
                "error: `rakata tlk show` needs a talk table: pass --game DIR for the install's dialog.tlk, or --from FILE"
            );
            return Exit::Usage;
        }
    };
    let tlk = match open(&mut env, &path) {
        Ok(t) => t,
        Err(d) => return fail(env, d),
    };
    if let Some(pattern) = pattern {
        let needle = pattern.to_lowercase();
        let mut lines = Vec::new();
        let mut matches = Vec::new();
        for (i, e) in tlk.entries.iter().enumerate() {
            let Some(text) = e.shown_text() else { continue };
            if text.to_lowercase().contains(&needle) {
                lines.push(format!("{i}\t{}", text.replace('\n', "\\n")));
                matches.push(entry_json(i, e));
            }
        }
        env.result = json!({
            "tlk": path.display().to_string(),
            "pattern": pattern,
            "matches": matches,
        });
        return ctx.finish(env, &lines.join("\n"), Vec::new(), &map, Exit::Ok);
    }
    let raw = strref.expect("clap requires STRREF without --text");
    let Some(entry) = tlk.get(StrRef::new(raw)) else {
        let d = Diagnostic::new(
            &rakata::codes::RE1304,
            format!(
                "the talk table holds {} entries; strref {raw} is not one of them",
                tlk.len()
            ),
        );
        return fail(env, d);
    };
    let index = StrRef::new(raw).index().expect("a found strref is set");
    let mut text = entry.shown_text().unwrap_or_default().to_owned();
    if let Some(s) = &entry.sound {
        text.push_str(&format!(
            "\nsound: {}",
            String::from_utf8_lossy(s.as_bytes())
        ));
    }
    let mut result = entry_json(index, entry);
    result["tlk"] = json!(path.display().to_string());
    env.result = result;
    ctx.finish(env, &text, Vec::new(), &map, Exit::Ok)
}

fn append(
    ctx: &Ctx,
    file: &Path,
    text: &str,
    sound: Option<&str>,
    sound_length: f32,
    output: &Path,
) -> Exit {
    let mut env = ctx.envelope("tlk append");
    let map = SourceMap::new();
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let mut tlk = match open(&mut env, file) {
        Ok(t) => t,
        Err(d) => return fail(env, d),
    };
    let mut entry = Entry::new(text);
    if let Some(s) = sound {
        match ResRef::new(s) {
            Ok(r) => entry = entry.with_sound(r, sound_length),
            Err(e) => return fail(env, diagnostic(e.code(), &e)),
        }
    } else {
        entry.sound_length = sound_length;
    }
    let strref = tlk.append(entry);
    let bytes = match tlk.to_bytes() {
        Ok(b) => b,
        Err(e) => return fail(env, diagnostic(e.code(), &e)),
    };
    if let Err(d) = inspect::write(&mut env, output, &bytes) {
        return fail(env, d);
    }
    env.result = json!({
        "strref": strref.raw(),
        "output": output.display().to_string(),
    });
    ctx.finish(env, &strref.raw().to_string(), Vec::new(), &map, Exit::Ok)
}
