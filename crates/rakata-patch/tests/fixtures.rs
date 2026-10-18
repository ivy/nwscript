//! The reader over the hand-written fixtures: one per section, one per
//! defect of modshop's patch series that was reader semantics.

use rakata_patch::changes::*;
use rakata_patch::plan::{Operation, Phase};
use rakata_patch::{Changes, Ini, Issue, Plan, Value, codes, load};
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn parse(name: &str) -> (Changes, Vec<Issue>) {
    let bytes = std::fs::read(fixtures().join(name)).unwrap();
    let mut issues = Vec::new();
    let ini = Ini::parse(&bytes, 0, &mut issues);
    let changes = Changes::parse(&ini, 0, &mut issues);
    (changes, issues)
}

fn codes_of(issues: &[Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.code).collect()
}

#[test]
fn settings_are_typed_and_an_unknown_key_is_kept_and_reported() {
    let (c, issues) = parse("settings.ini");
    let s = &c.settings;
    assert_eq!(s.window_caption.as_deref(), Some("Fixture: settings"));
    assert_eq!(s.log_level, Some(3));
    assert_eq!(s.installer_mode, Some(true));
    assert_eq!(s.plaintext_log, Some(false));
    assert_eq!(s.lookup_game_number, Some(1));
    assert_eq!(s.required.len(), 1);
    assert_eq!(s.required[0].file, "k_fp_barrier1.ncs");
    assert!(
        s.required[0]
            .message
            .as_deref()
            .unwrap()
            .starts_with("\"K2 Force Powers")
    );
    assert_eq!(s.unknown.len(), 1);
    assert_eq!(s.unknown[0].key, "Frobnicate");
    assert_eq!(codes_of(&issues), [codes::UNKNOWN_KEY]);
    assert_eq!(issues[0].line, 15);
}

#[test]
fn a_lone_required_msg_and_a_lone_empty_required_parse_and_require_nothing() {
    for name in ["required-msg-only.ini", "required-empty.ini"] {
        let (c, issues) = parse(name);
        assert!(issues.is_empty(), "{name}: {issues:?}");
        assert!(c.settings.required.is_empty(), "{name}");
        assert_eq!(c.install.len(), 1, "{name}");
        let mut plan_issues = Vec::new();
        let plan = Plan::build(&c, 0, &mut plan_issues);
        assert!(plan.requirements.is_empty());
        assert_eq!(plan.steps.len(), 1);
    }
}

#[test]
fn tlk_list_has_appends_and_replacement_files_in_both_spellings() {
    let (c, issues) = parse("tlk.ini");
    assert!(issues.is_empty(), "{issues:?}");
    let t = &c.tlk;
    assert_eq!(
        t.appends
            .iter()
            .map(|a| (a.token, a.index))
            .collect::<Vec<_>>(),
        [(0, 0), (1, 1), (2, 2)]
    );
    assert_eq!(t.replacements.len(), 2);
    assert_eq!(t.replacements[0].file, "append.tlk");
    assert_eq!(
        t.replacements[0]
            .entries
            .iter()
            .map(|e| (e.strref, e.index))
            .collect::<Vec<_>>(),
        [(25859, 27), (45953, 29)]
    );
    assert_eq!(t.replacements[1].key, "ReplaceFile1");
    assert_eq!(
        t.replacements[1]
            .entries
            .iter()
            .map(|e| (e.strref, e.index))
            .collect::<Vec<_>>(),
        [(223, 0), (246, 1)]
    );
}

