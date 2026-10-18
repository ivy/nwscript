//! Values as `changes.ini` writes them: literal text, or one of the tokens
//! TSLPatcher resolves while it runs. A token is kept as a placeholder here;
//! the executor fills it in.

use serde::Serialize;
use std::fmt;

/// A value on the right of `=`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Value {
    /// Text as written, `****` included.
    Literal(String),
    /// `2DAMEMORY#`: what an earlier operation stored in memory slot `#`.
    Memory(u32),
    /// `StrRef#`: the dialog.tlk index the TLK phase gave entry `#` of `append.tlk`.
    StrRef(u32),
    /// `high()`: one more than the column's highest number (2DA only).
    High,
}

impl Value {
    /// Classify text: the tokens are matched without regard to case.
    pub fn parse(text: &str) -> Value {
        if let Some(n) = token_number(text, "2DAMEMORY") {
            return Value::Memory(n);
        }
        if let Some(n) = token_number(text, "StrRef") {
            return Value::StrRef(n);
        }
        if text.eq_ignore_ascii_case("high()") {
            return Value::High;
        }
        Value::Literal(text.to_string())
    }

    /// The token this value reads, if it is one.
    pub fn token(&self) -> Option<Token> {
        match self {
            Value::Memory(n) => Some(Token::Memory(*n)),
            Value::StrRef(n) => Some(Token::StrRef(*n)),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Literal(s) => f.write_str(s),
            Value::Memory(n) => write!(f, "2DAMEMORY{n}"),
            Value::StrRef(n) => write!(f, "StrRef{n}"),
            Value::High => f.write_str("high()"),
        }
    }
}

/// A memory token: a slot an operation stores into and later ones read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(tag = "kind", content = "slot", rename_all = "snake_case")]
pub enum Token {
    Memory(u32),
    StrRef(u32),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Memory(n) => write!(f, "2DAMEMORY{n}"),
            Token::StrRef(n) => write!(f, "StrRef{n}"),
        }
    }
}

/// `PREFIX<digits>`, case-insensitively, as the number.
pub fn token_number(text: &str, prefix: &str) -> Option<u32> {
    let text = text.trim();
    if text.len() <= prefix.len() || !text.is_char_boundary(prefix.len()) {
        return None;
    }
    let (head, tail) = text.split_at(prefix.len());
    if !head.eq_ignore_ascii_case(prefix) {
        return None;
    }
    tail.parse().ok()
}

/// `2DAMEMORY#` as a key, which stores into the slot.
pub fn memory_key(key: &str) -> Option<u32> {
    token_number(key, "2DAMEMORY")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_recognised_without_regard_to_case() {
        assert_eq!(Value::parse("2damemory12"), Value::Memory(12));
        assert_eq!(Value::parse("StrRef0"), Value::StrRef(0));
        assert_eq!(Value::parse("HIGH()"), Value::High);
        assert_eq!(Value::parse("****"), Value::Literal("****".into()));
        assert_eq!(
            Value::parse("2DAMEMORY"),
            Value::Literal("2DAMEMORY".into())
        );
        assert_eq!(Value::parse("StrRefs"), Value::Literal("StrRefs".into()));
        assert_eq!(memory_key("2DAMEMORY3"), Some(3));
        assert_eq!(memory_key("label"), None);
    }

    #[test]
    fn display_is_the_ini_spelling() {
        assert_eq!(Value::Memory(4).to_string(), "2DAMEMORY4");
        assert_eq!(Token::StrRef(7).to_string(), "StrRef7");
        assert_eq!(Value::High.to_string(), "high()");
    }
}
