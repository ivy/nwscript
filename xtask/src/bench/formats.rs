//! The formats the benchmark measures, and Rakata's reader and writer for
//! each, through the adapters `rakata conformance` registers.

use rakata::conformance::Format;
use rakata::formats as f;

/// One benchmarked format: a name, the extensions its files carry.
pub struct BenchFormat {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
}

pub const ALL: &[BenchFormat] = &[
    BenchFormat {
        name: "key",
        extensions: &["key"],
    },
    BenchFormat {
        name: "bif",
        extensions: &["bif"],
    },
    BenchFormat {
        name: "erf",
        extensions: &["erf", "mod", "sav", "hak"],
    },
    BenchFormat {
        name: "rim",
        extensions: &["rim"],
    },
    BenchFormat {
        name: "gff",
        extensions: &[
            "gff", "res", "are", "ifo", "git", "gic", "utc", "utd", "ute", "uti", "utp", "uts",
            "utm", "utt", "utw", "utg", "dlg", "jrl", "fac", "itp", "ptm", "ptt", "bic", "gui",
            "pth", "btc", "bti", "btd", "bte", "btp", "bts", "btt", "btm", "btg",
        ],
    },
    BenchFormat {
        name: "2da",
        extensions: &["2da"],
    },
    BenchFormat {
        name: "tlk",
        extensions: &["tlk"],
    },
    BenchFormat {
        name: "lip",
        extensions: &["lip"],
    },
    BenchFormat {
        name: "tpc",
        extensions: &["tpc"],
    },
    BenchFormat {
        name: "txi",
        extensions: &["txi"],
    },
    BenchFormat {
        name: "tga",
        extensions: &["tga"],
    },
    BenchFormat {
        name: "lyt",
        extensions: &["lyt"],
    },
    BenchFormat {
        name: "vis",
        extensions: &["vis"],
    },
    BenchFormat {
        name: "walkmesh",
        extensions: &["wok", "pwk", "dwk"],
    },
    BenchFormat {
        name: "mdl",
        extensions: &["mdl"],
    },
];

/// The format whose files carry `ext`, in any case. An ERF's extensions
/// come first, so `.sav` is an archive here.
pub fn by_extension(ext: &str) -> Option<&'static BenchFormat> {
    ALL.iter()
        .find(|f| f.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
}

pub fn by_name(name: &str) -> Option<&'static BenchFormat> {
    ALL.iter().find(|f| f.name == name)
}

/// A file's bytes, with its MDX for a model.
pub struct Input<'a> {
    pub bytes: &'a [u8],
    pub mdx: Option<&'a [u8]>,
}

/// Whether Rakata writes this format.
pub fn writes(format: &str) -> bool {
    format != "mdl"
}

fn read_with<F: Format>(fmt: &F, bytes: &[u8]) -> Result<(), String> {
    fmt.read(bytes)
        .map(|m| {
            std::hint::black_box(&m);
        })
        .map_err(|p| format!("{}: {}", p.code, p.message))
}

fn write_with<F: Format>(fmt: &F, bytes: &[u8]) -> Result<Vec<u8>, String> {
    let m = fmt
        .read(bytes)
        .map_err(|p| format!("{}: {}", p.code, p.message))?;
    fmt.write(&m)
        .map_err(|p| format!("the writer failed: {}: {}", p.code, p.message))
}

/// `Ok(true)` when both read to the same data; `Err` names the side that
/// does not read.
fn same_with<F: Format>(fmt: &F, a: &[u8], b: &[u8]) -> Result<bool, String> {
    let ma = fmt
        .read(a)
        .map_err(|p| format!("the original does not read: {}: {}", p.code, p.message))?;
    let mb = fmt
        .read(b)
        .map_err(|p| format!("does not read: {}: {}", p.code, p.message))?;
    Ok(fmt.semantic_eq(&ma, &mb))
}

