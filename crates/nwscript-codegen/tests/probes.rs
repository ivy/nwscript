//! Code generation probes: small scripts whose bytecode was recorded from
//! nwnnsscomp `-v1.00`, run as a black box. Each pins one encoding rule;
//! where the vanilla corpus uses the construct, its bytes agree, and where
//! it does not, nwnnsscomp's choice is the one `docs/grammar.md` records
//! under Unconfirmed.

use nwscript_codegen::{Profile, generate};
use nwscript_diag::FileId;
use nwscript_ncs::asm::{PrintOptions, print};
use nwscript_sema::{EngineTable, ParsedIncludes, analyze};
use nwscript_syntax::parse;

/// The first eight routines of KotOR's `nwscript.nss`, at their numbers,
/// and two constants.
const NWSCRIPT: &str = "int TRUE = 1;\r\nfloat DIRECTION_NORTH = 90.0;\r\n\
int Random(int nMaxInteger);\r\n\
void PrintString(string sString);\r\n\
void PrintFloat(float fFloat, int nWidth=18, int nDecimals=9);\r\n\
string FloatToString(float fFloat, int nWidth=18, int nDecimals=9);\r\n\
void PrintInteger(int nInteger);\r\n\
void PrintObject(object oObject);\r\n\
void AssignCommand(object oActionSubject,action aActionToAssign);\r\n\
void DelayCommand(float fSeconds, action aActionToDelay);\r\n";

fn compile(src: &str) -> String {
    let (table, d) = EngineTable::from_nwscript(FileId(1), NWSCRIPT.as_bytes());
    assert!(d.is_empty(), "{d:?}");
    let src = src.replace('\n', "\r\n");
    let a = analyze(
        "t.nss",
        parse(FileId(0), src.as_bytes()),
        &table,
        &mut ParsedIncludes::new(),
    );
    assert!(!a.has_errors(), "{:#?}", a.diagnostics);
    let p = generate(&a.program, Profile::K1).unwrap();
    print(
        &p,
        &PrintOptions {
            raw: false,
            routine_name: None,
        },
    )
}

fn probe(src: &str, want: &str) {
    let got = compile(src);
    if got.trim() != want.trim() {
        let g: Vec<&str> = got.lines().collect();
        let w: Vec<&str> = want.trim().lines().collect();
        let i = g
            .iter()
            .zip(&w)
            .position(|(a, b)| a != b)
            .unwrap_or(g.len().min(w.len()));
        panic!(
            "first difference at line {i}:\n ours:   {:?}\n theirs: {:?}\n\nours:\n{got}",
            g.get(i),
            w.get(i)
        );
    }
}

#[test]
fn actions() {
    probe(
        r#"int G = 2;
void Later(int n, string s) { PrintString(s); }
void main()
{
    int x = 3;
    DelayCommand(1.0, Later(x, "s"));
    AssignCommand(OBJECT_SELF, PrintInteger(x));
    if (x) { string t = "t"; DelayCommand(2.0, PrintString(t)); }
}
"#,
        r#"
; 51 instructions, 291 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    CONSTI       2
    CPDOWNSP     -8, 4
    MOVSP        -4
    SAVEBP
    JSR          sub_3d
    RESTOREBP
    MOVSP        -4
    RETN
sub_3d:
    RSADDI
    CONSTI       3
    CPDOWNSP     -8, 4
    MOVSP        -4
    STORE_STATE  4, 4
    JMP          L_78
    CONSTS       "s"
    CPTOPSP      -8, 4
    JSR          sub_10e
    RETN
L_78:
    CONSTF       1.0
    ACTION       7, 2
    STORE_STATE  4, 4
    JMP          L_a2
    CPTOPSP      -4, 4
    ACTION       4, 1
    RETN
L_a2:
    CONSTO       0
    ACTION       6, 2
    CPTOPSP      -4, 4
    JZ           L_106
    RSADDS
    CONSTS       "t"
    CPDOWNSP     -8, 4
    MOVSP        -4
    STORE_STATE  4, 8
    JMP          L_ef
    CPTOPSP      -4, 4
    ACTION       1, 1
    RETN
L_ef:
    CONSTF       2.0
    ACTION       7, 2
    MOVSP        -4
    JMP          L_106
L_106:
    MOVSP        -4
    RETN
sub_10e:
    CPTOPSP      -8, 4
    ACTION       1, 1
    MOVSP        -8
    RETN
"#,
    );
}

