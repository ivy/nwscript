//! `cargo xtask readme`: regenerate the tables `docs/results.md` carries
//! from reports, between marker comments. Today one table: the round-trip
//! conformance summary from a `rakata conformance --report` JSON file,
//! between `<!-- rakata-conformance:begin -->` and
//! `<!-- rakata-conformance:end -->`. Running it twice over the same report
//! leaves the file unchanged.

use std::path::PathBuf;

use rakata::conformance::Report;

const BEGIN: &str = "<!-- rakata-conformance:begin -->";
const END: &str = "<!-- rakata-conformance:end -->";

const USAGE: &str = "\
usage: cargo xtask readme --report FILE [--readme docs/results.md] [--check]

Insert the per-format table of a `rakata conformance --report FILE` report
between the rakata-conformance markers of docs/results.md, or of the file
--readme names. --check exits 1 instead of writing when the file would
change.";

pub fn main(args: &[String]) -> Result<(), String> {
    let mut report: Option<PathBuf> = None;
    let mut readme = PathBuf::from("docs/results.md");
    let mut check = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--report" => report = Some(it.next().ok_or("--report needs FILE")?.into()),
            "--readme" => readme = it.next().ok_or("--readme needs FILE")?.into(),
            "--check" => check = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => return Err(format!("unknown argument `{other}`\n{USAGE}")),
        }
    }
    let report = report.ok_or_else(|| format!("--report FILE is required\n{USAGE}"))?;
    let text =
        std::fs::read_to_string(&report).map_err(|e| format!("{}: {e}", report.display()))?;
    let report: Report =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", report.display()))?;
    let current =
        std::fs::read_to_string(&readme).map_err(|e| format!("{}: {e}", readme.display()))?;
    let updated = splice(&current, &report.markdown_table())?;
    if updated == current {
        println!("{} is current", readme.display());
        return Ok(());
    }
    if check {
        return Err(format!(
            "{} is out of date; run `cargo xtask readme --report ...`",
            readme.display()
        ));
    }
    std::fs::write(&readme, updated).map_err(|e| format!("{}: {e}", readme.display()))?;
    println!("{} updated", readme.display());
    Ok(())
}

/// `readme` with the text between the markers replaced by `table`; the
/// markers stay, each on its own line.
pub fn splice(readme: &str, table: &str) -> Result<String, String> {
    let begin = readme
        .find(BEGIN)
        .ok_or_else(|| format!("the README has no `{BEGIN}` marker"))?;
    let after_begin = begin + BEGIN.len();
    let end = readme[after_begin..]
        .find(END)
        .map(|i| after_begin + i)
        .ok_or_else(|| format!("the README has no `{END}` marker after `{BEGIN}`"))?;
    let mut out = String::with_capacity(readme.len() + table.len());
    out.push_str(&readme[..after_begin]);
    out.push('\n');
    out.push_str(table.trim_end_matches('\n'));
    out.push('\n');
    out.push_str(&readme[end..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splicing_is_idempotent_and_keeps_the_markers() {
        let readme = format!("before\n{BEGIN}\nold\n{END}\nafter\n");
        let once = splice(&readme, "| a |\n| 1 |\n").unwrap();
        assert_eq!(
            once,
            format!("before\n{BEGIN}\n| a |\n| 1 |\n{END}\nafter\n")
        );
        assert_eq!(splice(&once, "| a |\n| 1 |\n").unwrap(), once);
    }

    #[test]
    fn a_readme_without_markers_is_an_error_naming_them() {
        let e = splice("nothing here", "t").unwrap_err();
        assert!(e.contains(BEGIN), "{e}");
    }
}
