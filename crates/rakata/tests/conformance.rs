//! The round-trip suite over trees built here: a writer that is wrong is
//! caught as different and exits 3; one that changes layout alone is equal
//! after re-read and named by the known variants; the archives built in
//! memory are identical; files no format reads are counted.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rakata::Exit;
use rakata::conformance::{Class, Format, Known, Nested, Problem, Registry, Variant, run};
use rakata_archive::{Erf, ErfKind, Key, Rim};
use rakata_res::ResId;

/// Reads bytes as they are and writes them reversed: a wrong writer.
struct Reversing;

impl Format for Reversing {
    type Model<'a> = &'a [u8];

    fn name(&self) -> &'static str {
        "reversing"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rev"]
    }

    fn read<'a>(&self, bytes: &'a [u8]) -> Result<&'a [u8], Problem> {
        Ok(bytes)
    }

    fn write(&self, model: &&[u8]) -> Result<Vec<u8>, Problem> {
        Ok(model.iter().rev().copied().collect())
    }

    fn semantic_eq(&self, a: &&[u8], b: &&[u8]) -> bool {
        a == b
    }
}

/// Reads the text before the first NUL and writes it with one trailing
/// NUL: a layout the reader does not see, so other paddings are equal
/// after re-read. The layout line is the padding length.
struct Padded;

impl Format for Padded {
    type Model<'a> = &'a [u8];

    fn name(&self) -> &'static str {
        "padded"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["pad"]
    }

    fn read<'a>(&self, bytes: &'a [u8]) -> Result<&'a [u8], Problem> {
        if bytes.is_empty() {
            return Err(Problem::new("RE0202", "an empty file has no text"));
        }
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        Ok(&bytes[..end])
    }

    fn write(&self, model: &&[u8]) -> Result<Vec<u8>, Problem> {
        let mut v = model.to_vec();
        v.push(0);
        Ok(v)
    }

    fn semantic_eq(&self, a: &&[u8], b: &&[u8]) -> bool {
        a == b
    }

    fn layout(&self, bytes: &[u8], model: &&[u8]) -> Option<String> {
        Some(format!("pad: {} trailing", bytes.len() - model.len()))
    }
}

/// A container: a sequence of `name.ext\0` + u32 length + bytes.
struct Box_;

impl Format for Box_ {
    type Model<'a> = Vec<(String, &'a [u8])>;

    fn name(&self) -> &'static str {
        "box"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["box"]
    }

    fn read<'a>(&self, mut bytes: &'a [u8]) -> Result<Vec<(String, &'a [u8])>, Problem> {
        let mut out = Vec::new();
        while !bytes.is_empty() {
            let nul = bytes.iter().position(|&b| b == 0).unwrap();
            let name = String::from_utf8(bytes[..nul].to_vec()).unwrap();
            let len = u32::from_le_bytes(bytes[nul + 1..nul + 5].try_into().unwrap()) as usize;
            out.push((name, &bytes[nul + 5..nul + 5 + len]));
            bytes = &bytes[nul + 5 + len..];
        }
        Ok(out)
    }

    fn write(&self, model: &Vec<(String, &[u8])>) -> Result<Vec<u8>, Problem> {
        let mut v = Vec::new();
        for (name, data) in model {
            v.extend_from_slice(name.as_bytes());
            v.push(0);
            v.extend_from_slice(&(data.len() as u32).to_le_bytes());
            v.extend_from_slice(data);
        }
        Ok(v)
    }

    fn semantic_eq(&self, a: &Vec<(String, &[u8])>, b: &Vec<(String, &[u8])>) -> bool {
        a == b
    }

    fn members<'m>(&self, model: &'m Vec<(String, &[u8])>) -> Vec<Nested<'m>> {
        model
            .iter()
            .map(|(name, data)| Nested {
                name: name.clone(),
                extension: name.rsplit_once('.').map(|(_, e)| e.to_string()),
                bytes: data,
            })
            .collect()
    }
}

fn boxed(members: &[(&str, &[u8])]) -> Vec<u8> {
    let model: Vec<(String, &[u8])> = members.iter().map(|(n, d)| (n.to_string(), *d)).collect();
    Box_.write(&model).unwrap()
}

/// A tree in a fresh temporary directory with a valid empty `chitin.key`.
fn tree(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("rakata-conformance-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Override")).unwrap();
    std::fs::write(dir.join("chitin.key"), Key::new().to_bytes().unwrap()).unwrap();
    dir
}