#[test]
fn assignexpr() {
    probe(
        r#"void main()
{
    int a, b;
    a = b = 3;
    if ((a = Random(2)) > 0) PrintInteger(a);
    PrintInteger(a == 1 ? 10 : 20);
}
"#,
        r#"
; 27 instructions, 158 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    RSADDI
    CONSTI       3
    CPDOWNSP     -8, 4
    CPDOWNSP     -12, 4
    MOVSP        -4
    CONSTI       2
    ACTION       0, 1
    CPDOWNSP     -12, 4
    CONSTI       0
    GTII
    JZ           L_69
    CPTOPSP      -8, 4
    ACTION       4, 1
    JMP          L_69
L_69:
    CPTOPSP      -8, 4
    CONSTI       1
    EQUALII
    JZ           L_8b
    CONSTI       10
    JMP          L_91
L_8b:
    CONSTI       20
L_91:
    ACTION       4, 1
    MOVSP        -8
    RETN
"#,
    );
}

#[test]
fn calls() {
    probe(
        r#"int Add(int a, int b = 2) { return a + b; }
void Show(string s) { PrintString(s); }
void main()
{
    int n = Add(1);
    Show(FloatToString(1.0));
    Add(n, 3);
}
"#,
        r#"
; 34 instructions, 195 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    RSADDI
    CONSTI       2
    CONSTI       1
    JSR          sub_7a
    CPDOWNSP     -8, 4
    MOVSP        -4
    CONSTI       9
    CONSTI       18
    CONSTF       1.0
    ACTION       3, 3
    JSR          sub_ae
    RSADDI
    CONSTI       3
    CPTOPSP      -12, 4
    JSR          sub_7a
    MOVSP        -4
    MOVSP        -4
    RETN
sub_7a:
    CPTOPSP      -4, 4
    CPTOPSP      -12, 4
    ADDII
    CPDOWNSP     -16, 4
    MOVSP        -4
    JMP          L_a6
    MOVSP        -4
L_a6:
    MOVSP        -8
    RETN
sub_ae:
    CPTOPSP      -4, 4
    ACTION       1, 1
    MOVSP        -4
    RETN
"#,
    );
}

#[test]
fn casedecl() {
    probe(
        r#"void main()
{
    int n = Random(4);
    switch (n)
    {
        case 1:
            int k = 2;
            PrintInteger(k);
            break;
        case 2:
            PrintInteger(n);
            break;
    }
}
"#,
        r#"
; 31 instructions, 186 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    CONSTI       4
    ACTION       0, 1
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    CPTOPSP      -4, 4
    CONSTI       1
    EQUALII
    JNZ          L_6a
    CPTOPSP      -4, 4
    CONSTI       2
    EQUALII
    JNZ          L_93
    JMP          L_ac
L_6a:
    RSADDI
    CONSTI       2
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    ACTION       4, 1
    JMP          L_ac
L_93:
    CPTOPSP      -12, 4
    ACTION       4, 1
    JMP          L_ac
    MOVSP        -4
L_ac:
    MOVSP        -4
    MOVSP        -4
    RETN
"#,
    );
}

#[test]
fn constants() {
    probe(
        r#"void main()
{
    int a = TRUE;
    float f = DIRECTION_NORTH;
    PrintInteger(a);
    PrintFloat(f);
}
"#,
        r#"
; 18 instructions, 111 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    CONSTI       1
    CPDOWNSP     -8, 4
    MOVSP        -4
    RSADDF
    CONSTF       90.0
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -8, 4
    ACTION       4, 1
    CONSTI       9
    CONSTI       18
    CPTOPSP      -12, 4
    ACTION       2, 3
    MOVSP        -8
    RETN
"#,
    );
}

