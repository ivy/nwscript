use nwscript_diag::{FileId, SourceMap};
use nwscript_sema::{ConstValue, EngineTable, Type};
use std::path::PathBuf;

const SMALL: &str = "#define ENGINE_NUM_STRUCTURES 2\r\n#define ENGINE_STRUCTURE_0 effect\r\n#define ENGINE_STRUCTURE_1 location\r\n\r\nint TRUE = 1;\r\nint FALSE = 0;\r\nint PLAYER_IS_PC = TRUE;\r\nfloat PI = 3.14;\r\nint NEG = -2;\r\n\r\n// Returns a random number.\r\n// From 0 up to nMax - 1.\r\nint Random(int nMax);\r\n\r\nvoid Print(string s, int n = NEG, float f = 0.0, object o = OBJECT_SELF, vector v = [1.0, 2.0, 3.0]);\r\neffect EffectHeal(int n);\r\nvoid Move(location l, action a);\r\nvoid Typo(INT n, OBJECT_ID o);\r\n";

#[test]
fn reads_functions_constants_and_structures() {
    let (t, diags) = EngineTable::from_nwscript(FileId(0), SMALL.as_bytes());
    assert_eq!(t.structures, ["effect", "location"]);
    assert_eq!(
        t.constant("PLAYER_IS_PC").unwrap().value,
        ConstValue::Int(1)
    );
    assert_eq!(t.constant("NEG").unwrap().value, ConstValue::Int(-2));
    let (n, random) = t.function("Random").unwrap();
    assert_eq!(n, 0);
    assert_eq!(
        random.doc,
        "Returns a random number.\nFrom 0 up to nMax - 1."
    );
    let (_, print) = t.function("Print").unwrap();
    let defaults: Vec<Option<ConstValue>> =
        print.params.iter().map(|p| p.default.clone()).collect();
    assert_eq!(
        defaults,
        [
            None,
            Some(ConstValue::Int(-2)),
            Some(ConstValue::Float(0.0)),
            Some(ConstValue::Object(0)),
            Some(ConstValue::Vector([1.0, 2.0, 3.0]))
        ]
    );
    assert_eq!(t.function("EffectHeal").unwrap().1.ret, Type::Engine(0));
    let (_, mv) = t.function("Move").unwrap();
    assert_eq!(mv.params[0].ty, Type::Engine(1));
    assert_eq!(mv.params[1].ty, Type::Action);
    // One diagnostic for the typo line, both names labelled; the slot is kept.
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0].code.id, "E0301");
    assert_eq!(diags[0].labels.len(), 2);
    let (n, typo) = t.function("Typo").unwrap();
    assert_eq!(n, 4);
    assert!(typo.recovered);
    assert_eq!(
        typo.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
        [Type::Int, Type::Object]
    );
}

fn corpus() -> Option<PathBuf> {
    let dir = match std::env::var_os("NWSCRIPT_CORPUS") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".cache/nwscript/corpus/k1"),
    };
    dir.join("nwscript.nss").is_file().then_some(dir)
}

#[test]
#[ignore = "needs the corpus: set NWSCRIPT_CORPUS"]
fn k1_nwscript() {
    let Some(root) = corpus() else {
        eprintln!("skipping: no corpus; set NWSCRIPT_CORPUS to an extracted corpus directory");
        return;
    };
    let bytes = std::fs::read(root.join("nwscript.nss")).unwrap();
    let mut map = SourceMap::new();
    let file = map.add("nwscript.nss", bytes.clone());
    let (t, diags) = EngineTable::from_nwscript(file, &bytes);
    println!(
        "k1 nwscript.nss: {} routines, {} constants, structures {:?}",
        t.functions.len(),
        t.constants.len(),
        t.structures
    );
    assert_eq!(t.functions.len(), 772);
    assert_eq!(t.constants.len(), 1490);
    assert_eq!(t.structures, ["effect", "event", "location", "talent"]);
    assert_eq!(diags.len(), 1, "{diags:#?}");
    let line = map
        .get(file)
        .line_col(diags[0].primary_span().unwrap().start)
        .line;
    assert_eq!(line, 5145);
    assert_eq!(t.function("SetAvailableNPCId").map(|f| f.0), Some(767));
    assert_eq!(t.function("Random").map(|f| f.0), Some(0));
    assert_eq!(t.function("PrintString").map(|f| f.0), Some(1));
    assert!(t.functions.iter().filter(|f| f.recovered).count() == 1);
    println!(
        "{}",
        nwscript_diag::render::render(&diags[0], &map, nwscript_diag::render::Color::Never)
    );
}