fn put(root: &Path, rel: &str, bytes: &[u8]) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

fn registry() -> Registry {
    let mut r = Registry::builtin();
    r.register(Box::new(Reversing));
    r.register(Box::new(Padded));
    r.register(Box::new(Box_));
    r
}

fn by_name<'r>(
    report: &'r rakata::conformance::Report,
    name: &str,
) -> &'r rakata::conformance::FormatReport {
    report.formats.iter().find(|f| f.name == name).unwrap()
}

#[test]
fn a_wrong_writer_is_different_and_exits_3() {
    let root = tree("wrong");
    put(&root, "Override/a.rev", b"abc");
    put(&root, "Override/same.rev", b"aba");
    let report = run(&root, &registry(), &Known::default(), 2).unwrap();
    let f = by_name(&report, "reversing");
    assert_eq!(f.counts.files, 2);
    assert_eq!(f.counts.identical, 1, "a palindrome reverses to itself");
    assert_eq!(f.counts.different, 1);
    let e = &f.entries["Override/a.rev"];
    assert_eq!(e.class, Class::Different);
    assert!(
        e.detail
            .as_deref()
            .unwrap()
            .contains("first difference at offset 0"),
        "{e:?}"
    );
    assert_eq!(report.exit(), Exit::Different);
    assert!(
        report.diagnostics().is_empty(),
        "a difference is not a diagnostic"
    );
    let md = report.markdown();
    assert!(md.contains("Different (1):"), "{md}");
    assert!(md.contains("- `Override/a.rev`:"), "{md}");
}

#[test]
fn a_layout_change_is_equal_after_reread_and_named_by_the_known_variants() {
    let root = tree("layout");
    put(&root, "Override/one.pad", b"hello\0");
    put(&root, "Override/two.pad", b"hello\0\0\0");
    put(&root, "Override/three.pad", b"hello");
    let mut known = Known::default();
    known.variant.insert(
        "triple-nul".to_string(),
        Variant {
            format: "padded".into(),
            writer: "a test".into(),
            description: "three trailing NULs".into(),
            recognised: "pad: 3 trailing".into(),
            files: vec![],
        },
    );
    known.variant.insert(
        "by-path".to_string(),
        Variant {
            format: "padded".into(),
            writer: "a test".into(),
            description: "named by its path".into(),
            recognised: "never matches".into(),
            files: vec!["override/thr*.pad".into()],
        },
    );
    let report = run(&root, &registry(), &known, 1).unwrap();
    let f = by_name(&report, "padded");
    assert_eq!(
        (f.counts.identical, f.counts.equal, f.counts.different),
        (1, 2, 0)
    );
    let two = &f.entries["Override/two.pad"];
    assert_eq!(two.class, Class::Equal);
    assert_eq!(two.variant.as_deref(), Some("triple-nul"));
    assert_eq!(two.layout.as_deref(), Some("pad: 3 trailing"));
    let three = &f.entries["Override/three.pad"];
    assert_eq!(three.variant.as_deref(), Some("by-path"));
    assert_eq!(report.exit(), Exit::Ok);
    assert_eq!(
        report.variants.len(),
        2,
        "the report carries the variants it names"
    );
    assert!(report.markdown().contains("[triple-nul]"));
}

#[test]
fn a_file_the_reader_refuses_is_failed_with_its_code_and_exits_1() {
    let root = tree("failed");
    put(&root, "Override/empty.pad", b"");
    let report = run(&root, &registry(), &Known::default(), 1).unwrap();
    let f = by_name(&report, "padded");
    assert_eq!(f.counts.failed, 1);
    let e = &f.entries["Override/empty.pad"];
    assert_eq!(e.class, Class::Failed);
    assert_eq!(e.code.as_deref(), Some("RE0202"));
    assert_eq!(report.exit(), Exit::Errors);
    let diags = report.diagnostics();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code.id, "RE0202");
    assert!(
        diags[0].message.starts_with("Override/empty.pad: "),
        "{}",
        diags[0].message
    );
}