#[test]
fn forparts() {
    probe(
        r#"void main()
{
    int i = 0;
    for (; i < 2; ) { i++; }
    for (i = 0; ; i++) { if (i > 3) break; }
}
"#,
        r#"
; 31 instructions, 191 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    CONSTI       0
    CPDOWNSP     -8, 4
    MOVSP        -4
L_2b:
    CPTOPSP      -4, 4
    CONSTI       2
    LTII
    JZ           L_5b
    CPTOPSP      -4, 4
    INCISP       -8
    MOVSP        -4
    JMP          L_2b
L_5b:
    CONSTI       0
    CPDOWNSP     -8, 4
    MOVSP        -4
L_6f:
    CONSTI       1
    JZ           L_b7
    CPTOPSP      -4, 4
    CONSTI       3
    GTII
    JZ           L_9d
    JMP          L_b7
    JMP          L_9d
L_9d:
    CPTOPSP      -4, 4
    INCISP       -8
    MOVSP        -4
    JMP          L_6f
L_b7:
    MOVSP        -4
    RETN
"#,
    );
}

#[test]
fn globals() {
    probe(
        r#"int G = 4;
string S = "x";
float F;
int Init() { return 7; }
int H = Init();
int StartingConditional()
{
    G++;
    F = 1.5;
    return G == 5;
}
"#,
        r#"
; 45 instructions, 244 bytes
    RSADDI
    JSR          sub_17
    RETN
sub_17:
    RSADDI
    CONSTI       4
    CPDOWNSP     -8, 4
    MOVSP        -4
    RSADDS
    CONSTS       "x"
    CPDOWNSP     -8, 4
    MOVSP        -4
    RSADDF
    RSADDI
    RSADDI
    JSR          sub_7e
    CPDOWNSP     -8, 4
    MOVSP        -4
    SAVEBP
    RSADDI
    JSR          sub_a0
    CPDOWNSP     -28, 4
    MOVSP        -4
    RESTOREBP
    MOVSP        -16
    RETN
sub_7e:
    CONSTI       7
    CPDOWNSP     -8, 4
    MOVSP        -4
    JMP          L_9e
    MOVSP        -4
L_9e:
    RETN
sub_a0:
    CPTOPBP      -16, 4
    INCIBP       -16
    MOVSP        -4
    CONSTF       1.5
    CPDOWNBP     -8, 4
    MOVSP        -4
    CPTOPBP      -16, 4
    CONSTI       5
    EQUALII
    CPDOWNSP     -8, 4
    MOVSP        -4
    JMP          L_f2
    MOVSP        -4
L_f2:
    RETN
"#,
    );
}

#[test]
fn ifs() {
    probe(
        r#"void main()
{
    int n = Random(3);
    if (n == 0) PrintInteger(0);
    if (n == 1) { PrintInteger(1); } else { PrintInteger(2); }
    if (n == 2) { int k = 3; PrintInteger(k); } else if (n == 3) PrintInteger(3); else { }
}
"#,
        r#"
; 44 instructions, 253 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    CONSTI       3
    ACTION       0, 1
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    CONSTI       0
    EQUALII
    JZ           L_57
    CONSTI       0
    ACTION       4, 1
    JMP          L_57
L_57:
    CPTOPSP      -4, 4
    CONSTI       1
    EQUALII
    JZ           L_7e
    CONSTI       1
    ACTION       4, 1
    JMP          L_89
L_7e:
    CONSTI       2
    ACTION       4, 1
L_89:
    CPTOPSP      -4, 4
    CONSTI       2
    EQUALII
    JZ           L_ce
    RSADDI
    CONSTI       3
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    ACTION       4, 1
    MOVSP        -4
    JMP          L_f5
L_ce:
    CPTOPSP      -4, 4
    CONSTI       3
    EQUALII
    JZ           L_f5
    CONSTI       3
    ACTION       4, 1
    JMP          L_f5
L_f5:
    MOVSP        -4
    RETN
"#,
    );
}

