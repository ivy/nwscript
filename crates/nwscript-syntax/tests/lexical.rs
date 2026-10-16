//! The lexical grammar: `docs/grammar.md`, "Lexical grammar".

mod common;
use common::*;
use nwscript_diag::FileId;
use nwscript_syntax::SyntaxKind::{self, *};
use nwscript_syntax::lexer::lex;

/// Kinds of the tokens of `src`, and the codes of its diagnostics.
fn kinds(src: &[u8]) -> (Vec<SyntaxKind>, Vec<&'static str>) {
    let (tokens, diags) = lex(FileId(0), src);
    let total: u32 = tokens.iter().map(|t| t.len).sum();
    assert_eq!(total as usize, src.len(), "tokens do not cover the input");
    (
        tokens.iter().map(|t| t.kind).collect(),
        diags.iter().map(|d| d.code.id).collect(),
    )
}

fn sig(src: &[u8]) -> Vec<SyntaxKind> {
    let (k, d) = kinds(src);
    assert!(d.is_empty(), "{d:?}");
    k.into_iter().filter(|k| !k.is_trivia()).collect()
}

#[test]
fn line_ends_and_whitespace() {
    assert_eq!(
        kinds(b"a\r\nb\nc\rd \t e").0,
        [
            IDENT, NEWLINE, IDENT, NEWLINE, IDENT, NEWLINE, IDENT, WHITESPACE, IDENT
        ]
    );
}

#[test]
fn comments() {
    assert_eq!(kinds(b"// a /* b\r\nx").0, [LINE_COMMENT, NEWLINE, IDENT]);
    assert_eq!(kinds(b"/* a // b\r\n c */x").0, [BLOCK_COMMENT, IDENT]);
    // The first `*/` ends it; comments do not nest.
    assert_eq!(
        kinds(b"/* /* */ */").0,
        [BLOCK_COMMENT, WHITESPACE, STAR, SLASH]
    );
    // Bytes above 0x7F are allowed in comments.
    assert_eq!(kinds(b"// \x93q\x94").1, Vec::<&str>::new());
    assert_eq!(kinds(b"/* \xe9 */").1, Vec::<&str>::new());
}

#[test]
fn identifiers_and_keywords() {
    assert_eq!(sig(b"_a1 Z9_"), [IDENT, IDENT]);
    let words = b"if else while do for switch case default break continue return struct const \
                  void int float string object vector action OBJECT_SELF OBJECT_INVALID";
    let ks = sig(words);
    assert_eq!(ks.len(), 22);
    assert!(ks.iter().all(|k| k.is_keyword()));
    // Keywords in another case, the engine structure types, and the
    // directive words are identifiers.
    assert_eq!(
        sig(b"INT Vector effect event location talent itemproperty include define TRUE"),
        [IDENT; 10]
    );
}

#[test]
fn numbers() {
    assert_eq!(sig(b"0 65535 0xffff 0xFF"), [INT_NUMBER; 4]);
    assert_eq!(sig(b"1.0 1.0f 0.25"), [FLOAT_NUMBER; 3]);
    for bad in [
        &b"1."[..],
        b"1e3",
        b"1.0e3",
        b"1f",
        b"1.0F",
        b"0x",
        b"0xg",
        b"12abc",
        b"0X1",
    ] {
        assert_eq!(kinds(bad).1, ["E0204"], "{}", String::from_utf8_lossy(bad));
    }
    // `.5` is a dot and a number.
    assert_eq!(sig(b".5"), [DOT, INT_NUMBER]);
}

#[test]
fn strings() {
    assert_eq!(sig(b"\"\" \"abc\" \"// /*\""), [STRING; 3]);
    // A backslash takes the next byte with it, so `\"` does not end a string.
    assert_eq!(sig(b"\"a\\\"b\""), [STRING]);
}

#[test]
fn punctuators_take_the_longest_match() {
    let all = b"{ } ( ) [ ] ; , . : ? # = += -= *= /= %= &= |= ^= <<= >>= >>>= || && | ^ & == != < > <= >= << >> >>> + - * / % ! ~ ++ --";
    let ks = sig(all);
    assert_eq!(ks.len(), all.split(|&b| b == b' ').count());
    assert!(ks.iter().all(|k| k.is_punct()));
    assert_eq!(sig(b">>>=>>>>>=>>"), [USHR_EQ, USHR, SHR_EQ, SHR]);
    assert_eq!(sig(b"a+++b"), [IDENT, PLUS2, PLUS, IDENT]);
    assert_eq!(sig(b"a---b"), [IDENT, MINUS2, MINUS, IDENT]);
}

#[test]
fn unknown_characters() {
    let (k, d) = kinds(b"a @$ b \x93 c");
    assert!(k.contains(&UNKNOWN));
    assert_eq!(d, ["E0201", "E0201"]);
}

#[test]
fn every_byte_is_kept() {
    let all: Vec<u8> = (0..=255).collect();
    let p = nwscript_syntax::parse(FileId(0), &all);
    assert_eq!(p.to_bytes(), all);
}

#[test]
fn spans_are_byte_offsets() {
    // Two high bytes in a comment before the error: tree text offsets run
    // two ahead of byte offsets after them.
    let src = b"// \x93\x94\r\nint x = 1 @;";
    let p = nwscript_syntax::parse(FileId(0), src);
    let d = &p.diagnostics()[0];
    let span = d.primary_span().unwrap();
    assert_eq!(&src[span.start..span.end], b"@");
    let decl = p.tree().globals().next().unwrap();
    let s = p.span_of(decl.syntax().clone());
    assert_eq!(&src[s.start..s.end], b"int x = 1 @;");
    assert_eq!(
        p.span(decl.syntax().text_range()).start,
        src.iter().position(|&b| b == b'i').unwrap()
    );
    assert_eq!(u32::from(p.text_offset(span.start)), span.start as u32 + 2);
}
