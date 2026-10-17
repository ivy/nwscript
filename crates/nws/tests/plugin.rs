//! The plugin's skill defers to `--help`: every `nws` command it names
//! exists, and every flag it uses is in that command's help or the global
//! options.

use assert_cmd::Command;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn markdown(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            markdown(&p, out);
        } else if p.extension().is_some_and(|x| x == "md") {
            out.push(p);
        }
    }
}

fn help(args: &[&str]) -> String {
    let out = Command::cargo_bin("nws")
        .unwrap()
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "nws {} failed", args.join(" "));
    String::from_utf8(out.stdout).unwrap()
}

/// The code in a markdown text: each line of a fenced block, and each
/// inline code span elsewhere.
fn code(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        } else if fenced {
            out.push(line.to_string());
        } else {
            out.extend(line.split('`').skip(1).step_by(2).map(str::to_string));
        }
    }
    out
}

/// Each `nws …` invocation in the text's code, as its words.
fn invocations(text: &str) -> Vec<Vec<String>> {
    code(text)
        .iter()
        .filter_map(|c| c.trim_start().strip_prefix("nws "))
        .map(|rest| rest.split_whitespace().map(str::to_string).collect())
        .collect()
}

#[test]
fn every_command_and_flag_the_skill_names_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugin");
    let mut files = Vec::new();
    markdown(&root, &mut files);
    assert!(!files.is_empty());
    let global = help(&["--help"]);
    let commands: BTreeSet<String> = global
        .split("Commands:")
        .nth(1)
        .unwrap()
        .split("Options:")
        .next()
        .unwrap()
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect();
    let mut checked = 0;
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        for words in invocations(&text) {
            // Global options come first; the first plain word is the command.
            let Some(cmd) = words
                .iter()
                .enumerate()
                .find(|(i, w)| !w.starts_with('-') && !(*i > 0 && takes_value(&words[i - 1])))
                .map(|(_, w)| w.trim_end_matches(['.', ',', ':', ')']))
            else {
                continue;
            };
            if cmd.starts_with('<') || cmd.chars().any(|c| c.is_ascii_uppercase()) || cmd.is_empty()
            {
                // `nws <command>` placeholders.
                continue;
            }
            assert!(
                commands.contains(cmd),
                "{}: `nws {}` names no command",
                f.display(),
                words.join(" ")
            );
            let own = help(&[cmd, "--help"]);
            for w in words.iter().filter(|w| w.starts_with("--")) {
                let flag = w.split('=').next().unwrap();
                assert!(
                    own.contains(flag) || global.contains(flag),
                    "{}: `{flag}` is not in `nws {cmd} --help`",
                    f.display()
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 10, "only {checked} invocations found");
}

/// Global options that take a value, so the word after one is no command.
fn takes_value(flag: &str) -> bool {
    matches!(
        flag,
        "--game"
            | "-I"
            | "--include"
            | "--nwscript"
            | "--declarations"
            | "--overlay"
            | "--format"
            | "--allow"
            | "--deny"
            | "--compat"
            | "--color"
    )
}
