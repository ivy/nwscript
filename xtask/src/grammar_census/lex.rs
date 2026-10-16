//! The census lexer: tokens for the recogniser, and a count of every lexical
//! form it meets. Trivia (whitespace, comments) is counted and dropped.

use super::Census;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Ident,
    Int,
    Float,
    Str,
    Punct,
    /// A `#` line; the text is the whole line without its line end.
    Directive,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: Kind,
    pub text: String,
    pub line: u32,
}

/// Longest first, so the first match is the longest.
const PUNCT: &[&str] = &[
    ">>>=", ">>>", "<<=", ">>=", "++", "--", "&&", "||", "==", "!=", "<=", ">=", "+=", "-=", "*=",
    "/=", "%=", "&=", "|=", "^=", "<<", ">>", "+", "-", "*", "/", "%", "&", "|", "^", "~", "!",
    "<", ">", "=", "?", ":", ";", ",", ".", "(", ")", "{", "}", "[", "]",
];

pub const KEYWORDS: &[&str] = &[
    "if",
    "else",
    "while",
    "do",
    "for",
    "switch",
    "case",
    "default",
    "break",
    "continue",
    "return",
    "struct",
    "const",
    "void",
    "int",
    "float",
    "string",
    "object",
    "vector",
    "action",
    "effect",
    "event",
    "location",
    "talent",
    "itemproperty",
    "OBJECT_SELF",
    "OBJECT_INVALID",
];

