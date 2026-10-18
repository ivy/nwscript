//! `Ref`: one place a resource names one of the game's symbols.
//!
//! The game's symbols are resrefs, tags, strrefs and global variable names,
//! and each format names them in its own way: a GFF's `CResRef` and
//! `CExoLocString` fields, a script's string and integer operands, a 2DA's
//! cells. A [`Ref`] is the format-neutral record of one such naming, which
//! the format crates' extractors produce and an index stores.

use std::fmt;

/// What kind of symbol a [`Ref`] names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RefKind {
    /// A resource by its [`crate::ResRef`]; the value is lowercase.
    ResRef,
    /// A line of the talk table; the value is the strref in decimal.
    StrRef,
    /// An object's tag, as written.
    Tag,
    /// A global variable's name (`globalcat.2da`, `GetGlobal*`), as written.
    Global,
}

impl RefKind {
    /// Every kind, in order.
    pub const ALL: [RefKind; 4] = [
        RefKind::ResRef,
        RefKind::StrRef,
        RefKind::Tag,
        RefKind::Global,
    ];

    /// `resref`, `strref`, `tag` or `global`.
    pub const fn name(self) -> &'static str {
        match self {
            RefKind::ResRef => "resref",
            RefKind::StrRef => "strref",
            RefKind::Tag => "tag",
            RefKind::Global => "global",
        }
    }
}

impl fmt::Display for RefKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One naming of a symbol: its kind, the symbol, and the site within the
/// resource that names it (a GFF field path, a script offset and routine,
/// a 2DA row and column), in the form the extractor documents.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ref {
    pub kind: RefKind,
    pub value: String,
    pub site: String,
}

impl Ref {
    /// A reference to a resource. The name is lowercased, since resrefs
    /// compare without regard to case.
    pub fn resref(name: &str, site: impl Into<String>) -> Self {
        Ref {
            kind: RefKind::ResRef,
            value: name.to_ascii_lowercase(),
            site: site.into(),
        }
    }

    /// A reference to a line of the talk table, by its 32-bit value.
    pub fn strref(raw: u32, site: impl Into<String>) -> Self {
        Ref {
            kind: RefKind::StrRef,
            value: raw.to_string(),
            site: site.into(),
        }
    }

    /// A reference to a tag.
    pub fn tag(tag: impl Into<String>, site: impl Into<String>) -> Self {
        Ref {
            kind: RefKind::Tag,
            value: tag.into(),
            site: site.into(),
        }
    }

    /// A reference to a global variable.
    pub fn global(name: impl Into<String>, site: impl Into<String>) -> Self {
        Ref {
            kind: RefKind::Global,
            value: name.into(),
            site: site.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resrefs_are_lowercased_and_strrefs_decimal() {
        let r = Ref::resref("K_PMAN_AIOFF", "Script");
        assert_eq!(r.value, "k_pman_aioff");
        assert_eq!(r.kind.name(), "resref");
        assert_eq!(Ref::strref(42724, "Text").value, "42724");
        assert_eq!(Ref::tag("Juhani", "Tag").value, "Juhani");
        assert_eq!(
            RefKind::ALL.map(RefKind::name),
            ["resref", "strref", "tag", "global"]
        );
    }
}
