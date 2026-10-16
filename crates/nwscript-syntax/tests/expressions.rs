//! Expressions: `docs/grammar.md`, "Expressions" and "Precedence and
//! associativity". Each operator, each precedence level against its
//! neighbours, and each associativity.

mod common;
use common::*;

#[test]
fn primaries() {
    assert_eq!(expr("42"), "42");
    assert_eq!(expr("0xff"), "255");
    assert_eq!(expr("1.5"), "1.5");
    assert_eq!(expr("1.5f"), "1.5");
    assert_eq!(expr("\"hi\""), "\"hi\"");
    assert_eq!(expr("\"\""), "\"\"");
    assert_eq!(expr("OBJECT_SELF"), "OBJECT_SELF");
    assert_eq!(expr("OBJECT_INVALID"), "OBJECT_INVALID");
    assert_eq!(expr("nCount"), "nCount");
    assert_eq!(expr("(x)"), "(paren x)");
    assert_eq!(expr("[0.0, 1.0, 2.0]"), "[0.0 1.0 2.0]");
}

#[test]
fn calls() {
    assert_eq!(expr("GetFirstPC()"), "(call GetFirstPC)");
    assert_eq!(expr("f(1)"), "(call f 1)");
    assert_eq!(
        expr("DelayCommand(1.0, AssignCommand(o, ActionWait(2.0)))"),
        "(call DelayCommand 1.0 (call AssignCommand o (call ActionWait 2.0)))"
    );
    assert_eq!(expr("f(a = 1, b + c)"), "(call f (= a 1) (+ b c))");
}

#[test]
fn members() {
    assert_eq!(expr("s.nField"), "(. s nField)");
    assert_eq!(expr("s.a.b"), "(. (. s a) b)");
    assert_eq!(expr("f().x"), "(. (call f) x)");
    assert_eq!(expr("v.x + v.y"), "(+ (. v x) (. v y))");
}

#[test]
fn postfix_and_prefix() {
    assert_eq!(expr("i++"), "(post++ i)");
    assert_eq!(expr("i--"), "(post-- i)");
    assert_eq!(expr("++i"), "(++ i)");
    assert_eq!(expr("--i"), "(-- i)");
    assert_eq!(expr("-1"), "(- 1)");
    assert_eq!(expr("+1"), "(+ 1)");
    assert_eq!(expr("!b"), "(! b)");
    assert_eq!(expr("~n"), "(~ n)");
    assert_eq!(expr("- -1"), "(- (- 1))");
    assert_eq!(expr("!!b"), "(! (! b))");
    // Postfix binds tighter than prefix.
    assert_eq!(expr("-i++"), "(- (post++ i))");
    assert_eq!(expr("-s.x"), "(- (. s x))");
    assert_eq!(expr("s.n++"), "(post++ (. s n))");
}

#[test]
fn each_binary_operator() {
    for op in [
        "||", "&&", "|", "^", "&", "==", "!=", "<", ">", "<=", ">=", "<<", ">>", ">>>", "+", "-",
        "*", "/", "%",
    ] {
        assert_eq!(expr(&format!("a {op} b")), format!("({op} a b)"), "{op}");
    }
}

#[test]
fn each_assignment_operator() {
    for op in [
        "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", ">>>=",
    ] {
        assert_eq!(expr(&format!("a {op} b")), format!("({op} a b)"), "{op}");
    }
    assert_eq!(expr("s.n = 1"), "(= (. s n) 1)");
}

#[test]
fn conditional() {
    assert_eq!(expr("c ? a : b"), "(? c a b)");
}

/// The operators of each level of the table, tightest first (levels 3 to 12;
/// unary and postfix are tested above, `?:` and assignment below).
const LEVELS: &[&[&str]] = &[
    &["*", "/", "%"],
    &["+", "-"],
    &["<<", ">>", ">>>"],
    &["<", ">", "<=", ">="],
    &["==", "!="],
    &["&"],
    &["^"],
    &["|"],
    &["&&"],
    &["||"],
];

#[test]
fn every_level_binds_tighter_than_every_looser_level() {
    for (i, tight) in LEVELS.iter().enumerate() {
        for loose in &LEVELS[i + 1..] {
            for t in *tight {
                for l in *loose {
                    assert_eq!(
                        expr(&format!("a {l} b {t} c")),
                        format!("({l} a ({t} b c))"),
                        "{l} over {t}"
                    );
                    assert_eq!(
                        expr(&format!("a {t} b {l} c")),
                        format!("({l} ({t} a b) c)"),
                        "{t} under {l}"
                    );
                }
            }
        }
    }
}