macro_rules! dispatch {
    ($format:expr, $fun:ident ( $($arg:expr),* ), $mdl:expr) => {
        match $format {
            "key" | "bif" | "erf" | "rim" => $fun(&f::Archive, $($arg),*),
            "gff" => $fun(&f::Gff, $($arg),*),
            "2da" => $fun(&f::TwoDa, $($arg),*),
            "tlk" => $fun(&f::Tlk, $($arg),*),
            "lip" => $fun(&f::Lip, $($arg),*),
            "tpc" => $fun(&f::Tpc, $($arg),*),
            "txi" => $fun(&f::Txi, $($arg),*),
            "tga" => $fun(&f::Tga, $($arg),*),
            "lyt" => $fun(&f::Lyt, $($arg),*),
            "vis" => $fun(&f::Vis, $($arg),*),
            "walkmesh" => $fun(&f::Walkmesh, $($arg),*),
            "mdl" => $mdl,
            other => Err(format!("no format `{other}`")),
        }
    };
}

/// Read a file as Rakata does.
pub fn read(format: &str, input: &Input) -> Result<(), String> {
    dispatch!(
        format,
        read_with(input.bytes),
        rakata_mdl::parse(input.bytes, input.mdx.unwrap_or_default())
            .map(|m| {
                std::hint::black_box(&m);
            })
            .map_err(|e| e.to_string())
    )
}

/// Read a file and write it again as Rakata does.
pub fn write(format: &str, input: &Input) -> Result<Vec<u8>, String> {
    dispatch!(
        format,
        write_with(input.bytes),
        Err("Rakata does not write MDL".to_string())
    )
}

/// Whether `other` holds the data of `original`, as Rakata reads both.
pub fn same(format: &str, original: &[u8], other: &[u8]) -> Result<bool, String> {
    dispatch!(
        format,
        same_with(original, other),
        Err("Rakata does not compare MDL".to_string())
    )
}

/// The first place two files differ, as `rakata diff` words it; `None`
/// when the diff finds no difference in the data or does not read them.
pub fn first_difference(name: &str, a: &[u8], b: &[u8]) -> Option<String> {
    // A GFF by its tree: `rakata diff` reads a dialogue as a dialogue, and
    // two trees can differ where two dialogues do not.
    if let (Ok(ga), Ok(gb)) = (rakata_gff::Gff::parse(a), rakata_gff::Gff::parse(b)) {
        let changes = rakata_gff::diff(&ga, &gb);
        let shown = |v: &Option<rakata_gff::Value>| match v {
            Some(v) => format!("{v:?}"),
            None => "absent".into(),
        };
        let line = match changes.first() {
            Some(c) => format!(
                "{} changes; first at `{}`: {} -> {}",
                changes.len(),
                c.path,
                shown(&c.before),
                shown(&c.after)
            ),
            None => {
                let s = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
                format!(
                    "no field differs by label; the header `{}{}` -> `{}{}`",
                    s(&ga.file_type),
                    s(&ga.version),
                    s(&gb.file_type),
                    s(&gb.version)
                )
            }
        };
        return Some(line.chars().take(240).collect());
    }
    let d = rakata::diff::diff_resources((name, a), (name, b), &rakata::diff::Options::default());
    let text = d.text();
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .nth(1)
        .map(|l| l.chars().take(200).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_format_claims_what_its_rakata_adapter_reads() {
        for (name, adapter) in [
            ("gff", f::Gff.extensions()),
            ("2da", f::TwoDa.extensions()),
            ("tlk", f::Tlk.extensions()),
            ("lip", f::Lip.extensions()),
            ("walkmesh", f::Walkmesh.extensions()),
        ] {
            assert_eq!(by_name(name).unwrap().extensions, adapter, "{name}");
        }
        let mut have: Vec<&str> = ["key", "bif", "erf", "rim"]
            .iter()
            .flat_map(|n| by_name(n).unwrap().extensions.iter().copied())
            .collect();
        let mut want = f::Archive.extensions().to_vec();
        want.sort_unstable();
        have.sort_unstable();
        assert_eq!(have, want);
    }

    #[test]
    fn extensions_find_their_format_in_any_case() {
        assert_eq!(by_extension("MOD").map(|f| f.name), Some("erf"));
        assert_eq!(by_extension("dwk").map(|f| f.name), Some("walkmesh"));
        assert!(by_extension("mdx").is_none());
    }
}
