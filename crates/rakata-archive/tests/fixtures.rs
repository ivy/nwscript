//! Hand-made archives, a few bytes each, built with the API: the add and
//! replace cases, and the regression this crate exists to make impossible.
//!
//! The defect: a capsule writer that set the new resource first and copied
//! every existing resource over it afterwards, so a replace into a `.mod`
//! that already held the resource wrote the old bytes back while its log
//! said it had replaced them (modshop, `games/kotor/patches/README.md`,
//! item 6). The tests here re-read what was written and look at the bytes.

use rakata_archive::{
    Bif, BifMember, BifRef, Description, Erf, ErfKind, Key, KeyEntry, Rim, codes,
};
use rakata_res::{ContentHash, ResId, ResType};

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

fn id(text: &str) -> ResId {
    text.parse().unwrap()
}

/// The scenario: a `.mod` holding `k_pdan_juhani31.ncs`, one member
/// replaced, re-read; the member's bytes are the new ones and every other
/// member's hash is unchanged.
#[test]
fn a_replaced_member_is_what_the_mod_holds() {
    let mut original = Erf::new(ErfKind::Mod);
    original.build_year = 103;
    original.build_day = 309;
    original
        .add(id("k_pdan_juhani31.ncs"), b"old juhani bytecode".to_vec())
        .unwrap();
    original
        .add(id("danm13.are"), b"the area, untouched".to_vec())
        .unwrap();
    original
        .add(id("k_pdan_bastila.ncs"), b"bastila, untouched".to_vec())
        .unwrap();
    let before = original.to_bytes().unwrap();
    let untouched: Vec<(ResId, ContentHash)> = original
        .members()
        .iter()
        .filter(|m| m.id != id("k_pdan_juhani31.ncs"))
        .map(|m| (m.id, ContentHash::of(m.data())))
        .collect();

    let mut read = Erf::parse(seed(&before)).unwrap();
    let new_bytes = b"NEW juhani bytecode, longer than before".to_vec();
    read.replace(id("K_PDAN_JUHANI31.ncs"), new_bytes.clone())
        .unwrap();
    let after = read.to_bytes().unwrap();

    let reread = Erf::parse(seed(&after)).unwrap();
    assert_eq!(reread.get(id("k_pdan_juhani31.ncs")), Some(&new_bytes[..]));
    assert_eq!(reread.len(), 3);
    for (member, hash) in &untouched {
        assert_eq!(
            ContentHash::of(reread.get(*member).unwrap()),
            *hash,
            "{member}"
        );
    }
    // The stored spelling is the one read, not the one the replace used.
    assert_eq!(reread.members()[0].id.name.to_string(), "k_pdan_juhani31");
    assert_ne!(before, after);
}

/// Writing the result of a replace again is byte-identical to building
/// the same archive fresh.
#[test]
fn an_erf_after_replace_equals_the_same_archive_built_fresh() {
    let mut erf = Erf::new(ErfKind::Erf);
    erf.add(id("one.txt"), b"first".to_vec()).unwrap();
    erf.add(id("two.txt"), b"second".to_vec()).unwrap();
    let bytes = erf.to_bytes().unwrap();

    let mut read = Erf::parse(seed(&bytes)).unwrap();
    read.replace(id("two.txt"), b"replacement".to_vec())
        .unwrap();
    assert_eq!(read.get(id("two.txt")), Some(&b"replacement"[..]));
    assert_eq!(read.get(id("one.txt")), Some(&b"first"[..]));
    let written = read.to_bytes().unwrap();

    let mut fresh = Erf::new(ErfKind::Erf);
    fresh.add(id("one.txt"), b"first".to_vec()).unwrap();
    fresh.add(id("two.txt"), b"replacement".to_vec()).unwrap();
    assert_eq!(written, fresh.to_bytes().unwrap());
    // And the same archive, read back, is what was written.
    assert_eq!(
        Erf::parse(seed(&written)).unwrap().to_bytes().unwrap(),
        written
    );
}

#[test]
fn a_rim_after_replace_and_add_equals_the_same_archive_built_fresh() {
    let mut rim = Rim::new();
    rim.add(id("m01aa.are"), vec![1; 5]).unwrap();
    rim.add(id("m01aa.git"), vec![2; 9]).unwrap();
    let bytes = rim.to_bytes().unwrap();

    let mut read = Rim::parse(seed(&bytes)).unwrap();
    read.replace(id("M01AA.are"), vec![3; 2]).unwrap();
    read.add(id("m01aa.ifo"), vec![4; 4]).unwrap();
    let written = read.to_bytes().unwrap();

    let mut fresh = Rim::new();
    fresh.add(id("m01aa.are"), vec![3; 2]).unwrap();
    fresh.add(id("m01aa.git"), vec![2; 9]).unwrap();
    fresh.add(id("m01aa.ifo"), vec![4; 4]).unwrap();
    assert_eq!(written, fresh.to_bytes().unwrap());

    let reread = Rim::parse(seed(&written)).unwrap();
    assert_eq!(reread.get(id("m01aa.are")), Some(&[3, 3][..]));
    assert_eq!(reread.get(id("m01aa.git")), Some(&[2; 9][..]));
    assert_eq!(reread.get(id("m01aa.ifo")), Some(&[4; 4][..]));
    assert_eq!(reread.members().len(), 3);
}

