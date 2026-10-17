//! `nws test` and `nws lua`: Lua tests and scripts over the library.

use super::Ctx;
use crate::Exit;
use nwscript::diag::SourceMap;
use nwscript_lua::{Host, run_lua_file, run_test_file};
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(clap::Args, Debug)]
#[command(after_help = "\
Examples:
  nws --game k1 -I corpus test                      every *_test.lua and *.snapshot.toml below .
  nws --game k1 -I corpus test docs/examples --filter keycard
  nws test --reference                              the API: worlds, objects, runs, assertions

A test file declares tests with describe/it (busted style) and runs
compiled scripts with nws.script(name):run(world). Scripts are found as
NAME.ncs in the test file's directory, then the -I directories; nwscript.nss
comes from --nwscript or -I. `nws test --reference` prints the reference;
assertions are `assert.equal`, `assert.is_true` and the others it lists,
not busted's `assert.are.*`.
Exit 0 when every test passes, 3 when any fails.")]
pub struct TestArgs {
    /// Files or directories to search for *_test.lua and *.snapshot.toml; the current directory by default.
    pub paths: Vec<PathBuf>,
    /// Run only tests whose full name contains this text.
    #[arg(long)]
    pub filter: Option<String>,
    /// Print the reference for tests and Lua scripts, and exit.
    #[arg(long)]
    pub reference: bool,
}

#[derive(clap::Args, Debug)]
#[command(after_help = "\
Example:
  nws --game k1 -I corpus lua survey.lua extra args

The script gets the same nws module as tests, with nws.args holding the
arguments after its name. nws.script(NAME) finds NAME.ncs in the Lua file's
directory, then the -I directories; nwscript.nss comes from --nwscript or
-I. `nws test --reference` documents the module.")]
pub struct LuaArgs {
    /// The Lua file to run.
    pub script: PathBuf,
    /// Arguments passed to the script as nws.args.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

fn host(ctx: &Ctx, game: nwscript::Game, base: &Path) -> Result<Host, Exit> {
    let mut resolver = ctx.resolver(Some(base.to_path_buf()));
    let mut map = SourceMap::new();
    let table = match nwscript::engine::load_table(&mut resolver, &mut map) {
        Ok(Some((t, _))) => t,
        Ok(None) => {
            eprintln!("error: Lua needs the game's nwscript.nss: pass --nwscript FILE or -I DIR");
            return Err(Exit::Usage);
        }
        Err(e) => {
            eprintln!("error: reading nwscript.nss: {e}");
            return Err(Exit::Usage);
        }
    };
    let routines = nwscript::engine::routines(&table);
    let dirs: Vec<PathBuf> = std::iter::once(base.to_path_buf())
        .chain(ctx.global.include.iter().cloned())
        .collect();
    Ok(Host {
        game: game.name().into(),
        table: Rc::new(table),
        routines: Rc::new(routines),
        find: Rc::new(move |name: &str| {
            let file = format!("{name}.ncs");
            dirs.iter()
                .find_map(|d| nwscript::resolve::find_ci(d, &file))
                .and_then(|p| std::fs::read(&p).ok().map(|b| (p.display().to_string(), b)))
        }),
    })
}

fn discover(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths.to_vec()
    };
    while let Some(p) = stack.pop() {
        if p.is_dir() {
            if let Ok(rd) = std::fs::read_dir(&p) {
                stack.extend(rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| {
                    !p.file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with('.') || n == "target")
                }));
            }
        } else if p.file_name().is_some_and(|n| {
            let n = n.to_string_lossy();
            n.ends_with("_test.lua") || n.ends_with(".snapshot.toml")
        }) {
            out.push(p);
        }
    }
    out.sort();
    out
}

