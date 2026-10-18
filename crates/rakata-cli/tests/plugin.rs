//! The plugin's skills defer to `--help` and to the Python module's stubs:
//! every `rakata` command and subcommand they name exists, every flag they
//! use is in that command's help or the global options, and every name of
//! the module the data skill cites is in the stubs.

use assert_cmd::Command;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            files(&p, ext, out);
        } else if p.extension().is_some_and(|x| x == ext) {
            out.push(p);
        }
    }
}

fn markdown(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    files(dir, "md", &mut out);
    assert!(!out.is_empty(), "no markdown under {}", dir.display());
    out.into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).unwrap();
            (p, text)
        })
        .collect()
}

fn help(args: &[&str]) -> String {
    let out = Command::cargo_bin("rakata")
        .unwrap()
        .args(args)
        .arg("--help")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "rakata {} --help failed",
        args.join(" ")
    );
    String::from_utf8(out.stdout).unwrap()
}

/// The names in a help text's `Commands:` section.
fn commands(help: &str) -> BTreeSet<String> {
    let Some(section) = help.split("Commands:").nth(1) else {
        return BTreeSet::new();
    };
    section
        .split("Options:")
        .next()
        .unwrap()
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .filter(|w| w != "help")
        .collect()
}

/// A fenced block or an inline code span of a markdown text.
struct Code {
    text: String,
    python: bool,
    inline: bool,
}

/// The code in a markdown text: each line of a fenced block, and each
/// inline code span elsewhere.
fn code(text: &str) -> Vec<Code> {
    let mut out = Vec::new();
    let mut fence: Option<String> = None;
    for line in text.lines() {
        let t = line.trim_start();
        if let Some(info) = t.strip_prefix("```") {
            fence = match fence {
                Some(_) => None,
                None => Some(info.trim().to_string()),
            };
        } else if let Some(lang) = &fence {
            out.push(Code {
                text: line.to_string(),
                python: lang == "python",
                inline: false,
            });
        } else {
            out.extend(line.split('`').skip(1).step_by(2).map(|s| Code {
                text: s.to_string(),
                python: false,
                inline: true,
            }));
        }
    }
    out
}

/// Options that take a value, so the word after one is no command.
fn takes_value(flag: &str) -> bool {
    matches!(
        flag,
        "--game"
            | "--format"
            | "--color"
            | "--module"
            | "-m"
            | "--db"
            | "--tree"
            | "--kind"
            | "--known"
            | "--write-known"
            | "--paths"
            | "--profile"
            | "--only"
            | "--from"
            | "-o"
            | "--output"
            | "--path"
            | "--row"
            | "--column"
            | "--text"
            | "--parent"
            | "--changed"
            | "--threads"
            | "--option"
    )
}

/// The plain words of an invocation, skipping options and their values.
fn plain(words: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if w.starts_with('-') || (i > 0 && takes_value(&words[i - 1])) {
            continue;
        }
        out.push(w.trim_end_matches(['.', ',', ':', ')', ';']).to_string());
    }
    out
}

fn placeholder(w: &str) -> bool {
    w.is_empty() || w.starts_with('<') || w.chars().any(|c| c.is_ascii_uppercase())
}

#[test]
fn every_rakata_command_and_flag_the_plugin_names_exists() {
    let global = help(&[]);
    let top = commands(&global);
    assert!(top.contains("dialog") && top.contains("diff"));
    let mut checked = 0;
    let mut skill = 0;
    for (f, text) in markdown(&root().join("plugin")) {
        for c in code(&text) {
            let Some(rest) = c.text.trim_start().strip_prefix("rakata ") else {
                continue;
            };
            let words: Vec<String> = rest.split_whitespace().map(str::to_string).collect();
            let plain = plain(&words);
            let Some(cmd) = plain.first().filter(|w| !placeholder(w)) else {
                continue;
            };
            assert!(
                top.contains(cmd),
                "{}: `rakata {rest}` names no command of `rakata --help`",
                f.display()
            );
            let mut path = vec![cmd.as_str()];
            let mut own = help(&path);
            let subs = commands(&own);
            if !subs.is_empty() {
                let sub = plain.get(1).map(String::as_str).unwrap_or("");
                if placeholder(sub) {
                    continue;
                }
                assert!(
                    subs.contains(sub),
                    "{}: `rakata {rest}` names no subcommand of `rakata {cmd} --help`",
                    f.display()
                );
                path.push(sub);
                own = help(&path);
            }
            for w in words.iter().filter(|w| w.starts_with('-')) {
                let flag = w
                    .split('=')
                    .next()
                    .unwrap()
                    .trim_end_matches(['.', ',', ')']);
                assert!(
                    own.contains(flag) || global.contains(flag),
                    "{}: `{flag}` is not in `rakata {} --help`",
                    f.display(),
                    path.join(" ")
                );
            }
            checked += 1;
            if f.components().any(|p| p.as_os_str() == "rakata") {
                skill += 1;
            }
        }
    }
    assert!(
        skill > 20,
        "only {skill} invocations found in the rakata skill"
    );
    assert!(checked >= skill);
}