#[test]
fn a_bif_member_replaced_by_index_leaves_the_others() {
    let mut bif = Bif::new();
    bif.push(BifMember::owned(0, ResType::Ncs, vec![1, 2, 3]));
    bif.push(BifMember::owned(
        1,
        ResType::Nss,
        b"void main() {}".to_vec(),
    ));
    let bytes = bif.to_bytes().unwrap();

    let mut read = Bif::parse(seed(&bytes)).unwrap();
    read.replace(0, vec![9, 9, 9, 9, 9]).unwrap();
    let written = read.to_bytes().unwrap();

    let reread = Bif::parse(seed(&written)).unwrap();
    assert_eq!(reread.get(0), Some(&[9; 5][..]));
    assert_eq!(reread.get(1), Some(&b"void main() {}"[..]));
    assert_eq!(reread.members()[1].resource_id, 1);

    let mut fresh = Bif::new();
    fresh.push(BifMember::owned(0, ResType::Ncs, vec![9; 5]));
    fresh.push(BifMember::owned(
        1,
        ResType::Nss,
        b"void main() {}".to_vec(),
    ));
    assert_eq!(written, fresh.to_bytes().unwrap());
}

#[test]
fn add_refuses_a_duplicate_and_replace_refuses_an_absentee() {
    let mut erf = Erf::new(ErfKind::Sav);
    erf.add(id("globalvars.res"), vec![1]).unwrap();
    let e = erf.add(id("GLOBALVARS.res"), vec![2]).unwrap_err();
    assert_eq!(e.code(), codes::ALREADY_PRESENT);
    assert!(e.to_string().starts_with("RE0206: "));
    let e = erf.replace(id("partytable.res"), vec![2]).unwrap_err();
    assert_eq!(e.code(), codes::NOT_PRESENT);
    // The same name under another type is another member.
    erf.add(id("globalvars.ncs"), vec![3]).unwrap();
    assert_eq!(erf.len(), 2);
    assert_eq!(erf.get(id("globalvars.res")), Some(&[1][..]));
}

#[test]
fn members_borrow_from_the_source_until_replaced() {
    let mut erf = Erf::new(ErfKind::Mod);
    erf.add(id("a.ncs"), vec![1]).unwrap();
    erf.add(id("b.ncs"), vec![2]).unwrap();
    let bytes = erf.to_bytes().unwrap();
    let mut read = Erf::parse(seed(&bytes)).unwrap();
    assert!(read.members().iter().all(|m| m.is_borrowed()));
    let range = bytes.as_ptr_range();
    for m in read.members() {
        assert!(
            range.contains(&m.data().as_ptr()),
            "a member outside the source"
        );
    }
    read.replace(id("a.ncs"), vec![7]).unwrap();
    assert!(!read.members()[0].is_borrowed());
    assert!(read.members()[1].is_borrowed());
}

#[test]
fn a_key_over_two_bifs_and_an_erf_with_descriptions_read_back() {
    let mut key = Key::new();
    for (name, size) in [(&b"data\\2da.bif"[..], 42), (b"data\\scripts.bif", 7)] {
        key.add_bif(BifRef {
            size,
            name: name.to_vec(),
            drives: 1,
        });
    }
    for (i, name) in ["appearance.2da", "k_ai_master.ncs"]
        .into_iter()
        .enumerate()
    {
        key.add(KeyEntry {
            id: id(name),
            bif: i as u32,
            index: 3,
        })
        .unwrap();
    }
    let bytes = key.to_bytes().unwrap();
    assert_eq!(Key::parse(seed(&bytes)).unwrap(), key);

    let mut erf = Erf::new(ErfKind::Erf);
    erf.descriptions.push(Description {
        language: 0,
        text: b"A module".to_vec(),
    });
    erf.description_strref = 12;
    erf.add(id("module.ifo"), vec![1, 2, 3]).unwrap();
    let bytes = erf.to_bytes().unwrap();
    assert_eq!(Erf::parse(seed(&bytes)).unwrap(), erf);
}

#[test]
fn an_erf_counting_more_descriptions_than_its_bytes_hold_is_refused_unallocated() {
    // A header of 0x48 bytes repeated: 1.2 billion descriptions, which the
    // reader once reserved room for (39 GB) before reading the first. Found
    // by fuzzing.
    let mut b = b"ERF V1.0".to_vec();
    b.resize(160, 0x48);
    let e = Erf::parse(seed(&b)).unwrap_err();
    assert_eq!(e.code(), codes::TRUNCATED);
}