#[test]
fn incdec() {
    probe(
        r#"int G;
void main()
{
    int i = 1;
    int j = i++;
    j = ++i;
    j = i--;
    j = --i;
    G++; ++G; G--; --G;
    j = G++;
    i++;
}
"#,
        r#"
; 50 instructions, 313 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    SAVEBP
    JSR          sub_29
    RESTOREBP
    MOVSP        -4
    RETN
sub_29:
    RSADDI
    CONSTI       1
    CPDOWNSP     -8, 4
    MOVSP        -4
    RSADDI
    CPTOPSP      -8, 4
    INCISP       -12
    CPDOWNSP     -8, 4
    MOVSP        -4
    INCISP       -8
    CPTOPSP      -8, 4
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -8, 4
    DECISP       -12
    CPDOWNSP     -8, 4
    MOVSP        -4
    DECISP       -8
    CPTOPSP      -8, 4
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPBP      -4, 4
    INCIBP       -4
    MOVSP        -4
    INCIBP       -4
    CPTOPBP      -4, 4
    MOVSP        -4
    CPTOPBP      -4, 4
    DECIBP       -4
    MOVSP        -4
    DECIBP       -4
    CPTOPBP      -4, 4
    MOVSP        -4
    CPTOPBP      -4, 4
    INCIBP       -4
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -8, 4
    INCISP       -12
    MOVSP        -4
    MOVSP        -8
    RETN
"#,
    );
}

#[test]
fn locals() {
    probe(
        r#"void main()
{
    int a = 1, b, c = a;
    string s;
    float f = 2.5;
    object o = OBJECT_SELF;
    PrintInteger(a + b + c);
}
"#,
        r#"
; 28 instructions, 156 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    CONSTI       1
    CPDOWNSP     -8, 4
    MOVSP        -4
    RSADDI
    RSADDI
    CPTOPSP      -12, 4
    CPDOWNSP     -8, 4
    MOVSP        -4
    RSADDS
    RSADDF
    CONSTF       2.5
    CPDOWNSP     -8, 4
    MOVSP        -4
    RSADDO
    CONSTO       0
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -24, 4
    CPTOPSP      -24, 4
    ADDII
    CPTOPSP      -20, 4
    ADDII
    ACTION       4, 1
    MOVSP        -24
    RETN
"#,
    );
}

#[test]
fn loops() {
    probe(
        r#"void main()
{
    int i;
    while (i < 3) { i++; if (i == 2) continue; PrintInteger(i); }
    for (i = 0; i < 3; i++) { if (i == 1) break; }
    do { i--; if (i == 1) continue; } while (i > 0);
}
"#,
        r#"
; 52 instructions, 310 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
L_17:
    CPTOPSP      -4, 4
    CONSTI       3
    LTII
    JZ           L_76
    CPTOPSP      -4, 4
    INCISP       -8
    MOVSP        -4
    CPTOPSP      -4, 4
    CONSTI       2
    EQUALII
    JZ           L_63
    JMP          L_70
    JMP          L_63
L_63:
    CPTOPSP      -4, 4
    ACTION       4, 1
L_70:
    JMP          L_17
L_76:
    CONSTI       0
    CPDOWNSP     -8, 4
    MOVSP        -4
L_8a:
    CPTOPSP      -4, 4
    CONSTI       3
    LTII
    JZ           L_dc
    CPTOPSP      -4, 4
    CONSTI       1
    EQUALII
    JZ           L_c2
    JMP          L_dc
    JMP          L_c2
L_c2:
    CPTOPSP      -4, 4
    INCISP       -8
    MOVSP        -4
    JMP          L_8a
L_dc:
    CPTOPSP      -4, 4
    DECISP       -8
    MOVSP        -4
    CPTOPSP      -4, 4
    CONSTI       1
    EQUALII
    JZ           L_112
    JMP          L_112
    JMP          L_112
L_112:
    CPTOPSP      -4, 4
    CONSTI       0
    GTII
    JZ           L_12e
    JMP          L_dc
L_12e:
    MOVSP        -4
    RETN
"#,
    );
}

