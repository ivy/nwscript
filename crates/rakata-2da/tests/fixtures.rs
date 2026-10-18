//! Two fixtures built here, no game bytes: a small table written by the
//! API, and the same bytes with the header's cell-data size patched to 0,
//! the layout some editors ship. Both read to the same table, and writing
//! the patched one gives the normal one's bytes.

use rakata_2da::{TwoDa, inspect};

/// `bytes`, kept as a seed for the fuzz targets in the directory
/// `$RAKATA_FUZZ_SEEDS` names, when it is set.
fn seed(bytes: &[u8]) -> &[u8] {
    if let Some(dir) = std::env::var_os("RAKATA_FUZZ_SEEDS") {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut h = DefaultHasher::new();
        bytes.hash(&mut h);
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{:016x}", h.finish())), bytes).unwrap();
    }
    bytes
}

fn table() -> TwoDa {
    let mut t = TwoDa::new(["label", "name", "description", "icon"]);
    t.add_row(
        "0",
        &[
            ("label", ""),
            ("name", "0"),
            ("description", "0"),
            ("icon", ""),
        ],
    )
    .unwrap();
    t.add_row(
        "1",
        &[
            ("label", "ADVANCED_JEDI_DEFENSE"),
            ("name", "148"),
            ("description", "1126"),
            ("icon", "I_jedidef02"),
        ],
    )
    .unwrap();
    t.add_row(
        "2",
        &[
            ("label", "ARMOUR_PROF_HEAVY"),
            ("name", "1"),
            ("description", "0"),
            ("icon", ""),
        ],
    )
    .unwrap();
    t
}

/// The offset of the size field: after the signature, the names, the row
/// count, the labels and the offsets.
fn size_field_at(bytes: &[u8], columns: usize, rows: usize) -> usize {
    let names_end = bytes.iter().position(|&b| b == 0).unwrap() + 1;
    let mut p = names_end + 4;
    for _ in 0..rows {
        p = bytes[p..].iter().position(|&b| b == b'\t').unwrap() + p + 1;
    }
    p + rows * columns * 2
}

fn patched_to_zero(normal: &[u8]) -> Vec<u8> {
    let at = size_field_at(normal, 4, 3);
    let mut patched = normal.to_vec();
    assert_ne!(&patched[at..at + 2], &[0, 0], "the normal size is not 0");
    patched[at] = 0;
    patched[at + 1] = 0;
    patched
}

#[test]
fn a_zero_size_field_reads_every_cell_and_writes_back_normal() {
    let t = table();
    let normal = t.to_bytes().unwrap();
    let patched = patched_to_zero(&normal);
    assert_ne!(normal, patched);

    let from_normal = TwoDa::parse(seed(&normal)).unwrap();
    let from_patched = TwoDa::parse(seed(&patched)).unwrap();
    assert_eq!(from_normal, t);
    assert_eq!(from_patched, t);
    assert!(from_normal.semantic_eq(&from_patched));
    assert_eq!(
        from_patched.get("1", "label").unwrap(),
        "ADVANCED_JEDI_DEFENSE"
    );
    assert_eq!(from_patched.get("1", "icon").unwrap(), "I_jedidef02");
    assert_eq!(from_patched.get("2", "icon").unwrap(), "");

    assert_eq!(from_patched.to_bytes().unwrap(), normal);
}

#[test]
fn the_layout_line_tells_the_two_apart() {
    let normal = table().to_bytes().unwrap();
    let patched = patched_to_zero(&normal);
    let a = inspect(seed(&normal)).unwrap();
    let b = inspect(seed(&patched)).unwrap();
    assert_eq!(a.declared_size as usize, a.strings_len);
    assert_eq!(b.declared_size, 0);
    assert_eq!(b.strings_len, a.strings_len);
    assert!(b.shared && b.first_occurrence_order);
    assert!(
        b.to_string()
            .starts_with("2da: tab-ended names, size 0 for "),
        "{b}"
    );
}

#[test]
fn a_repeated_column_name_pairs_by_occurrence_in_a_diff() {
    // Found by fuzzing: the second `icon` was compared with the first, so a
    // table differed from itself.
    let mut a = TwoDa::new(["label", "icon", "icon"]);
    a.add_row("0", &[("label", "x")]).unwrap();
    a.rows[0].cells[2] = "second".into();
    assert!(a.diff(&a).is_empty());
    let mut b = a.clone();
    b.rows[0].cells[2] = "changed".into();
    assert_eq!(
        a.diff(&b),
        [rakata_2da::Change::Cell {
            label: "0".into(),
            column: "icon".into(),
            before: "second".into(),
            after: "changed".into(),
        }]
    );
    let mut c = TwoDa::new(["label", "icon"]);
    c.add_row("0", &[("label", "x")]).unwrap();
    assert_eq!(
        a.diff(&c),
        [rakata_2da::Change::ColumnRemoved {
            name: "icon".into()
        }]
    );
}

#[test]
fn an_empty_column_name_is_refused_where_names_end_in_nul() {
    // Found by fuzzing: written as a NUL where a name begins, it ended the
    // list, and the table read back without it.
    let mut t = TwoDa::new(["labeln", ""]);
    t.terminator = rakata_2da::Terminator::Nul;
    assert_eq!(
        t.to_bytes().unwrap_err().code(),
        rakata_2da::codes::UNREPRESENTABLE
    );
    t.terminator = rakata_2da::Terminator::Tab;
    let bytes = t.to_bytes().unwrap();
    assert_eq!(TwoDa::parse(seed(&bytes)).unwrap(), t);
}

#[test]
fn a_row_count_past_the_bytes_is_refused_before_it_allocates() {
    // 1.2 billion rows in a 50-byte file: the reader once reserved room
    // for every label (18 GB) before reading one. Found by fuzzing.
    let mut b = b"2DA V2.b\nlabel\tname\t\0".to_vec();
    b.extend_from_slice(&0x4500_0003u32.to_le_bytes());
    b.extend_from_slice(b"0\t1\t2\t");
    let e = TwoDa::parse(seed(&b)).unwrap_err();
    assert_eq!(e.code(), rakata_2da::codes::TRUNCATED);
    assert_eq!(
        inspect(&b).unwrap_err().code(),
        rakata_2da::codes::TRUNCATED
    );
}
