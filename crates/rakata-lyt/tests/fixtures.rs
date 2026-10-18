//! Hand-made layouts and visibility files, one for each form the crate
//! reads and each error it reports. The texts are built here; none is a
//! game file.

// 0.707107 is a value as the files spell it, not an approximation of 1/√2.
#![allow(clippy::approx_constant)]

use rakata_lyt::{DoorHook, Error, Lyt, Placement, Quaternion, Vis, VisRoom};

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

/// Lines joined with CRLF, each ended by one.
fn crlf(lines: &[&str]) -> Vec<u8> {
    lines
        .iter()
        .map(|l| format!("{l}\r\n"))
        .collect::<String>()
        .into_bytes()
}

fn layout() -> Vec<u8> {
    crlf(&[
        "#MAXLAYOUT ASCII",
        "filedependancy Test01.max",
        "beginlayout",
        "   roomcount 2",
        "      Test01_a 10.5 -20.25 0.0",
        "      Test01_b 9.76562e-006 100.0 -1.31576",
        "   trackcount 1",
        "      Test01_trk 0.0 0.0 0.0",
        "   obstaclecount 1",
        "      Test01_obs 1.0 2.0 3.0",
        "   doorhookcount 2",
        "      Test01_a Door_01 0 12.0 -20.25 0.0 1.0 0.0 0.0 0.0",
        "      Test01_b Door_02 0 1.0 2.0 3.0 0.707107 0.0 0.0 -0.707107",
        "donelayout",
    ])
}

#[test]
fn a_layout_with_every_section_reads_and_writes_back() {
    let bytes = layout();
    let lyt = Lyt::parse(seed(&bytes)).unwrap();
    assert_eq!(lyt.file_dependency.as_deref(), Some("Test01.max"));
    assert_eq!(
        lyt.rooms,
        [
            Placement::new("Test01_a", [10.5, -20.25, 0.0]),
            Placement::new("Test01_b", [9.76562e-6, 100.0, -1.31576]),
        ]
    );
    assert_eq!(lyt.tracks, [Placement::new("Test01_trk", [0.0; 3])]);
    assert_eq!(
        lyt.obstacles,
        [Placement::new("Test01_obs", [1.0, 2.0, 3.0])]
    );
    assert_eq!(lyt.door_hooks.len(), 2);
    assert_eq!(
        lyt.door_hooks[1],
        DoorHook {
            room: "Test01_b".into(),
            name: "Door_02".into(),
            unknown: 0,
            position: [1.0, 2.0, 3.0],
            orientation: Quaternion {
                w: 0.707107,
                x: 0.0,
                y: 0.0,
                z: -0.707107
            },
        }
    );
    assert_eq!(lyt.door_hooks[0].orientation, Quaternion::IDENTITY);
    assert_eq!(lyt.room("TEST01_B").unwrap().position[1], 100.0);
    assert_eq!(lyt.departures(), 0);
    assert_eq!(lyt.to_bytes().unwrap(), bytes);
}

#[test]
fn a_layout_built_from_nothing_is_written_in_the_vanilla_form() {
    let mut lyt = Lyt::new(Some("Test01.max".into()));
    lyt.rooms
        .push(Placement::new("Test01_a", [10.5, -20.25, 0.0]));
    lyt.rooms
        .push(Placement::new("Test01_b", [9.76562e-6, 100.0, -1.31576]));
    lyt.tracks.push(Placement::new("Test01_trk", [0.0; 3]));
    lyt.obstacles
        .push(Placement::new("Test01_obs", [1.0, 2.0, 3.0]));
    lyt.door_hooks.push(DoorHook {
        room: "Test01_a".into(),
        name: "Door_01".into(),
        unknown: 0,
        position: [12.0, -20.25, 0.0],
        orientation: Quaternion::IDENTITY,
    });
    lyt.door_hooks.push(DoorHook {
        room: "Test01_b".into(),
        name: "Door_02".into(),
        unknown: 0,
        position: [1.0, 2.0, 3.0],
        orientation: Quaternion::from_array([0.707107, 0.0, 0.0, -0.707107]),
    });
    assert_eq!(lyt.to_bytes().unwrap(), layout());
    assert!(lyt.semantic_eq(&Lyt::parse(seed(&layout())).unwrap()));

    let empty = String::from_utf8(Lyt::default().to_bytes().unwrap()).unwrap();
    assert_eq!(
        empty,
        "#MAXLAYOUT ASCII\r\nbeginlayout\r\n   roomcount 0\r\n   trackcount 0\r\n   obstaclecount 0\r\n   doorhookcount 0\r\ndonelayout\r\n"
    );
}