#[test]
fn nested() {
    probe(
        r#"void main()
{
    int n = Random(4);
    switch (n)
    {
        case 1:
            switch (n) { case 1: PrintInteger(1); break; }
            break;
    }
    while (n) { while (n) { n--; break; } break; }
}
"#,
        r#"
; 38 instructions, 235 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    CONSTI       4
    ACTION       0, 1
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    CPTOPSP      -4, 4
    CONSTI       1
    EQUALII
    JNZ          L_54
    JMP          L_95
L_54:
    CPTOPSP      -8, 4
    CPTOPSP      -4, 4
    CONSTI       1
    EQUALII
    JNZ          L_78
    JMP          L_89
L_78:
    CONSTI       1
    ACTION       4, 1
    JMP          L_89
L_89:
    MOVSP        -4
    JMP          L_95
L_95:
    MOVSP        -4
L_9b:
    CPTOPSP      -4, 4
    JZ           L_e3
L_a9:
    CPTOPSP      -4, 4
    JZ           L_d7
    CPTOPSP      -4, 4
    DECISP       -8
    MOVSP        -4
    JMP          L_d7
    JMP          L_a9
L_d7:
    JMP          L_e3
    JMP          L_9b
L_e3:
    MOVSP        -4
    RETN
"#,
    );
}

#[test]
fn operators() {
    probe(
        r#"void main()
{
    int a = 5, b = 3;
    float f = 1.5;
    a = a - b * 2 / 1 % 4;
    a = a & b | 1;
    a = a << 2;
    a = ~a;
    a = -a;
    f = f * a;
    f = a + f;
    a += 2; a -= 1; a /= 2;
    f /= 2.0; f *= 3.0; f += a;
    a = (a == b) && (f > 1.0) || !(a != 0) || (f <= 2.0) && (a >= 1);
    PrintFloat(f);
}
"#,
        r#"
; 125 instructions, 730 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    CONSTI       5
    CPDOWNSP     -8, 4
    MOVSP        -4
    RSADDI
    CONSTI       3
    CPDOWNSP     -8, 4
    MOVSP        -4
    RSADDF
    CONSTF       1.5
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -12, 4
    CPTOPSP      -12, 4
    CONSTI       2
    MULII
    CONSTI       1
    DIVII
    CONSTI       4
    MODII
    SUBII
    CPDOWNSP     -16, 4
    MOVSP        -4
    CPTOPSP      -12, 4
    CPTOPSP      -12, 4
    BOOLANDII
    CONSTI       1
    INCORII
    CPDOWNSP     -16, 4
    MOVSP        -4
    CPTOPSP      -12, 4
    CONSTI       2
    SHLEFTII
    CPDOWNSP     -16, 4
    MOVSP        -4
    CPTOPSP      -12, 4
    COMPI
    CPDOWNSP     -16, 4
    MOVSP        -4
    CPTOPSP      -12, 4
    NEGI
    CPDOWNSP     -16, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    CPTOPSP      -16, 4
    MULFI
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -12, 4
    CPTOPSP      -8, 4
    ADDIF
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -12, 4
    CONSTI       2
    ADDII
    CPDOWNSP     -16, 4
    MOVSP        -4
    CPTOPSP      -12, 4
    CONSTI       1
    SUBII
    CPDOWNSP     -16, 4
    MOVSP        -4
    CPTOPSP      -12, 4
    CONSTI       2
    DIVII
    CPDOWNSP     -16, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    CONSTF       2.0
    DIVFF
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    CONSTF       3.0
    MULFF
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    CPTOPSP      -16, 4
    ADDFI
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -12, 4
    CPTOPSP      -12, 4
    EQUALII
    CPTOPSP      -4, 4
    JZ           L_22d
    CPTOPSP      -8, 4
    CONSTF       1.0
    GTFF
    LOGANDII
L_22d:
    CPTOPSP      -4, 4
    JZ           L_249
    CPTOPSP      -4, 4
    JZ           L_25b
L_249:
    CPTOPSP      -16, 4
    CONSTI       0
    NEQUALII
    NOTI
L_25b:
    LOGORII
    CPTOPSP      -4, 4
    JZ           L_279
    CPTOPSP      -4, 4
    JZ           L_2a9
L_279:
    CPTOPSP      -8, 4
    CONSTF       2.0
    LEQFF
    CPTOPSP      -4, 4
    JZ           L_2a9
    CPTOPSP      -20, 4
    CONSTI       1
    GEQII
    LOGANDII
L_2a9:
    LOGORII
    CPDOWNSP     -16, 4
    MOVSP        -4
    CONSTI       9
    CONSTI       18
    CPTOPSP      -12, 4
    ACTION       2, 3
    MOVSP        -12
    RETN
"#,
    );
}