#[test]
fn install_folders_are_named_by_key_and_files_carry_their_flag_and_rename() {
    let (c, issues) = parse("install.ini");
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(c.install.len(), 4);
    let f0 = &c.install[0];
    assert_eq!(
        (f0.key.as_str(), f0.destination.as_str()),
        ("install_folder0", "Override")
    );
    assert_eq!(
        f0.files
            .iter()
            .map(|f| (f.name.as_str(), f.replace))
            .collect::<Vec<_>>(),
        [("k2_shield1.tpc", false), ("v_superbike.mdl", true)]
    );
    assert_eq!(c.install[2].destination, "modules\\tar_m02af.mod");
    assert_eq!(
        c.install[2].files[0].save_as.as_deref(),
        Some("k_pebn_pophawk.ncs")
    );
    assert_eq!(c.install[3].destination, "StreamVoice\\m12aa");
    let mut pi = Vec::new();
    let plan = Plan::build(&c, 0, &mut pi);
    let renamed = plan
        .phase(Phase::Install)
        .find_map(|s| match &s.op {
            Operation::InstallFile {
                source,
                name,
                destination,
                ..
            } if source != name => Some((source.clone(), name.clone(), destination.clone())),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        renamed,
        (
            "TAR_k_pebn_pophawk.ncs".to_string(),
            "k_pebn_pophawk.ncs".to_string(),
            "modules\\tar_m02af.mod".to_string()
        )
    );
}

#[test]
fn every_2da_modifier_row_selection_special_value_and_token() {
    let (c, issues) = parse("2da.ini");
    assert_eq!(codes_of(&issues), [codes::UNKNOWN_KEY], "{issues:?}");
    assert!(issues[0].message.contains("Frob0"));
    assert_eq!(c.tables.len(), 3);
    let spells = &c.tables[0];
    assert_eq!(spells.file, "spells.2da");
    assert_eq!(spells.ops.len(), 4);
    let TableOpKind::AddRow(add) = &spells.ops[0].kind else {
        panic!("{:?}", spells.ops[0]);
    };
    assert_eq!(add.exclusive_column.as_deref(), Some("label"));
    assert_eq!(add.row_label, None);
    assert_eq!(
        add.stores,
        [Store {
            slot: 2,
            source: StoreSource::RowIndex
        }]
    );
    let by_col = |col: &str| {
        add.cells
            .iter()
            .find(|c| c.column == col)
            .unwrap()
            .value
            .clone()
    };
    assert_eq!(by_col("name"), Value::StrRef(0));
    assert_eq!(by_col("forcepriority"), Value::High);
    assert_eq!(by_col("prereqfeat1"), Value::Memory(1));
    assert_eq!(
        by_col("impactscript"),
        Value::Literal("k_fp_barrier1".into())
    );
    let TableOpKind::ChangeRow(ch) = &spells.ops[1].kind else {
        panic!();
    };
    assert_eq!(ch.target, Some(RowTarget::Index(12)));
    assert_eq!(
        ch.stores,
        [Store {
            slot: 3,
            source: StoreSource::Column("label".into())
        }]
    );
    assert_eq!(ch.cells[1].value, Value::Literal("****".into()));
    let TableOpKind::CopyRow(cp) = &spells.ops[2].kind else {
        panic!();
    };
    assert_eq!(
        cp.target,
        Some(RowTarget::LabelIndex("FORCE_POWER_HEAL".into()))
    );
    assert_eq!(cp.new_row_label, Some(Value::High));
    assert_eq!(cp.exclusive_column.as_deref(), Some("label"));
    let TableOpKind::AddColumn(ac) = &spells.ops[3].kind else {
        panic!();
    };
    assert_eq!(ac.label, "newcol");
    assert_eq!(ac.default, Some(Value::Literal("****".into())));
    assert_eq!(ac.cells[0].row, RowRef::Index(0));
    assert_eq!(ac.cells[1].row, RowRef::Label("some_label".into()));
    assert_eq!(
        ac.stores,
        [ColumnStore {
            slot: 5,
            row: RowRef::Index(0)
        }]
    );
    let appearance = &c.tables[1];
    let TableOpKind::ChangeRow(ch) = &appearance.ops[0].kind else {
        panic!();
    };
    assert_eq!(ch.target, Some(RowTarget::Label("205".into())));
    let TableOpKind::AddRow(add) = &appearance.ops[1].kind else {
        panic!();
    };
    assert_eq!(add.row_label, Some(Value::High));
    assert_eq!(c.tables[2].unknown.len(), 1);
}

#[test]
fn gff_edits_additions_and_nesting() {
    let (c, issues) = parse("gff.ini");
    // The second AddField of p_bastilla.utc names a section that is not there.
    assert_eq!(codes_of(&issues), [codes::MISSING_SECTION], "{issues:?}");
    assert_eq!(c.gff.len(), 3);
    let senni = &c.gff[0];
    assert_eq!(senni.destination.as_deref(), Some("modules\\tat_m18aa.mod"));
    assert!(!senni.replace_file);
    assert_eq!(senni.edits.len(), 4);
    assert_eq!(senni.edits[0].target, FieldTarget::Substring(0));
    assert_eq!(senni.edits[0].value, Value::Literal("Senni Vek".into()));
    assert_eq!(senni.edits[1].target, FieldTarget::StrRef);
    assert_eq!(senni.edits[1].value, Value::StrRef(0));
    assert_eq!(
        senni.edits[2].path,
        FieldPath::Literal("Appearance_Type".into())
    );
    assert_eq!(senni.edits[2].value, Value::Memory(1));
    // A 2DAMEMORY# key at file level is a path held in the token, not a store.
    assert_eq!(senni.edits[3].path, FieldPath::Memory(7));
    let dlg = &c.gff[1];
    assert!(dlg.replace_file);
    assert_eq!(dlg.filename.as_deref(), Some("tar02_bastvision.dlg"));
    assert_eq!(
        dlg.edits[0].path,
        FieldPath::Literal("EntryList\\3\\Text".into())
    );
    assert_eq!(dlg.edits[0].value, Value::Literal("41901".into()));
    assert_eq!(dlg.adds.len(), 1);
    let list = &dlg.adds[0];
    assert_eq!(list.field_type, FieldType::List);
    assert_eq!(list.path, None, "an empty Path is no path");
    let st = &list.children[0];
    assert_eq!(st.field_type, FieldType::Struct);
    assert_eq!(st.type_id, Some(0));
    assert_eq!(
        st.stores,
        [AddStore {
            slot: 50,
            what: AddStoreWhat::ListIndex
        }]
    );
    assert_eq!(st.children.len(), 3);
    let text = &st.children[0];
    assert_eq!(text.field_type, FieldType::ExoLocString);
    assert_eq!(text.strref, Some(Value::Literal("-1".into())));
    assert_eq!(text.substrings, [(0, "Hello there.".to_string())]);
    assert_eq!(st.children[1].field_type, FieldType::Position);
    assert_eq!(st.children[2].field_type, FieldType::Orientation, "by id");
    let feat = &c.gff[2].adds[0];
    assert_eq!(feat.path.as_deref(), Some("FeatList"));
    assert_eq!(feat.children[0].value, Some(Value::Memory(2)));
    // The ListIndex store at line 28 comes before the read at line 24? No:
    // the edit reading 2DAMEMORY50 is listed after the AddField that sets it.
    let mut pi = Vec::new();
    let plan = Plan::build(&c, 0, &mut pi);
    let unset: Vec<&str> = pi.iter().map(|i| i.message.as_str()).collect();
    assert!(
        unset.iter().all(|m| !m.contains("2DAMEMORY50")),
        "{unset:?}"
    );
    assert_eq!(plan.phase(Phase::Gff).count(), 3);
}

#[test]
fn hack_list_offsets_values_and_flags() {
    let (c, issues) = parse("hack.ini");
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(c.hacks.len(), 2);
    let h = &c.hacks[0];
    assert!(!h.replace_file);
    assert_eq!(h.destination.as_deref(), Some("override"));
    assert_eq!(
        h.writes
            .iter()
            .map(|w| (w.offset, w.value.clone()))
            .collect::<Vec<_>>(),
        [(35, Value::Literal("1".into())), (0x30, Value::StrRef(0))]
    );
    assert!(c.hacks[1].replace_file, "Replace# sets the flag");
}

#[test]
fn compile_list_carries_the_replace_flag_and_destinations() {
    let (c, issues) = parse("compile.ini");
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(c.compile.default_destination.as_deref(), Some("override"));
    assert_eq!(
        c.compile
            .scripts
            .iter()
            .map(|s| (s.file.as_str(), s.replace))
            .collect::<Vec<_>>(),
        [
            ("k_fp_barrier1.nss", true),
            ("k_punk_bastatt.nss", false),
            ("k_sta_bastatt.nss", true)
        ]
    );
    assert_eq!(
        c.compile.scripts[1].destination.as_deref(),
        Some("modules\\unk_m44ac.mod")
    );
    let mut pi = Vec::new();
    let plan = Plan::build(&c, 0, &mut pi);
    let dests: Vec<(String, String, bool)> = plan
        .phase(Phase::Compile)
        .map(|s| match &s.op {
            Operation::Compile {
                destination,
                name,
                replace,
                ..
            } => (destination.clone(), name.clone(), *replace),
            _ => panic!(),
        })
        .collect();
    assert_eq!(
        dests,
        [
            (
                "override".to_string(),
                "k_fp_barrier1.ncs".to_string(),
                true
            ),
            (
                "modules\\unk_m44ac.mod".to_string(),
                "k_punk_bastatt.ncs".to_string(),
                false
            ),
            (
                "override".to_string(),
                "k_sta_bastatt.ncs".to_string(),
                true
            ),
        ]
    );
}

#[test]
fn ssf_list_sounds_and_tokens() {
    let (c, issues) = parse("ssf.ini");
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(c.ssf.len(), 2);
    assert!(c.ssf[0].replace);
    assert!(!c.ssf[1].replace);
    assert_eq!(c.ssf[0].entries[0].sound, "Battlecry 1");
    assert_eq!(c.ssf[0].entries[0].value, Value::StrRef(12));
    assert_eq!(c.ssf[0].entries[3].value, Value::Literal("12345".into()));
}

#[test]
fn a_row_naming_a_missing_section_warns_and_the_plan_skips_it() {
    let (c, issues) = parse("missing-section.ini");
    assert_eq!(
        codes_of(&issues),
        [codes::MISSING_SECTION, codes::MISSING_SECTION]
    );
    assert!(issues.iter().all(|i| !i.is_error()));
    assert_eq!((issues[0].line, issues[1].line), (9, 10));
    assert!(issues[0].message.contains("[repeating_blaster]"));
    assert_eq!(c.tables[0].ops.len(), 1);
    let mut pi = Vec::new();
    let plan = Plan::build(&c, 0, &mut pi);
    assert_eq!(plan.steps.len(), 1);
    assert!(pi.is_empty());
}

#[test]
fn the_plan_runs_the_phases_in_the_patchers_order_whatever_the_files_order() {
    let (c, issues) = parse("full/changes.ini");
    assert!(issues.is_empty(), "{issues:?}");
    let mut pi = Vec::new();
    let plan = Plan::build(&c, 0, &mut pi);
    assert!(
        pi.is_empty(),
        "every token is set before it is read: {pi:?}"
    );
    let phases: Vec<Phase> = plan.steps.iter().map(|s| s.phase).collect();
    assert_eq!(
        phases,
        [
            Phase::Tlk,
            Phase::Install,
            Phase::TwoDa,
            Phase::Gff,
            Phase::Hack,
            Phase::Compile,
            Phase::Ssf
        ]
    );
    let text = plan.to_string();
    let heads: Vec<&str> = text.lines().filter(|l| l.starts_with('[')).collect();
    assert_eq!(
        heads,
        [
            "[TLKList]",
            "[InstallList]",
            "[2DAList]",
            "[GFFList]",
            "[HACKList]",
            "[CompileList]",
            "[SSFList]"
        ]
    );
    assert!(text.contains("Appearance_Type = 2DAMEMORY1"), "{text}");
    let json = serde_json::to_value(&plan).unwrap();
    assert_eq!(json["steps"][3]["phase"], "gff");
    assert_eq!(json["steps"][2]["op"], "table");
}

#[test]
fn a_token_read_before_it_is_set_is_a_warning_at_the_read() {
    let mut issues = Vec::new();
    let ini = Ini::parse(
        b"[GFFList]\nFile0=a.utc\n[a.utc]\nAppearance_Type=2DAMEMORY9\n[2DAList]\nTable0=x.2da\n[x.2da]\nAddRow0=r\n[r]\nlabel=x\n2DAMEMORY9=RowIndex\n",
        0,
        &mut issues,
    );
    let c = Changes::parse(&ini, 0, &mut issues);
    let _ = Plan::build(&c, 0, &mut issues);
    assert!(
        issues.is_empty(),
        "2DA runs before GFF, so the token is set: {issues:?}"
    );
    let ini = Ini::parse(
        b"[GFFList]\nFile0=a.utc\n[a.utc]\nAppearance_Type=2DAMEMORY9\n",
        0,
        &mut issues,
    );
    let c = Changes::parse(&ini, 0, &mut issues);
    let _ = Plan::build(&c, 0, &mut issues);
    assert_eq!(codes_of(&issues), [codes::TOKEN_UNSET]);
    assert_eq!(issues[0].line, 4);
}

#[test]
fn the_dialect_edges_each_get_their_code_and_the_first_wins() {
    let (c, issues) = parse("dialect.ini");
    assert_eq!(
        c.settings.log_level,
        Some(3),
        "the first LogLevel, case-insensitively"
    );
    assert_eq!(c.settings.window_caption.as_deref(), Some("a=b"));
    assert_eq!(c.gff[0].edits.len(), 1);
    assert_eq!(c.gff[0].edits[0].value, Value::Literal("first".into()));
    let mut codes = codes_of(&issues);
    codes.sort_unstable();
    assert_eq!(
        codes,
        [
            codes::MALFORMED_LINE,
            codes::MALFORMED_LINE,
            codes::DUPLICATE,
            codes::DUPLICATE
        ]
    );
}

#[test]
fn a_namespace_without_ininame_defaults_to_changes_ini_and_the_option_is_loaded() {
    let dir = fixtures().join("options");
    let err = load(&dir, None).unwrap_err();
    assert!(err.to_string().contains("--option"), "{err}");
    let loaded = load(&dir, Some(1)).unwrap();
    assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
    let o = loaded.option.as_ref().unwrap();
    assert_eq!(
        (o.ini_name.as_str(), o.info_name.as_str()),
        ("changes.ini", "info.rtf")
    );
    assert!(loaded.sources[1].path.ends_with("alpha/changes.ini"));
    assert_eq!(loaded.changes.install[0].files[0].name, "alpha.tpc");
    let loaded = load(&dir, Some(2)).unwrap();
    assert!(loaded.sources[1].path.ends_with("beta/beta_changes.ini"));
    assert!(loaded.changes.install[0].files[0].replace);
    let err = load(&dir, Some(3)).unwrap_err();
    assert!(err.to_string().contains("no option 3"), "{err}");
}

#[test]
fn a_folder_without_namespaces_loads_changes_ini_directly() {
    let loaded = load(&fixtures().join("full"), None).unwrap();
    assert!(loaded.namespaces.is_none());
    assert_eq!(loaded.sources.len(), 1);
    assert_eq!(
        loaded.changes.settings.window_caption.as_deref(),
        Some("Fixture: full")
    );
}