#[test]
fn departures_from_the_written_form_are_kept_and_an_edited_line_is_rewritten() {
    // A tab for the row indent, bytes after donelayout, as two vanilla
    // files have.
    let mut bytes = crlf(&[
        "#MAXLAYOUT ASCII",
        "filedependancy Test02.max",
        "beginlayout",
        "   roomcount 2",
        "\tTest02_a 0.0 0.0 0.0",
        "      Test02_b 1.0 0.0 0.0",
        "   trackcount 0",
        "   obstaclecount 0",
        "   doorhookcount 0",
        "donelayout",
    ]);
    bytes.extend_from_slice(b"Test02_b");
    let mut lyt = Lyt::parse(seed(&bytes)).unwrap();
    assert_eq!(lyt.departures(), 1);
    assert_eq!(lyt.trailer(), "Test02_b");
    assert_eq!(lyt.to_bytes().unwrap(), bytes);

    // Blank lines before donelayout: the bytes after it are still what
    // follows it (found by fuzzing, which read past the lines).
    for trailer in [&b""[..], b"Test02_b\r\nTest02_c"] {
        let mut b = crlf(&[
            "beginlayout",
            "",
            "",
            "   roomcount 0",
            "",
            "   trackcount 0",
            "   obstaclecount 0",
            "   doorhookcount 0",
            "donelayout",
        ]);
        b.extend_from_slice(trailer);
        let l = Lyt::parse(seed(&b)).unwrap();
        assert_eq!(l.trailer().as_bytes(), trailer);
        assert_eq!(l.to_bytes().unwrap(), b);
    }

    lyt.rooms[0].position[0] = 5.0;
    let written = String::from_utf8(lyt.to_bytes().unwrap()).unwrap();
    assert!(
        written.contains("\r\n      Test02_a 5.0 0.0 0.0\r\n"),
        "{written}"
    );
    assert!(!written.contains('\t'));

    // LF endings throughout, keywords in another case, no final ending.
    let lf: &[u8] = b"#MAXLAYOUT ASCII\nBEGINLAYOUT\n   RoomCount 1\n      R 1.0 2.0 3.0\n   trackcount 0\n   obstaclecount 0\n   doorhookcount 0\nDoneLayout";
    let lyt = Lyt::parse(seed(lf)).unwrap();
    assert_eq!(lyt.rooms, [Placement::new("R", [1.0, 2.0, 3.0])]);
    assert_eq!(lyt.to_bytes().unwrap(), lf);
}

#[test]
fn a_layout_is_refused_with_the_code_for_what_is_wrong() {
    let code = |b: &[u8]| Lyt::parse(seed(b)).unwrap_err().code();
    assert_eq!(code(b"#MAXLAYOUT ASCII\r\n"), "RE2301");
    assert_eq!(code(b""), "RE2301");
    assert_eq!(
        code(&crlf(&[
            "beginlayout",
            "   roomcount 1",
            "      R 1.0 2.0",
            "donelayout"
        ])),
        "RE2302"
    );
    assert_eq!(
        code(&crlf(&["filedependancy", "beginlayout", "donelayout"])),
        "RE2302"
    );
    assert_eq!(
        code(&crlf(&["beginlayout", "   walkcount 0", "donelayout"])),
        "RE2302"
    );
    assert_eq!(
        code(&crlf(&[
            "beginlayout",
            "   roomcount 1",
            "      R 1.0 two 3.0",
            "donelayout"
        ])),
        "RE2303"
    );
    assert_eq!(
        code(&crlf(&[
            "beginlayout",
            "   roomcount 1",
            "      R 1.0 nan 3.0",
            "donelayout"
        ])),
        "RE2303"
    );
    assert_eq!(
        code(&crlf(&["beginlayout", "   roomcount -1", "donelayout"])),
        "RE2303"
    );
    let e = Lyt::parse(seed(&crlf(&[
        "beginlayout",
        "   roomcount 2",
        "      R 1.0 2.0 3.0",
        "   doorhookcount 0",
        "donelayout",
    ])))
    .unwrap_err();
    assert_eq!(
        e,
        Error::CountMismatch {
            line: 2,
            section: "roomcount",
            declared: 2,
            found: 1
        }
    );
    assert_eq!(e.code(), "RE2304");
    assert_eq!(code(&crlf(&["beginlayout", "   roomcount 0"])), "RE2305");
    assert_eq!(code(b"beginlayout\r\n\xe9\r\ndonelayout\r\n"), "RE2306");
}