#[test]
fn order() {
    probe(
        r#"void C() { PrintInteger(3); }
void B() { C(); }
void A() { PrintInteger(1); }
void main()
{
    A();
    B();
    C();
}
"#,
        r#"
; 14 instructions, 75 bytes
    JSR          sub_15
    RETN
sub_15:
    JSR          sub_29
    JSR          sub_36
    JSR          sub_3e
    RETN
sub_29:
    CONSTI       1
    ACTION       4, 1
    RETN
sub_36:
    JSR          sub_3e
    RETN
sub_3e:
    CONSTI       3
    ACTION       4, 1
    RETN
"#,
    );
}

#[test]
fn returns() {
    probe(
        r#"int F(int n)
{
    int a = 1;
    if (n) { int b = 2; return a + b; }
    while (n) { return 5; }
    return 0;
}
void G(int n) { if (n) return; PrintInteger(n); }
void H() { return; }
void main()
{
    F(1);
    G(2);
    H();
}
"#,
        r#"
; 55 instructions, 328 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    CONSTI       1
    JSR          sub_3d
    MOVSP        -4
    CONSTI       2
    JSR          sub_111
    JSR          sub_140
    RETN
sub_3d:
    RSADDI
    CONSTI       1
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -8, 4
    JZ           L_af
    RSADDI
    CONSTI       2
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -8, 4
    CPTOPSP      -8, 4
    ADDII
    CPDOWNSP     -20, 4
    MOVSP        -12
    JMP          L_109
    MOVSP        -4
    MOVSP        -4
    JMP          L_af
L_af:
    CPTOPSP      -8, 4
    JZ           L_e3
    CONSTI       5
    CPDOWNSP     -16, 4
    MOVSP        -8
    JMP          L_109
    MOVSP        -4
    JMP          L_af
L_e3:
    CONSTI       0
    CPDOWNSP     -16, 4
    MOVSP        -8
    JMP          L_109
    MOVSP        -4
    MOVSP        -4
L_109:
    MOVSP        -4
    RETN
sub_111:
    CPTOPSP      -4, 4
    JZ           L_12b
    JMP          L_138
    JMP          L_12b
L_12b:
    CPTOPSP      -4, 4
    ACTION       4, 1
L_138:
    MOVSP        -4
    RETN
sub_140:
    JMP          L_146
L_146:
    RETN
"#,
    );
}

#[test]
fn strings() {
    probe(
        r#"void main()
{
    string s = "a" + "b";
    s = s + "c";
    if (s == "abc" && s != "") PrintString(s);
    object o = OBJECT_INVALID;
    if (o == OBJECT_SELF) PrintObject(o);
}
"#,
        r#"
; 39 instructions, 221 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDS
    CONSTS       "a"
    CONSTS       "b"
    ADDSS
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    CONSTS       "c"
    ADDSS
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    CONSTS       "abc"
    EQUALSS
    CPTOPSP      -4, 4
    JZ           L_7d
    CPTOPSP      -8, 4
    CONSTS       ""
    NEQUALSS
    LOGANDII
L_7d:
    JZ           L_96
    CPTOPSP      -4, 4
    ACTION       1, 1
    JMP          L_96
L_96:
    RSADDO
    CONSTO       1
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    CONSTO       0
    EQUALOO
    JZ           L_d5
    CPTOPSP      -4, 4
    ACTION       5, 1
    JMP          L_d5
L_d5:
    MOVSP        -8
    RETN
"#,
    );
}

