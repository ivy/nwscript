//! Two fixtures built here, no game bytes: a small table written by the
//! API, and the same bytes with the header's cell-data size patched to 0,
//! the layout some editors ship. Both read to the same table, and writing
//! the patched one gives the normal one's bytes.

use rakata_2da::{TwoDa, inspect};

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

    let from_normal = TwoDa::parse(&normal).unwrap();
    let from_patched = TwoDa::parse(&patched).unwrap();
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
    let a = inspect(&normal).unwrap();
    let b = inspect(&patched).unwrap();
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