#[test]
fn a_value_the_text_cannot_hold_is_refused_by_the_writer() {
    let mut lyt = Lyt::default();
    lyt.rooms.push(Placement::new("R", [f32::NAN, 0.0, 0.0]));
    assert_eq!(lyt.to_bytes().unwrap_err().code(), "RE2307");
    lyt.rooms[0] = Placement::new("two words", [0.0; 3]);
    assert_eq!(lyt.to_bytes().unwrap_err().code(), "RE2307");
    let vis = Vis::new(vec![VisRoom {
        room: String::new(),
        visible: vec![],
    }]);
    assert_eq!(vis.to_bytes().unwrap_err().code(), "RE2307");
}

#[test]
fn a_visibility_file_reads_and_writes_back() {
    let bytes = crlf(&[
        "test_a 2", "  test_b", "  TEST_c", "test_b 1", "  test_a", "test_c 0",
    ]);
    let vis = Vis::parse(seed(&bytes)).unwrap();
    assert_eq!(vis.rooms.len(), 3);
    assert_eq!(vis.visible("TEST_A").unwrap(), ["test_b", "TEST_c"]);
    assert_eq!(vis.visible("test_c").unwrap(), [] as [&str; 0]);
    assert_eq!(vis.visible("test_d"), None);
    assert_eq!(vis.departures(), 0);
    assert_eq!(vis.to_bytes().unwrap(), bytes);

    let built = Vis::new(vec![
        VisRoom {
            room: "test_a".into(),
            visible: vec!["test_b".into(), "TEST_c".into()],
        },
        VisRoom {
            room: "test_b".into(),
            visible: vec!["test_a".into()],
        },
        VisRoom {
            room: "test_c".into(),
            visible: vec![],
        },
    ]);
    assert_eq!(built.to_bytes().unwrap(), bytes);
    assert!(built.semantic_eq(&vis));
}

#[test]
fn a_visibility_files_departures_are_kept_and_its_rooms_follow_the_indentation() {
    // A count one off, a room line with no count, a trailing space, two
    // spaces before a count, no final line ending: each in a vanilla file.
    let bytes = b"test_a 3\r\n  test_b\r\n  test_c \r\ntest_b\r\ntest_c  1\r\n  test_a";
    let vis = Vis::parse(seed(bytes)).unwrap();
    assert_eq!(vis.visible("test_a").unwrap(), ["test_b", "test_c"]);
    assert_eq!(vis.visible("test_b").unwrap(), [] as [&str; 0]);
    assert_eq!(vis.visible("test_c").unwrap(), ["test_a"]);
    assert_eq!(vis.departures(), 5);
    assert_eq!(vis.to_bytes().unwrap(), bytes);

    let mut edited = vis.clone();
    edited.rooms[0].visible.push("test_d".into());
    let written = String::from_utf8(edited.to_bytes().unwrap()).unwrap();
    assert!(
        written.starts_with("test_a 3\r\n  test_b\r\n  test_c \r\n  test_d\r\n"),
        "{written}"
    );
}

#[test]
fn a_room_seen_with_a_count_after_it_is_a_room_seen() {
    // A mod's visibility file writes the last room seen as `  NAME 5`.
    let bytes = b"test_a 2\r\n  test_b\r\n  test_c 5\r\n";
    let vis = Vis::parse(seed(bytes)).unwrap();
    assert_eq!(vis.visible("test_a").unwrap(), ["test_b", "test_c"]);
    assert_eq!(vis.departures(), 1);
    assert_eq!(vis.to_bytes().unwrap(), bytes);
}

#[test]
fn blank_lines_and_mixed_line_endings_write_back() {
    // A mod's visibility file: LF after some rooms seen, an indented blank
    // line ended CR LF after each, a blank line before the first.
    let bytes = b"\r\ntest_a 2\n  \r\n  test_b\n  \r\n  test_c\r\ntest_b 0\r\n";
    let vis = Vis::parse(seed(bytes)).unwrap();
    assert_eq!(vis.visible("test_a").unwrap(), ["test_b", "test_c"]);
    assert_eq!(vis.to_bytes().unwrap(), bytes);
}

#[test]
fn a_visibility_file_is_refused_with_the_code_for_what_is_wrong() {
    let e = Vis::parse(seed(b"  test_b\r\ntest_a 1\r\n")).unwrap_err();
    assert_eq!(e.code(), "RE2311");
    assert_eq!(
        Vis::parse(seed(b"test_a 1 2\r\n")).unwrap_err().code(),
        "RE2312"
    );
    assert_eq!(
        Vis::parse(seed(b"test_a 1\r\n  test_b test_c\r\n"))
            .unwrap_err()
            .code(),
        "RE2312"
    );
    assert_eq!(
        Vis::parse(seed(b"test_a one\r\n")).unwrap_err().code(),
        "RE2303"
    );
    assert_eq!(
        Vis::parse(seed(b"test_\xff 0\r\n")).unwrap_err().code(),
        "RE2306"
    );
}