#[test]
fn structfn() {
    probe(
        r#"struct P { int a; string b; };
struct P Make(int a) { struct P p; p.a = a; return p; }
int Get(struct P p) { return p.a; }
void main()
{
    struct P q = Make(2);
    PrintInteger(Get(q));
    PrintInteger(Make(3).a);
}
"#,
        r#"
; 47 instructions, 249 bytes
    JSR          sub_15
    RETN
sub_15:
    SAVEBP
    JSR          sub_21
    RESTOREBP
    RETN
sub_21:
    RSADDI
    RSADDS
    RSADDI
    RSADDS
    CONSTI       2
    JSR          sub_7d
    CPDOWNSP     -16, 8
    MOVSP        -8
    RSADDI
    CPTOPSP      -12, 8
    JSR          sub_c7
    ACTION       4, 1
    RSADDI
    RSADDS
    CONSTI       3
    JSR          sub_7d
    DESTRUCT     8, 0, 4
    ACTION       4, 1
    MOVSP        -8
    RETN
sub_7d:
    RSADDI
    RSADDS
    CPTOPSP      -12, 4
    CPDOWNSP     -12, 4
    MOVSP        -4
    CPTOPSP      -8, 8
    CPDOWNSP     -28, 8
    MOVSP        -16
    JMP          L_bf
    MOVSP        -8
    MOVSP        -8
L_bf:
    MOVSP        -4
    RETN
sub_c7:
    CPTOPSP      -8, 8
    DESTRUCT     8, 0, 4
    CPDOWNSP     -16, 4
    MOVSP        -4
    JMP          L_f1
    MOVSP        -4
L_f1:
    MOVSP        -8
    RETN
"#,
    );
}

#[test]
fn structs() {
    probe(
        r#"struct P { int a; float b; string c; };
struct P g;
void main()
{
    struct P l;
    l.a = 1;
    l.b = 2.0;
    g = l;
    g.c = "c";
    PrintInteger(l.a);
    PrintString(g.c);
    if (l == g) PrintInteger(1);
    l.a++;
    l.a += 3;
}
"#,
        r#"
; 50 instructions, 291 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    RSADDF
    RSADDS
    SAVEBP
    JSR          sub_2d
    RESTOREBP
    MOVSP        -12
    RETN
sub_2d:
    RSADDI
    RSADDF
    RSADDS
    CONSTI       1
    CPDOWNSP     -16, 4
    MOVSP        -4
    CONSTF       2.0
    CPDOWNSP     -12, 4
    MOVSP        -4
    CPTOPSP      -12, 12
    CPDOWNBP     -12, 12
    MOVSP        -12
    CONSTS       "c"
    CPDOWNBP     -4, 4
    MOVSP        -4
    CPTOPSP      -12, 12
    DESTRUCT     12, 0, 4
    ACTION       4, 1
    CPTOPBP      -12, 12
    DESTRUCT     12, 8, 4
    ACTION       1, 1
    CPTOPSP      -12, 12
    CPTOPBP      -12, 12
    EQUALTT      12
    JZ           L_d9
    CONSTI       1
    ACTION       4, 1
    JMP          L_d9
L_d9:
    CPTOPSP      -12, 12
    DESTRUCT     12, 0, 4
    INCISP       -16
    MOVSP        -4
    CPTOPSP      -12, 12
    DESTRUCT     12, 0, 4
    CONSTI       3
    ADDII
    CPDOWNSP     -16, 4
    MOVSP        -4
    MOVSP        -12
    RETN
"#,
    );
}

#[test]
fn switches() {
    probe(
        r#"void main()
{
    int n = Random(5);
    switch (n)
    {
        case 0: PrintInteger(0); break;
        case 1:
        case 2: { PrintInteger(2); } break;
        case -3: PrintInteger(3);
        default: PrintInteger(9); break;
    }
    switch (n) { case 4: PrintInteger(4); }
}
"#,
        r#"
; 48 instructions, 279 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    CONSTI       5
    ACTION       0, 1
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    CPTOPSP      -4, 4
    CONSTI       0
    EQUALII
    JNZ          L_96
    CPTOPSP      -4, 4
    CONSTI       1
    EQUALII
    JNZ          L_a7
    CPTOPSP      -4, 4
    CONSTI       2
    EQUALII
    JNZ          L_a7
    CPTOPSP      -4, 4
    CONSTI       -3
    EQUALII
    JNZ          L_b8
    JMP          L_c3
L_96:
    CONSTI       0
    ACTION       4, 1
    JMP          L_d4
L_a7:
    CONSTI       2
    ACTION       4, 1
    JMP          L_d4
L_b8:
    CONSTI       3
    ACTION       4, 1
L_c3:
    CONSTI       9
    ACTION       4, 1
    JMP          L_d4
L_d4:
    MOVSP        -4
    CPTOPSP      -4, 4
    CPTOPSP      -4, 4
    CONSTI       4
    EQUALII
    JNZ          L_fe
    JMP          L_109
L_fe:
    CONSTI       4
    ACTION       4, 1
L_109:
    MOVSP        -4
    MOVSP        -4
    RETN
"#,
    );
}

