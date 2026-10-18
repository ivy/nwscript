//! Hand-made archives, a few bytes each, built with the API: the add and
//! replace cases, and the regression this crate exists to make impossible.
//!
//! The defect: a capsule writer that set the new resource first and copied
//! every existing resource over it afterwards, so a replace into a `.mod`
//! that already held the resource wrote the old bytes back while its log
//! said it had replaced them (modshop, `games/kotor/patches/README.md`,
//! item 6). The tests here re-read what was written and look at the bytes.

use rakata_archive::{Bif, BifMember, Erf, ErfKind, Rim, codes};
use rakata_res::{ContentHash, ResId, ResType};

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

    let mut read = Erf::parse(&before).unwrap();
    let new_bytes = b"NEW juhani bytecode, longer than before".to_vec();
    read.replace(id("K_PDAN_JUHANI31.ncs"), new_bytes.clone())
        .unwrap();
    let after = read.to_bytes().unwrap();

    let reread = Erf::parse(&after).unwrap();
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

    let mut read = Erf::parse(&bytes).unwrap();
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
    assert_eq!(Erf::parse(&written).unwrap().to_bytes().unwrap(), written);
}

#[test]
fn a_rim_after_replace_and_add_equals_the_same_archive_built_fresh() {
    let mut rim = Rim::new();
    rim.add(id("m01aa.are"), vec![1; 5]).unwrap();
    rim.add(id("m01aa.git"), vec![2; 9]).unwrap();
    let bytes = rim.to_bytes().unwrap();

    let mut read = Rim::parse(&bytes).unwrap();
    read.replace(id("M01AA.are"), vec![3; 2]).unwrap();
    read.add(id("m01aa.ifo"), vec![4; 4]).unwrap();
    let written = read.to_bytes().unwrap();

    let mut fresh = Rim::new();
    fresh.add(id("m01aa.are"), vec![3; 2]).unwrap();
    fresh.add(id("m01aa.git"), vec![2; 9]).unwrap();
    fresh.add(id("m01aa.ifo"), vec![4; 4]).unwrap();
    assert_eq!(written, fresh.to_bytes().unwrap());

    let reread = Rim::parse(&written).unwrap();
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

    let mut read = Bif::parse(&bytes).unwrap();
    read.replace(0, vec![9, 9, 9, 9, 9]).unwrap();
    let written = read.to_bytes().unwrap();

    let reread = Bif::parse(&written).unwrap();
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
    let mut read = Erf::parse(&bytes).unwrap();
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