#[test]
fn binary_operators_are_left_associative() {
    for level in LEVELS {
        for x in *level {
            for y in *level {
                assert_eq!(
                    expr(&format!("a {x} b {y} c")),
                    format!("({y} ({x} a b) c)"),
                    "{x} then {y}"
                );
            }
        }
    }
}

#[test]
fn prefix_binds_tighter_than_binary() {
    // `k_swg_jolee12.nss`: `!n > 2` is `(!n) > 2`.
    assert_eq!(expr("!n > 2"), "(> (! n) 2)");
    assert_eq!(expr("-a * b"), "(* (- a) b)");
    assert_eq!(expr("~a & b"), "(& (~ a) b)");
    assert_eq!(expr("a - -b"), "(- a (- b))");
}

#[test]
fn conditional_is_right_associative_and_below_logical_or() {
    assert_eq!(expr("a || b ? c : d"), "(? (|| a b) c d)");
    assert_eq!(expr("a ? b : c ? d : e"), "(? a b (? c d e))");
    assert_eq!(expr("a ? b ? c : d : e"), "(? a (? b c d) e)");
    assert_eq!(expr("a ? b = 1 : c"), "(? a (= b 1) c)");
}

#[test]
fn assignment_is_right_associative_and_loosest() {
    assert_eq!(expr("a = b = c"), "(= a (= b c))");
    assert_eq!(expr("a += b -= c"), "(+= a (-= b c))");
    assert_eq!(expr("a = b || c"), "(= a (|| b c))");
    assert_eq!(expr("a = b ? c : d"), "(= a (? b c d))");
    // `k_inc_treasure.nss`: an assignment as a parenthesised operand.
    assert_eq!(expr("(i = f(s)) >= 0"), "(>= (paren (= i (call f s))) 0)");
}

#[test]
fn parentheses_override_precedence() {
    assert_eq!(expr("(a + b) * c"), "(* (paren (+ a b)) c)");
    assert_eq!(expr("a & (b == c)"), "(& a (paren (== b c)))");
}

#[test]
fn string_concatenation() {
    assert_eq!(
        expr("\"a\" + IntToString(n) + \"b\""),
        "(+ (+ \"a\" (call IntToString n)) \"b\")"
    );
}

#[test]
fn literal_values() {
    let v = |src: &str| {
        let stmts = statements(&format!("{src};"));
        let Stmt::Expr(e) = &stmts[0] else { panic!() };
        let Some(Expr::Literal(l)) = e.expr() else {
            panic!()
        };
        l.value()
    };
    assert_eq!(v("65535"), Some(LiteralValue::Int(65535)));
    assert_eq!(v("0xffff"), Some(LiteralValue::Int(65535)));
    assert_eq!(v("2147483648"), Some(LiteralValue::Int(2_147_483_648)));
    assert_eq!(v("0.25f"), Some(LiteralValue::Float(0.25)));
    assert_eq!(v("\"x y\""), Some(LiteralValue::String(b"x y".to_vec())));
    assert_eq!(
        v("\"a\\\"b\""),
        Some(LiteralValue::String(b"a\\\"b".to_vec()))
    );
    assert_eq!(v("OBJECT_SELF"), Some(LiteralValue::ObjectSelf));
    assert_eq!(v("OBJECT_INVALID"), Some(LiteralValue::ObjectInvalid));
}

#[test]
fn operator_enums() {
    let stmts = statements("a >>>= b; a >>> b; -a; a--;");
    let e = |i: usize| match &stmts[i] {
        Stmt::Expr(e) => e.expr().unwrap(),
        _ => panic!(),
    };
    let Expr::Assign(a) = e(0) else { panic!() };
    assert_eq!(a.op(), Some(AssignOp::UShr));
    assert_eq!(AssignOp::UShr.binary_op(), Some(BinaryOp::UShr));
    let Expr::Binary(b) = e(1) else { panic!() };
    assert_eq!(b.op(), Some(BinaryOp::UShr));
    let Expr::Prefix(p) = e(2) else { panic!() };
    assert_eq!(p.op(), Some(PrefixOp::Neg));
    let Expr::Postfix(p) = e(3) else { panic!() };
    assert_eq!(p.op(), Some(IncDec::Dec));
}