#[test]
fn unary() {
    probe(
        r#"void main()
{
    int a = -1;
    float f = -1.5;
    a = +a;
    a = !a;
    f = -f;
    PrintFloat(f);
}
"#,
        r#"
; 29 instructions, 172 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    CONSTI       1
    NEGI
    CPDOWNSP     -8, 4
    MOVSP        -4
    RSADDF
    CONSTF       1.5
    NEGF
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -8, 4
    CPDOWNSP     -12, 4
    MOVSP        -4
    CPTOPSP      -8, 4
    NOTI
    CPDOWNSP     -12, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    NEGF
    CPDOWNSP     -8, 4
    MOVSP        -4
    CONSTI       9
    CONSTI       18
    CPTOPSP      -12, 4
    ACTION       2, 3
    MOVSP        -8
    RETN
"#,
    );
}

#[test]
fn vectors() {
    probe(
        r#"vector V(vector v = [1.0, 2.0, 3.0]) { return v; }
void main()
{
    vector a = [0.5, 1.5, 2.0];
    vector b = V();
    a = a + b;
    a = a - b;
    a = a * 2.0;
    a = 2.0 * a;
    a = a / 2.0;
    PrintFloat(a.x + a.y + a.z);
    a.y = 4.0;
    if (a == b) PrintInteger(1);
    if (a != b) PrintInteger(2);
}
"#,
        r#"
; 84 instructions, 488 bytes
    JSR          sub_15
    RETN
sub_15:
    RSADDF
    RSADDF
    RSADDF
    CONSTF       0.5
    CONSTF       1.5
    CONSTF       2.0
    CPDOWNSP     -24, 12
    MOVSP        -12
    RSADDF
    RSADDF
    RSADDF
    RSADDF
    RSADDF
    RSADDF
    CONSTF       1.0
    CONSTF       2.0
    CONSTF       3.0
    JSR          sub_1be
    CPDOWNSP     -24, 12
    MOVSP        -12
    CPTOPSP      -24, 12
    CPTOPSP      -24, 12
    ADDVV
    CPDOWNSP     -36, 12
    MOVSP        -12
    CPTOPSP      -24, 12
    CPTOPSP      -24, 12
    SUBVV
    CPDOWNSP     -36, 12
    MOVSP        -12
    CPTOPSP      -24, 12
    CONSTF       2.0
    MULVF
    CPDOWNSP     -36, 12
    MOVSP        -12
    CONSTF       2.0
    CPTOPSP      -28, 12
    MULFV
    CPDOWNSP     -36, 12
    MOVSP        -12
    CPTOPSP      -24, 12
    CONSTF       2.0
    DIVVF
    CPDOWNSP     -36, 12
    MOVSP        -12
    CONSTI       9
    CONSTI       18
    CPTOPSP      -32, 12
    DESTRUCT     12, 0, 4
    CPTOPSP      -36, 12
    DESTRUCT     12, 4, 4
    ADDFF
    CPTOPSP      -36, 12
    DESTRUCT     12, 8, 4
    ADDFF
    ACTION       2, 3
    CONSTF       4.0
    CPDOWNSP     -24, 4
    MOVSP        -4
    CPTOPSP      -24, 12
    CPTOPSP      -24, 12
    EQUALTT      12
    JZ           L_18b
    CONSTI       1
    ACTION       4, 1
    JMP          L_18b
L_18b:
    CPTOPSP      -24, 12
    CPTOPSP      -24, 12
    NEQUALTT     12
    JZ           L_1b6
    CONSTI       2
    ACTION       4, 1
    JMP          L_1b6
L_1b6:
    MOVSP        -24
    RETN
sub_1be:
    CPTOPSP      -12, 12
    CPDOWNSP     -36, 12
    MOVSP        -12
    JMP          L_1e0
    MOVSP        -12
L_1e0:
    MOVSP        -12
    RETN
"#,
    );
}