#[test]
fn members_are_classified_by_their_own_format_and_the_rest_counted_as_no_reader() {
    let root = tree("members");
    put(
        &root,
        "modules/m.box",
        &boxed(&[("a.rev", b"abc"), ("b.pad", b"x\0\0"), ("c.utc", b"GFF ")]),
    );
    put(&root, "Override/loose.utc", b"GFF ");
    put(&root, "Override/noext", b"");
    let report = run(&root, &registry(), &Known::default(), 1).unwrap();
    assert_eq!(by_name(&report, "box").counts.identical, 1);
    let rev = by_name(&report, "reversing");
    assert_eq!(rev.counts.different, 1);
    assert!(
        rev.entries.contains_key("modules/m.box!a.rev"),
        "{:?}",
        rev.entries
    );
    assert_eq!(by_name(&report, "padded").counts.equal, 1);
    assert_eq!(report.no_reader.total, 3);
    assert_eq!(report.no_reader.by_extension["utc"], 2);
    assert_eq!(report.no_reader.by_extension["(none)"], 1);
    // chitin.key, the box, its three members, the two loose files.
    assert_eq!(report.files, 7);
    assert_eq!(
        by_name(&report, "archive").counts.identical,
        1,
        "the empty KEY"
    );
}

#[test]
fn archives_built_in_memory_are_identical_and_their_members_listed() {
    let root = tree("archives");
    let utc = ResId::parse("p_bastila.utc").unwrap();
    let mut rim = Rim::new();
    rim.add(utc, b"GFF V3.2".to_vec()).unwrap();
    put(&root, "modules/danm13.rim", &rim.to_bytes().unwrap());
    let mut erf = Erf::new(ErfKind::Mod);
    erf.add(utc, b"GFF V3.2".to_vec()).unwrap();
    erf.add(
        ResId::parse("inner.rim").unwrap(),
        Rim::new().to_bytes().unwrap(),
    )
    .unwrap();
    put(&root, "modules/danm13.mod", &erf.to_bytes().unwrap());
    put(
        &root,
        "lips/danm13_loc.mod",
        &Erf::new(ErfKind::Mod).to_bytes().unwrap(),
    );
    put(&root, "rims/global.rim", &Rim::new().to_bytes().unwrap());
    put(
        &root,
        "patch.erf",
        &Erf::new(ErfKind::Erf).to_bytes().unwrap(),
    );
    put(
        &root,
        "TexturePacks/swpc_tex_gui.erf",
        &Erf::new(ErfKind::Erf).to_bytes().unwrap(),
    );
    put(&root, "streamwaves/m03af/a.lip", b"LIP V1.0");
    put(
        &root,
        "dialog.tlk",
        &rakata_tlk::Tlk::default().to_bytes().unwrap(),
    );
    let report = run(&root, &registry(), &Known::default(), 0).unwrap();
    let a = by_name(&report, "archive");
    assert_eq!(a.counts.files, 8, "{:?}", a.entries);
    assert_eq!(
        a.counts.identical, 8,
        "seven files and the RIM inside the .mod"
    );
    assert_eq!(report.no_reader.by_extension["utc"], 2);
    assert_eq!(report.no_reader.by_extension["lip"], 1);
    assert_eq!(by_name(&report, "tlk").counts.identical, 1);
    assert_eq!(report.exit(), Exit::Ok);
    let md = report.markdown_table();
    assert!(md.contains("| `archive` | 8 | 8 | 0 | 0 | |"), "{md}");
}