pub fn test(ctx: &Ctx, args: &TestArgs) -> Exit {
    if args.reference {
        print!("{}", nwscript_lua::REFERENCE);
        return Exit::Ok;
    }
    let game = match ctx.require_game("test") {
        Ok(g) => g,
        Err(e) => return e,
    };
    let files = discover(&args.paths);
    if files.is_empty() {
        eprintln!("error: no *_test.lua or *.snapshot.toml files found");
        return Exit::Usage;
    }
    let (mut passed, mut failed) = (0, 0);
    let mut failures = Vec::new();
    let mut report = Vec::new();
    for f in &files {
        crate::working_on(f.display().to_string());
        let base = f
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .to_path_buf();
        let host = match host(ctx, game, &base) {
            Ok(h) => h,
            Err(e) => return e,
        };
        if f.to_string_lossy().ends_with(".snapshot.toml") {
            let name = f.display().to_string();
            if args.filter.as_deref().is_some_and(|x| !name.contains(x)) {
                continue;
            }
            let outcome = replay(&host, f);
            println!(
                "test {name} (snapshot) ... {}",
                if outcome.is_ok() { "ok" } else { "FAILED" }
            );
            report.push(serde_json::json!({ "file": name, "name": name, "passed": outcome.is_ok(), "message": outcome.as_ref().err() }));
            match outcome {
                Ok(()) => passed += 1,
                Err(e) => {
                    failed += 1;
                    failures.push((name, e, String::new()));
                }
            }
            continue;
        }
        match run_test_file(&host, f, args.filter.as_deref()) {
            Err(e) => {
                println!("{}: could not load: {e}", f.display());
                failed += 1;
                failures.push((f.display().to_string(), e, String::new()));
            }
            Ok(outcomes) => {
                for o in outcomes {
                    println!(
                        "test {} ... {}",
                        o.name,
                        if o.passed { "ok" } else { "FAILED" }
                    );
                    report.push(serde_json::json!({ "file": f.display().to_string(), "name": o.name, "passed": o.passed, "message": o.message }));
                    if o.passed {
                        passed += 1;
                    } else {
                        failed += 1;
                        failures.push((o.name, o.message.unwrap_or_default(), o.trace));
                    }
                }
            }
        }
    }
    if !failures.is_empty() {
        println!("\nfailures:\n");
        for (name, msg, trace) in &failures {
            println!("---- {name} ----\n{msg}");
            if !trace.is_empty() {
                println!("trace of the last run:\n{trace}");
            }
        }
    }
    println!(
        "\ntest result: {}. {passed} passed; {failed} failed",
        if failed == 0 { "ok" } else { "FAILED" }
    );
    if failed == 0 {
        Exit::Ok
    } else {
        Exit::Different
    }
}

pub fn lua(ctx: &Ctx, args: &LuaArgs) -> Exit {
    let game = match ctx.require_game("lua") {
        Ok(g) => g,
        Err(e) => return e,
    };
    crate::working_on(args.script.display().to_string());
    let base = args
        .script
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let host = match host(ctx, game, &base) {
        Ok(h) => h,
        Err(e) => return e,
    };
    match run_lua_file(&host, &args.script, &args.args) {
        Ok(()) => Exit::Ok,
        Err(e) => {
            eprintln!("error: {e}");
            Exit::Errors
        }
    }
}

/// Replay a snapshot: run its script in its world and compare.
fn replay(host: &Host, path: &Path) -> Result<(), String> {
    use nwscript::snapshot::{Snapshot, compare, execute};
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let snap: Snapshot = toml::from_str(&text).map_err(|e| format!("not a snapshot: {e}"))?;
    if snap.game != host.game {
        return Err(format!(
            "recorded with --game {}, replayed with --game {}",
            snap.game, host.game
        ));
    }
    let (found, bytes) = (host.find)(&snap.script).ok_or_else(|| {
        format!(
            "no compiled script `{}.ncs` beside the snapshot or in the -I directories",
            snap.script
        )
    })?;
    let mut map = SourceMap::new();
    let program = nwscript::ncs::decode_named(found.clone(), &bytes, &mut map)
        .map_err(|d| format!("{found}: {}", d.message))?;
    let r = execute(
        &program,
        &host.routines,
        snap.engine,
        &snap.world,
        nwscript::vm::Limits::default(),
    )?;
    match compare(&snap, &r) {
        None => Ok(()),
        Some(diff) => Err(format!(
            "{diff}\n(the script's bytecode {} the recording's)",
            if nwscript::sha256_hex(&bytes) == snap.script_sha256 {
                "matches"
            } else {
                "differs from"
            }
        )),
    }
}
