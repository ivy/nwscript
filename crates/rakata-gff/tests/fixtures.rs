//! Hand-made files built with the API: the writer's layout for a file built
//! from scratch, the path API, the diff, and the reader's refusals. No game
//! bytes.

use rakata_gff::{
    Change, Error, FieldType, Gff, Label, LocString, Path, Struct, Value, codes, diff,
};

fn label(s: &str) -> Label {
    Label::new(s).unwrap()
}

/// A creature blueprint with the fields a test needs: a tag, a name, a
/// class list and a few numbers.
fn creature(tag: &str, hp: i16, class: i32) -> Gff {
    let mut g = Gff::new(*b"UTC ");
    g.root
        .push(label("TemplateResRef"), Value::resref("p_bastila").unwrap())
        .push(label("Tag"), Value::string(tag))
        .push(
            label("FirstName"),
            Value::LocString(LocString::strref(31_337)),
        )
        .push(label("CurrentHitPoints"), Value::Short(hp))
        .push(label("Appearance_Type"), Value::Word(4))
        .push(label("ChallengeRating"), Value::Float(1.5));
    let mut c = Struct::new(2);
    c.push(label("Class"), Value::Int(class))
        .push(label("ClassLevel"), Value::Short(3));
    g.root.push(label("ClassList"), Value::List(vec![c]));
    g
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

#[test]
fn a_file_built_from_scratch_has_the_one_pass_layout() {
    let g = creature("Bastila", 24, 3);
    let bytes = g.to_bytes().unwrap();
    assert_eq!(&bytes[..8], b"UTC V3.2");
    // Header: sections adjacent in the document's order.
    let (so, sc, fo, fc, lo, lc, dofs, dc, fio, fic, lio, lic) = (
        u32_at(&bytes, 8),
        u32_at(&bytes, 12),
        u32_at(&bytes, 16),
        u32_at(&bytes, 20),
        u32_at(&bytes, 24),
        u32_at(&bytes, 28),
        u32_at(&bytes, 32),
        u32_at(&bytes, 36),
        u32_at(&bytes, 40),
        u32_at(&bytes, 44),
        u32_at(&bytes, 48),
        u32_at(&bytes, 52),
    );
    assert_eq!((so, sc), (56, 2));
    assert_eq!((fo, fc), (56 + 24, 9));
    assert_eq!((lo, lc), (fo + 9 * 12, 9));
    assert_eq!(dofs, lo + 9 * 16);
    // resref 1+9, string 4+7, locstring 4+8
    assert_eq!(dc, 10 + 11 + 12);
    assert_eq!(fio, dofs + dc);
    // root 7 fields, class struct 2 fields
    assert_eq!(fic, 7 * 4 + 2 * 4);
    assert_eq!(lio, fio + fic);
    assert_eq!(lic, 4 + 4);
    assert_eq!(lio + lic, bytes.len() as u32);
    // Root struct: id -1, indices at 0, 7 fields; class struct id 2, indices
    // at 28, 2 fields.
    assert_eq!(u32_at(&bytes, so as usize), 0xFFFF_FFFF);
    assert_eq!(u32_at(&bytes, so as usize + 4), 0);
    assert_eq!(u32_at(&bytes, so as usize + 8), 7);
    assert_eq!(u32_at(&bytes, so as usize + 12), 2);
    assert_eq!(u32_at(&bytes, so as usize + 16), 28);
    assert_eq!(u32_at(&bytes, so as usize + 20), 2);
    // Field 6 is the list, pointing at list offset 0 with one struct, 1.
    let f6 = fo as usize + 6 * 12;
    assert_eq!(u32_at(&bytes, f6), FieldType::List.id());
    assert_eq!(u32_at(&bytes, f6 + 8), 0);
    assert_eq!(u32_at(&bytes, lio as usize), 1);
    assert_eq!(u32_at(&bytes, lio as usize + 4), 1);
    // Fields 7 and 8 are the class struct's, after the list field.
    let f7 = fo as usize + 7 * 12;
    assert_eq!(u32_at(&bytes, f7), FieldType::Int.id());
    assert_eq!(u32_at(&bytes, f7 + 8), 3);
    // Labels in order of first use, the sixteen bytes NUL padded.
    assert_eq!(&bytes[lo as usize..lo as usize + 16], b"TemplateResRef\0\0");
    assert_eq!(
        &bytes[lo as usize + 7 * 16..lo as usize + 7 * 16 + 5],
        b"Class"
    );
    // A short's data word carries only its sixteen bits.
    let f3 = fo as usize + 3 * 12;
    assert_eq!(u32_at(&bytes, f3 + 8), 24);

    let back = Gff::parse(&bytes).unwrap();
    assert_eq!(back, g);
    assert!(back.semantic_eq(&g));
    assert_eq!(back.to_bytes().unwrap(), bytes);
}

#[test]
fn set_through_a_path_then_write_read_and_get_leaves_the_rest_unchanged() {
    let original = creature("Bastila", 24, 3);
    let mut g = Gff::parse(&original.to_bytes().unwrap()).unwrap();
    let tag = Path::parse("Tag").unwrap();
    let old = g.set(&tag, Value::string("Bastila_Fallen")).unwrap();
    assert_eq!(old, Value::string("Bastila"));
    let bytes = g.to_bytes().unwrap();
    let back = Gff::parse(&bytes).unwrap();
    assert_eq!(back.get(&tag).unwrap(), &Value::string("Bastila_Fallen"));
    let changes = diff(&original, &back);
    assert_eq!(
        changes,
        vec![Change {
            path: tag,
            before: Some(Value::string("Bastila")),
            after: Some(Value::string("Bastila_Fallen")),
        }]
    );
    let mut same = back.clone();
    same.set(&Path::parse("Tag").unwrap(), Value::string("Bastila"))
        .unwrap();
    assert!(diff(&original, &same).is_empty());
}

#[test]
fn diff_of_two_blueprints_differing_in_two_fields_lists_exactly_those_paths() {
    let a = creature("Bastila", 24, 3);
    let b = creature("Bastila", 30, 5);
    let changes = diff(&a, &b);
    let paths: Vec<String> = changes.iter().map(|c| c.path.to_string()).collect();
    assert_eq!(paths, ["CurrentHitPoints", "ClassList[0].Class"]);
    assert_eq!(changes[0].before, Some(Value::Short(24)));
    assert_eq!(changes[0].after, Some(Value::Short(30)));
    assert_eq!(changes[1].before, Some(Value::Int(3)));
    assert_eq!(changes[1].after, Some(Value::Int(5)));
}

#[test]
fn diff_reports_added_and_removed_fields_and_list_elements_and_struct_ids() {
    let a = creature("Bastila", 24, 3);
    let mut b = a.clone();
    b.root.remove("Appearance_Type");
    b.root.push(label("Gender"), Value::Byte(1));
    if let Value::List(items) = b.root.get_mut("ClassList").unwrap() {
        items[0].id = 7;
        items.push(Struct::new(9));
    }
    let changes = diff(&a, &b);
    let shown: Vec<(String, bool, bool)> = changes
        .iter()
        .map(|c| (c.path.to_string(), c.before.is_some(), c.after.is_some()))
        .collect();
    assert_eq!(
        shown,
        [
            ("Appearance_Type".to_string(), true, false),
            ("ClassList[0]".to_string(), true, true),
            ("ClassList[1]".to_string(), false, true),
            ("Gender".to_string(), false, true),
        ]
    );
    assert_eq!(changes[1].before, Some(Value::Dword(2)));
    assert_eq!(changes[1].after, Some(Value::Dword(7)));
    assert!(!a.semantic_eq(&b));
}

#[test]
fn semantic_equality_ignores_field_order_but_not_values() {
    let a = creature("Bastila", 24, 3);
    let mut b = a.clone();
    b.root.fields.swap(0, 1);
    assert_ne!(a, b);
    assert!(a.semantic_eq(&b));
    b.root.set(label("Tag"), Value::string("x"));
    assert!(!a.semantic_eq(&b));
}

#[test]
fn paths_parse_and_print_and_resolve() {
    let p = Path::parse("EntryList[3].RepliesList[0].Index").unwrap();
    assert_eq!(p.to_string(), "EntryList[3].RepliesList[0].Index");
    assert_eq!(p.segments().len(), 5);
    assert_eq!(Path::parse("").unwrap(), Path::root());
    assert_eq!(
        Path::parse("Creature List[0].TemplateResRef")
            .unwrap()
            .to_string(),
        "Creature List[0].TemplateResRef"
    );
    for bad in [
        "[0]",
        "A[",
        "A[x]",
        "A.",
        "A]",
        "A[0]x",
        "ThisLabelIsFarTooLongForGff",
    ] {
        let e = Path::parse(bad).unwrap_err();
        assert_eq!(e.code(), codes::BAD_PATH, "{bad}");
    }

    let g = creature("Bastila", 24, 3);
    assert_eq!(
        g.get(&Path::parse("ClassList[0].ClassLevel").unwrap())
            .unwrap(),
        &Value::Short(3)
    );
    assert_eq!(g.struct_at(&Path::root()).unwrap().id, Struct::ROOT_ID);
    assert_eq!(
        g.struct_at(&Path::parse("ClassList[0]").unwrap())
            .unwrap()
            .id,
        2
    );
    for (bad, at) in [
        ("Nope", ""),
        ("ClassList[1].Class", "ClassList"),
        ("Tag.X", "Tag"),
        ("Tag[0]", "Tag"),
        ("ClassList[0]", "ClassList[0]"),
    ] {
        let e = g.get(&Path::parse(bad).unwrap()).unwrap_err();
        assert_eq!(e.code(), codes::NO_SUCH_PATH, "{bad}");
        match e {
            Error::NoSuchPath { at: a, .. } => assert_eq!(a, at, "{bad}"),
            other => panic!("{other}"),
        }
    }
}

#[test]
fn every_type_round_trips() {
    let mut g = Gff::new(*b"GFF ");
    let mut inner = Struct::new(1);
    inner.push(label("Inner"), Value::Byte(1));
    let mut loc = LocString::none();
    loc.set(0, "Hello");
    loc.set(3, "Bonjour");
    g.root
        .push(label("Byte"), Value::Byte(255))
        .push(label("Char"), Value::Char(-1))
        .push(label("Word"), Value::Word(65535))
        .push(label("Short"), Value::Short(-2))
        .push(label("Dword"), Value::Dword(0xFFFF_FFFF))
        .push(label("Int"), Value::Int(-3))
        .push(label("Dword64"), Value::Dword64(u64::MAX))
        .push(label("Int64"), Value::Int64(i64::MIN))
        .push(label("Float"), Value::Float(f32::from_bits(0x7FC0_0001)))
        .push(label("Double"), Value::Double(-0.0))
        .push(label("String"), Value::String(vec![0, 1, 2, 255]))
        .push(label("ResRef"), Value::resref("sixteen_chars_ok").unwrap())
        .push(label("LocString"), Value::LocString(loc))
        .push(label("Void"), Value::Void(vec![9; 33]))
        .push(label("Struct"), Value::Struct(inner))
        .push(
            label("List"),
            Value::List(vec![Struct::new(5), Struct::new(6)]),
        )
        .push(
            label("Orientation"),
            Value::Orientation([1.0, 0.0, 0.0, 0.0]),
        )
        .push(label("Vector"), Value::Vector([1.0, 2.0, 3.0]))
        .push(label("StrRef"), Value::StrRef(123));
    let bytes = g.to_bytes().unwrap();
    let back = Gff::parse(&bytes).unwrap();
    assert_eq!(back, g);
    assert_eq!(back.to_bytes().unwrap(), bytes);
    assert_eq!(
        back.get(&Path::parse("LocString").unwrap()).unwrap(),
        g.get(&Path::parse("LocString").unwrap()).unwrap()
    );
    // A struct with no fields writes a zero offset and reads back empty.
    assert_eq!(
        back.struct_at(&Path::parse("List[1]").unwrap())
            .unwrap()
            .fields
            .len(),
        0
    );
}

#[test]
fn labels_are_shared_across_structs_and_kept_in_table_order() {
    let g = creature("Bastila", 24, 3);
    let mut back = Gff::parse(&g.to_bytes().unwrap()).unwrap();
    assert_eq!(back.labels.len(), 9);
    // Two more fields with a label already present, and one new.
    if let Value::List(items) = back.root.get_mut("ClassList").unwrap() {
        let mut c = Struct::new(2);
        c.push(label("Class"), Value::Int(1))
            .push(label("ClassLevel"), Value::Short(1))
            .push(label("Extra"), Value::Byte(0));
        items.push(c);
    }
    let bytes = back.to_bytes().unwrap();
    let again = Gff::parse(&bytes).unwrap();
    assert_eq!(again.labels.len(), 10);
    assert!(again.labels[9].is("Extra"));
    assert_eq!(u32_at(&bytes, 28), 10);
}

#[test]
fn the_reader_refuses_what_it_should_with_the_registered_codes() {
    let g = creature("Bastila", 24, 3);
    let bytes = g.to_bytes().unwrap();

    let e = Gff::parse(b"RIM V1.0").unwrap_err();
    assert_eq!(e.code(), codes::NOT_GFF);
    assert_eq!(e.code(), "RE0601");

    let e = Gff::parse(&bytes[..bytes.len() - 1]).unwrap_err();
    assert_eq!(e.code(), codes::TRUNCATED);

    let mut cut = bytes.clone();
    let dofs = u32_at(&bytes, 32) as usize;
    cut[dofs..dofs + 4].copy_from_slice(&1000u32.to_le_bytes()); // the resref's size byte and beyond
    let e = Gff::parse(&cut).unwrap_err();
    assert!(
        matches!(e.code(), codes::TRUNCATED | codes::TOO_LONG),
        "{e}"
    );

    let mut bad_label = bytes.clone();
    let fo = u32_at(&bytes, 16) as usize;
    bad_label[fo + 4..fo + 8].copy_from_slice(&99u32.to_le_bytes());
    assert_eq!(Gff::parse(&bad_label).unwrap_err().code(), codes::BAD_INDEX);

    let mut bad_type = bytes.clone();
    bad_type[fo..fo + 4].copy_from_slice(&19u32.to_le_bytes());
    assert_eq!(
        Gff::parse(&bad_type).unwrap_err().code(),
        codes::UNKNOWN_TYPE
    );

    // The list's one struct index pointed back at the root: a cycle.
    let mut cycle = bytes.clone();
    let lio = u32_at(&bytes, 48) as usize;
    cycle[lio + 4..lio + 8].copy_from_slice(&0u32.to_le_bytes());
    let e = Gff::parse(&cycle).unwrap_err();
    assert_eq!(e.code(), codes::BAD_INDEX);

    assert_eq!(
        Label::new("seventeen_chars__").unwrap_err().code(),
        codes::TOO_LONG
    );
    assert_eq!(
        Value::resref("seventeen_chars__").unwrap_err().code(),
        codes::TOO_LONG
    );
}

#[test]
fn a_file_edited_in_shape_keeps_its_slots_and_appends() {
    // Build, write, read: every struct and field now has a file position.
    let g = creature("Bastila", 24, 3);
    let mut read = Gff::parse(&g.to_bytes().unwrap()).unwrap();
    // Add a root field: the root's index list no longer fits where it was
    // (seven entries at 0, followed by the class struct's at 28), so it is
    // appended and the old list stays as stale bytes, as the engine's own
    // writer leaves it.
    read.root.push(label("Gender"), Value::Byte(0));
    let bytes = read.to_bytes().unwrap();
    let fic = u32_at(&bytes, 44);
    assert_eq!(fic, 7 * 4 + 2 * 4 + 8 * 4);
    let so = u32_at(&bytes, 8) as usize;
    assert_eq!(u32_at(&bytes, so + 4), 36);
    assert_eq!(u32_at(&bytes, so + 8), 8);
    // The new field is last in the field array.
    let fc = u32_at(&bytes, 20);
    assert_eq!(fc, 10);
    let fo = u32_at(&bytes, 16) as usize;
    assert_eq!(u32_at(&bytes, fo + 9 * 12), FieldType::Byte.id());
    let back = Gff::parse(&bytes).unwrap();
    assert!(back.semantic_eq(&read));
    let s = rakata_gff::survey(&bytes).unwrap();
    assert_eq!(s.field_indices_bytes - s.field_indices_used, 28);
    assert_eq!(
        s.to_string(),
        "gff: UTC V3.2; sections adjacent; data: all used; field indices: 28 unused bytes; list indices: all used; locstring totals: all right; narrow fields: clean"
    );
}