pub fn lex(src: &[u8], file: &str, c: &mut Census) -> Vec<Token> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1u32;
    // Only whitespace since the last line end: a `#` here starts a directive.
    let mut line_start = true;
    let n = src.len();
    let at = |j: usize| src.get(j).copied().unwrap_or(0);

    if n > 0 && !src.ends_with(b"\n") {
        c.hit_at("lex.no_final_line_end", file, line);
    }
    while i < n {
        let b = src[i];
        match b {
            b'\r' if at(i + 1) == b'\n' => {
                c.hit_at("lex.line_end.crlf", file, line);
                i += 2;
                line += 1;
                line_start = true;
            }
            b'\n' => {
                c.hit_at("lex.line_end.lf", file, line);
                i += 1;
                line += 1;
                line_start = true;
            }
            b'\r' => {
                c.hit_at("lex.line_end.cr", file, line);
                i += 1;
                line += 1;
                line_start = true;
            }
            b' ' => i += 1,
            b'\t' => {
                c.hit_at("lex.ws.tab", file, line);
                i += 1;
            }
            0x0b | 0x0c => {
                c.hit_at("lex.ws.vt_ff", file, line);
                i += 1;
            }
            0x1a => {
                c.hit_at("lex.ctrl_z", file, line);
                i += 1;
            }
            b'/' if at(i + 1) == b'/' => {
                let s = i;
                while i < n && src[i] != b'\n' && src[i] != b'\r' {
                    i += 1;
                }
                let body = &src[s..i];
                c.hit_at("lex.comment.line", file, line);
                if i == n {
                    c.hit_at("lex.comment.line.at_eof", file, line);
                }
                if body.ends_with(b"\\") {
                    c.hit_at("lex.comment.line.backslash_end", file, line);
                }
                if body.iter().any(|&x| x >= 0x80) {
                    c.hit_at("lex.non_ascii.comment", file, line);
                }
                if body.windows(2).any(|w| w == b"/*") {
                    c.hit_at("lex.comment.line.contains_block_open", file, line);
                }
                if body.windows(2).any(|w| w == b"*/") {
                    c.hit_at("lex.comment.line.contains_block_close", file, line);
                }
            }
            b'/' if at(i + 1) == b'*' => {
                let s = i;
                i += 2;
                let mut closed = false;
                while i < n {
                    if src[i] == b'*' && at(i + 1) == b'/' {
                        i += 2;
                        closed = true;
                        break;
                    }
                    if src[i] == b'\n' {
                        line += 1;
                    }
                    i += 1;
                }
                let body = &src[s + 2..if closed { i - 2 } else { i }];
                c.hit_at("lex.comment.block", file, line);
                if body.contains(&b'\n') {
                    c.hit_at("lex.comment.block.multiline", file, line);
                }
                if body.windows(2).any(|w| w == b"/*") {
                    c.hit_at("lex.comment.block.contains_block_open", file, line);
                }
                if body.windows(2).any(|w| w == b"//") {
                    c.hit_at("lex.comment.block.contains_line_comment", file, line);
                }
                if !closed {
                    c.hit_at("lex.comment.block.unterminated", file, line);
                }
                if body.iter().any(|&x| x >= 0x80) {
                    c.hit_at("lex.non_ascii.comment", file, line);
                }
                // A block comment that ends a line's code keeps line_start as is.
            }
            b'#' => {
                let s = i;
                while i < n && src[i] != b'\n' && src[i] != b'\r' {
                    i += 1;
                }
                let text = String::from_utf8_lossy(&src[s..i]).into_owned();
                if !line_start {
                    c.hit_at("lex.hash_mid_line", file, line);
                }
                out.push(Token {
                    kind: Kind::Directive,
                    text,
                    line,
                });
            }
            b'"' => {
                let s = i;
                i += 1;
                let mut closed = false;
                while i < n && src[i] != b'\n' && src[i] != b'\r' {
                    if src[i] == b'\\' {
                        let e = at(i + 1);
                        c.hit_at(&format!("lex.string.escape.{}", escape_name(e)), file, line);
                        i += 2;
                        continue;
                    }
                    if src[i] == b'"' {
                        closed = true;
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                let body = &src[s + 1..if closed { i - 1 } else { i }];
                c.hit_at("lex.string", file, line);
                if body.is_empty() {
                    c.hit_at("lex.string.empty", file, line);
                }
                if !closed {
                    c.hit_at("lex.string.unterminated", file, line);
                }
                if body.iter().any(|&x| x >= 0x80) {
                    c.hit_at("lex.non_ascii.string", file, line);
                }
                if body.windows(2).any(|w| w == b"//" || w == b"/*") {
                    c.hit_at("lex.string.contains_comment_marker", file, line);
                }
                c.max("lex.string.longest", body.len() as i64, file);
                out.push(Token {
                    kind: Kind::Str,
                    text: String::from_utf8_lossy(&src[s..i]).into_owned(),
                    line,
                });
                line_start = false;
            }
            b'0'..=b'9' | b'.' if b != b'.' || at(i + 1).is_ascii_digit() => {
                let s = i;
                let (kind, form) = number(src, &mut i);
                let text = String::from_utf8_lossy(&src[s..i]).into_owned();
                c.hit_at(&format!("lex.{form}"), file, line);
                if at(i).is_ascii_alphanumeric() || at(i) == b'_' {
                    c.hit_at("lex.number.followed_by_ident_char", file, line);
                }
                if form == "int.hex" {
                    let digits = &text[2..];
                    if text.as_bytes()[1] == b'X' {
                        c.hit_at("lex.int.hex.upper_x", file, line);
                    }
                    if digits.bytes().any(|b| b.is_ascii_uppercase()) {
                        c.hit_at("lex.int.hex.upper_digits", file, line);
                    }
                    if digits.bytes().any(|b| b.is_ascii_lowercase()) {
                        c.hit_at("lex.int.hex.lower_digits", file, line);
                    }
                    if let Ok(v) = i64::from_str_radix(digits, 16) {
                        c.max("lex.int.hex.largest", v, file);
                    }
                } else if kind == Kind::Int
                    && let Ok(v) = text.parse::<i64>()
                {
                    c.max("lex.int.decimal.largest", v, file);
                }
                out.push(Token { kind, text, line });
                line_start = false;
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                let s = i;
                while i < n && (src[i].is_ascii_alphanumeric() || src[i] == b'_') {
                    i += 1;
                }
                let text = String::from_utf8_lossy(&src[s..i]).into_owned();
                c.max("lex.ident.longest", text.len() as i64, file);
                if KEYWORDS.contains(&text.as_str()) {
                    c.hit_at(&format!("lex.keyword.{text}"), file, line);
                } else {
                    c.hit_at("lex.ident", file, line);
                    if KEYWORDS.contains(&text.to_ascii_lowercase().as_str()) {
                        c.hit_at("lex.ident.keyword_other_case", file, line);
                    }
                }
                out.push(Token {
                    kind: Kind::Ident,
                    text,
                    line,
                });
                line_start = false;
            }
            _ => {
                let rest = &src[i..];
                if let Some(p) = PUNCT.iter().find(|p| rest.starts_with(p.as_bytes())) {
                    i += p.len();
                    out.push(Token {
                        kind: Kind::Punct,
                        text: (*p).to_string(),
                        line,
                    });
                    line_start = false;
                } else {
                    c.error(file, line, &format!("unexpected byte 0x{b:02x}"));
                    if b >= 0x80 {
                        c.hit_at("lex.non_ascii.code", file, line);
                    }
                    i += 1;
                }
            }
        }
    }
    out.push(Token {
        kind: Kind::Eof,
        text: String::new(),
        line,
    });
    out
}

fn escape_name(e: u8) -> String {
    match e {
        b'n' => "n".into(),
        b't' => "t".into(),
        b'"' => "quote".into(),
        b'\\' => "backslash".into(),
        b'x' => "x".into(),
        _ if e.is_ascii_alphanumeric() => (e as char).to_string(),
        _ => format!("0x{e:02x}"),
    }
}

/// Scan a number at `*i` and name its form, e.g. `float.digits_dot_digits_f`.
fn number(src: &[u8], i: &mut usize) -> (Kind, String) {
    let at = |j: usize| src.get(j).copied().unwrap_or(0);
    let digits = |i: &mut usize| {
        let s = *i;
        while at(*i).is_ascii_digit() {
            *i += 1;
        }
        *i - s
    };
    if at(*i) == b'0' && matches!(at(*i + 1), b'x' | b'X') {
        *i += 2;
        while at(*i).is_ascii_hexdigit() {
            *i += 1;
        }
        return (Kind::Int, "int.hex".into());
    }
    let int_digits_start = *i;
    let whole = digits(i);
    let leading_zero = whole > 1 && at(int_digits_start) == b'0';
    let mut form = String::new();
    let mut is_float = false;
    if whole > 0 {
        form.push_str("digits");
    }
    if at(*i) == b'.' {
        *i += 1;
        is_float = true;
        form.push_str("_dot");
        if digits(i) > 0 {
            form.push_str("_digits");
        }
    }
    if matches!(at(*i), b'e' | b'E')
        && (at(*i + 1).is_ascii_digit()
            || (matches!(at(*i + 1), b'+' | b'-') && at(*i + 2).is_ascii_digit()))
    {
        *i += 2;
        digits(i);
        is_float = true;
        form.push_str("_exp");
    }
    match at(*i) {
        b'f' => {
            *i += 1;
            is_float = true;
            form.push_str("_f");
        }
        b'F' => {
            *i += 1;
            is_float = true;
            form.push_str("_upper_f");
        }
        _ => {}
    }
    let form = form.trim_start_matches('_').to_string();
    if is_float {
        (Kind::Float, format!("float.{form}"))
    } else if leading_zero {
        (Kind::Int, "int.leading_zero".into())
    } else {
        (Kind::Int, "int.decimal".into())
    }
}