#[test]
fn a_directory_without_chitin_key_is_not_an_install() {
    let dir = std::env::temp_dir().join(format!(
        "rakata-conformance-noinstall-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let d = run(&dir, &Registry::builtin(), &Known::default(), 1).unwrap_err();
    assert_eq!(d.code.id, "RE0501");
    assert!(d.message.contains("chitin.key"), "{}", d.message);
}

#[test]
fn only_scores_the_formats_named_but_still_walks_into_the_others() {
    let r = registry()
        .only(&["archive".to_string(), "box".to_string()])
        .unwrap();
    assert!(r.is_scored("box") && !r.is_scored("reversing"));
    let e = registry().only(&["gff".to_string()]).unwrap_err();
    assert!(
        e.contains("no format `gff`") && e.contains("archive"),
        "{e}"
    );
    // Selecting only the member's format reaches it inside the unselected box.
    let root = tree("only");
    put(&root, "modules/m.box", &boxed(&[("a.rev", b"abc")]));
    let registry = registry().only(&["reversing".to_string()]).unwrap();
    let report = run(&root, &registry, &Known::default(), 1).unwrap();
    let names: Vec<_> = report.formats.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        ["reversing"],
        "unscored formats stay out of the report"
    );
    let rev = by_name(&report, "reversing");
    assert_eq!(rev.counts.different, 1);
    assert!(
        rev.entries.contains_key("modules/m.box!a.rev"),
        "{:?}",
        rev.entries
    );
    assert_eq!(
        report.no_reader.total, 0,
        "the box and the key are walked, not counted"
    );
}

#[test]
#[should_panic(expected = "two formats read `.rev`")]
fn two_formats_claiming_one_extension_is_a_programming_error() {
    let mut r = registry();
    r.register(Box::new(Reversing));
}

#[test]
fn the_known_variants_file_in_the_repository_parses_and_names_registered_formats() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/rakata-known.toml");
    let known = Known::load(&path).unwrap();
    assert!(!known.variant.is_empty());
    let names = Registry::builtin().names();
    for (name, v) in &known.variant {
        assert!(
            names.contains(&v.format.as_str()),
            "{name} names format `{}`",
            v.format
        );
        assert!(!v.description.is_empty() && !v.writer.is_empty() && !v.recognised.is_empty());
    }
    let missing = Known::load(Path::new("/nonexistent/rakata-known.toml")).unwrap();
    assert!(missing.variant.is_empty());
}

#[test]
fn the_report_round_trips_through_json() {
    let root = tree("json");
    put(&root, "Override/a.rev", b"abc");
    let report = run(&root, &registry(), &Known::default(), 1).unwrap();
    let text = serde_json::to_string(&report).unwrap();
    let back: rakata::conformance::Report = serde_json::from_str(&text).unwrap();
    assert_eq!(back.files, report.files);
    assert_eq!(back.markdown_table(), report.markdown_table());
    let doc: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(doc["wall_seconds"].is_number());
    assert!(doc["formats"][0]["busy_seconds"].is_number());
    let _: BTreeMap<String, serde_json::Value> =
        serde_json::from_value(doc["no_reader"]["by_extension"].clone()).unwrap();
}

/// The install `var` names, or none with a message naming the variable.
fn game_tree(var: &str) -> Option<PathBuf> {
    match std::env::var(var) {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: ${var} is unset; set it to a KotOR 1 install in the game's layout");
            None
        }
    }
}

fn repository_known() -> Known {
    Known::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/rakata-known.toml"))
        .unwrap()
}

/// The vanilla install: every archive identical, the count of the
/// `rakata-archive` suite.
#[test]
fn every_vanilla_archive_is_identical() {
    let Some(root) = game_tree("RAKATA_GAME_K1") else {
        return;
    };
    let registry = Registry::builtin().only(&["archive".to_string()]).unwrap();
    let report = run(&root, &registry, &repository_known(), 0).unwrap();
    let a = by_name(&report, "archive");
    println!(
        "{} archives, {} identical, in {:.2} s wall ({:.2} s busy) on {} threads",
        a.counts.files, a.counts.identical, report.wall_seconds, a.busy_seconds, report.threads
    );
    assert_eq!(a.counts.files, 401, "{:?}", a.entries);
    assert_eq!(a.counts.identical, 401, "{:?}", a.entries);
    assert_eq!(report.exit(), Exit::Ok);
}

/// The modded build: nothing different, and every file that is equal after
/// re-read has a named variant.
#[test]
fn the_build_has_nothing_different_and_every_variant_is_named() {
    let Some(root) = game_tree("RAKATA_BUILD") else {
        return;
    };
    let report = run(&root, &Registry::builtin(), &repository_known(), 0).unwrap();
    println!("{}", report.markdown_table());
    println!(
        "{} files in {:.2} s wall on {} threads",
        report.files, report.wall_seconds, report.threads
    );
    for f in &report.formats {
        assert_eq!(f.counts.different, 0, "{}: {:?}", f.name, f.entries);
        assert_eq!(f.counts.failed, 0, "{}: {:?}", f.name, f.entries);
        let unnamed: Vec<_> = f
            .entries
            .iter()
            .filter(|(_, e)| e.class == Class::Equal && e.variant.is_none())
            .collect();
        assert!(
            unnamed.is_empty(),
            "{}: equal after re-read with no variant in rakata-known.toml: {unnamed:#?}",
            f.name
        );
    }
    assert_eq!(report.exit(), Exit::Ok);
}