/// Every name a stub defines: modules, classes, functions, attributes,
/// parameters and constants.
fn stub_names() -> BTreeSet<String> {
    let dir = root().join("crates/rakata-py/python/rakata");
    let mut stubs = Vec::new();
    files(&dir, "pyi", &mut stubs);
    assert!(!stubs.is_empty());
    let mut names = BTreeSet::from(["rakata".to_string(), "author".to_string()]);
    for p in &stubs {
        names.insert(p.file_stem().unwrap().to_string_lossy().into_owned());
        let text = std::fs::read_to_string(p).unwrap();
        let mut word = String::new();
        let mut prev = String::new();
        for ch in text.chars().chain(std::iter::once(' ')) {
            if ch.is_alphanumeric() || ch == '_' {
                word.push(ch);
                continue;
            }
            if !word.is_empty() {
                // `def NAME`, `class NAME`, `NAME:`, `NAME(`, `NAME =`.
                if matches!(prev.as_str(), "def" | "class" | "import")
                    || matches!(ch, ':' | '(' | '=')
                {
                    names.insert(word.clone());
                }
                prev = std::mem::take(&mut word);
            }
            if !ch.is_whitespace() {
                prev.clear();
            }
        }
    }
    names
}

/// Python source with its string literals and comments removed.
fn strip_python(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix("\"\"\"") {
            rest = r.split_once("\"\"\"").map_or("", |(_, after)| after);
        } else if let Some(r) = rest.strip_prefix('"') {
            rest = r.split_once('"').map_or("", |(_, after)| after);
        } else if let Some(r) = rest.strip_prefix('#') {
            rest = r.split_once('\n').map_or("", |(_, after)| after);
            out.push('\n');
        } else {
            let ch = rest.chars().next().unwrap();
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
        }
    }
    out
}

/// The names a piece of Python uses as an attribute (`.name`), a call
/// (`name(`) or a keyword argument (`name=`).
fn used_names(code: &str) -> Vec<String> {
    let chars: Vec<char> = code.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let attr = start > 0 && chars[start - 1] == '.';
            let next = chars.get(i).copied();
            let call = next == Some('(');
            let kwarg = next == Some('=') && chars.get(i + 1) != Some(&'=');
            if attr || call || kwarg {
                out.push(word);
            }
        } else {
            i += 1;
        }
    }
    out
}

/// An inline span that is a Python expression: `rakata.Mod`,
/// `dialog.link(d, …)`, `NodeKey.parse("E3").is_fragile()`.
fn looks_like_python(span: &str) -> bool {
    let head: String = span
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
        .collect();
    let first = head.chars().next();
    first.is_some_and(|c| c.is_alphabetic() || c == '_')
        && (span.starts_with("rakata.") || span[head.len()..].starts_with('('))
}

#[test]
fn every_python_name_the_rakata_skill_cites_is_in_the_stubs() {
    let names = stub_names();
    for n in [
        "Mod",
        "add_reply",
        "link",
        "declare",
        "compile",
        "write_override",
        "NodeKey",
    ] {
        assert!(names.contains(n), "the stubs lost `{n}`");
    }
    let builtin = ["print", "help", "python3"];
    let mut checked = 0;
    for (f, text) in markdown(&root().join("plugin/skills/rakata")) {
        let mut python = String::new();
        for c in code(&text) {
            if c.python || (c.inline && looks_like_python(&c.text)) {
                python.push_str(&c.text);
                python.push('\n');
            }
        }
        for name in used_names(&strip_python(&python)) {
            if name.starts_with("__") || builtin.contains(&name.as_str()) {
                continue;
            }
            assert!(
                names.contains(&name),
                "{}: `{name}` is not a name of the rakata module's stubs",
                f.display()
            );
            checked += 1;
        }
    }
    assert!(checked > 20, "only {checked} Python names found");
}
