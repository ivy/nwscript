//! Localised text as the engine addresses it: a [`StrRef`] into the talk
//! table, a [`Language`] and [`Gender`] that pick one table and one form of
//! a line, and the [`LocalizedString`] (BioWare's `CExoLocString`) a GFF
//! field holds, which names a strref and may embed its own text per
//! language and gender.
//!
//! The layout facts are those of BioWare's Talk Table and Localized Strings
//! documents (xoreos-docs, `specs/bioware/`): a strref is 32 bits with all
//! bits set for "none" and only the low 24 bits indexing the table, and a
//! GFF stores a substring's language and gender as one id, the language
//! times two plus the gender.

use std::fmt;

/// A reference into the talk table: an index into `dialog.tlk`.
///
/// `0xFFFFFFFF` means no string. Of the remaining bits only the low 24
/// index the table; bit `0x01000000` asks for a module's alternate table,
/// and the rest are masked off by the engine.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrRef(u32);

impl StrRef {
    /// The value that refers to no string.
    pub const NONE: StrRef = StrRef(u32::MAX);

    /// The bit that selects a module's alternate talk table.
    pub const ALTERNATE: u32 = 0x0100_0000;

    /// The bits that index the table.
    const INDEX_MASK: u32 = 0x00FF_FFFF;

    /// A strref from the 32-bit value a file stores.
    pub const fn new(raw: u32) -> Self {
        StrRef(raw)
    }

    /// The 32-bit value a file stores.
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Whether the strref names a string at all: anything but `NONE`.
    pub const fn is_set(self) -> bool {
        self.0 != u32::MAX
    }

    /// The index into the table, when set: the low 24 bits.
    pub const fn index(self) -> Option<usize> {
        if self.is_set() {
            Some((self.0 & Self::INDEX_MASK) as usize)
        } else {
            None
        }
    }

    /// Whether the strref asks for the module's alternate talk table.
    pub const fn is_alternate(self) -> bool {
        self.is_set() && self.0 & Self::ALTERNATE != 0
    }
}

impl Default for StrRef {
    /// No string.
    fn default() -> Self {
        StrRef::NONE
    }
}

impl From<u32> for StrRef {
    fn from(raw: u32) -> Self {
        StrRef(raw)
    }
}

impl From<StrRef> for u32 {
    fn from(s: StrRef) -> Self {
        s.0
    }
}

impl fmt::Debug for StrRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_set() {
            write!(f, "StrRef({})", self.0)
        } else {
            f.write_str("StrRef(NONE)")
        }
    }
}

impl fmt::Display for StrRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_set() {
            write!(f, "{}", self.0)
        } else {
            f.write_str("-1")
        }
    }
}

/// A language the talk table and the localised strings name by id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Language {
    English,
    French,
    German,
    Italian,
    Spanish,
    Polish,
    Korean,
    ChineseTraditional,
    ChineseSimplified,
    Japanese,
    /// An id the documents do not list, kept as read.
    Other(u32),
}

impl Language {
    /// The languages BioWare's documents list, in the order the engine
    /// searches a localised string's substrings when the user's own
    /// language has none.
    pub const FALLBACK_ORDER: [Language; 5] = [
        Language::English,
        Language::French,
        Language::German,
        Language::Italian,
        Language::Spanish,
    ];

    /// The language with this id, as the talk table header stores it.
    pub const fn from_id(id: u32) -> Self {
        match id {
            0 => Language::English,
            1 => Language::French,
            2 => Language::German,
            3 => Language::Italian,
            4 => Language::Spanish,
            5 => Language::Polish,
            128 => Language::Korean,
            129 => Language::ChineseTraditional,
            130 => Language::ChineseSimplified,
            131 => Language::Japanese,
            other => Language::Other(other),
        }
    }

    /// The id the talk table header stores.
    pub const fn id(self) -> u32 {
        match self {
            Language::English => 0,
            Language::French => 1,
            Language::German => 2,
            Language::Italian => 3,
            Language::Spanish => 4,
            Language::Polish => 5,
            Language::Korean => 128,
            Language::ChineseTraditional => 129,
            Language::ChineseSimplified => 130,
            Language::Japanese => 131,
            Language::Other(id) => id,
        }
    }
}

/// The gender a form of a line is written for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Gender {
    /// Masculine or neutral: `dialog.tlk`, and gender 0 in a GFF.
    #[default]
    Male,
    /// Feminine: `dialogf.tlk`, and gender 1 in a GFF.
    Female,
}

impl Gender {
    /// The gender bit of a GFF string id.
    pub const fn from_bit(bit: u32) -> Self {
        if bit & 1 == 0 {
            Gender::Male
        } else {
            Gender::Female
        }
    }

    /// The gender bit of a GFF string id.
    pub const fn bit(self) -> u32 {
        match self {
            Gender::Male => 0,
            Gender::Female => 1,
        }
    }
}

/// One embedded form of a localised string: its language, gender and text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Substring {
    pub language: Language,
    pub gender: Gender,
    pub text: String,
}

impl Substring {
    /// The id a GFF stores for this substring: the language times two plus
    /// the gender.
    pub const fn string_id(&self) -> u32 {
        string_id(self.language, self.gender)
    }
}

/// The id a GFF stores for a language and gender: the language times two
/// plus the gender bit.
pub const fn string_id(language: Language, gender: Gender) -> u32 {
    language.id() * 2 + gender.bit()
}

/// The language and gender a GFF string id names.
pub const fn from_string_id(id: u32) -> (Language, Gender) {
    (Language::from_id(id / 2), Gender::from_bit(id))
}

/// BioWare's `CExoLocString`: a strref into the talk table and any number of
/// embedded substrings, each for one language and gender. Substrings are
/// kept in the order read, so a GFF writes them back as it found them.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct LocalizedString {
    pub strref: StrRef,
    pub substrings: Vec<Substring>,
}

impl LocalizedString {
    /// A string that refers to nothing and embeds nothing.
    pub const EMPTY: LocalizedString = LocalizedString {
        strref: StrRef::NONE,
        substrings: Vec::new(),
    };

    /// A string that refers to the talk table alone.
    pub const fn from_strref(strref: StrRef) -> Self {
        LocalizedString {
            strref,
            substrings: Vec::new(),
        }
    }

    /// A string that embeds one text and refers to no talk table entry.
    pub fn embedded(language: Language, gender: Gender, text: impl Into<String>) -> Self {
        let mut s = Self::EMPTY;
        s.set_substring(language, gender, text);
        s
    }

    /// The embedded text for a language and gender, if any.
    pub fn substring(&self, language: Language, gender: Gender) -> Option<&str> {
        self.substrings
            .iter()
            .find(|s| s.language == language && s.gender == gender)
            .map(|s| s.text.as_str())
    }

    /// Set the embedded text for a language and gender, replacing any
    /// already there and keeping its position; a new one goes last.
    pub fn set_substring(&mut self, language: Language, gender: Gender, text: impl Into<String>) {
        let text = text.into();
        match self
            .substrings
            .iter_mut()
            .find(|s| s.language == language && s.gender == gender)
        {
            Some(s) => s.text = text,
            None => self.substrings.push(Substring {
                language,
                gender,
                text,
            }),
        }
    }

    /// Whether the string has neither a strref nor any substring.
    pub fn is_empty(&self) -> bool {
        !self.strref.is_set() && self.substrings.is_empty()
    }

    /// The text to show, by the procedure of BioWare's Localized Strings
    /// document: the substring for `language` and `gender`; else the talk
    /// table's text for the strref, through `table` (the caller passes the
    /// table for the gender, `dialog.tlk` or `dialogf.tlk`); else the first
    /// substring of the same gender in a fallback language, English, French,
    /// German, Italian and Spanish in that order; else none.
    pub fn resolve_with<'a, F>(
        &'a self,
        language: Language,
        gender: Gender,
        table: F,
    ) -> Option<&'a str>
    where
        F: FnOnce(StrRef) -> Option<&'a str>,
    {
        if let Some(text) = self.substring(language, gender) {
            return Some(text);
        }
        if self.strref.is_set()
            && let Some(text) = table(self.strref)
        {
            return Some(text);
        }
        Language::FALLBACK_ORDER
            .iter()
            .filter(|&&l| l != language)
            .find_map(|&l| self.substring(l, gender))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strref_bits() {
        assert!(!StrRef::NONE.is_set());
        assert_eq!(StrRef::NONE.index(), None);
        assert_eq!(StrRef::new(5).index(), Some(5));
        let alt = StrRef::new(0x0100_0005);
        assert!(alt.is_alternate());
        assert_eq!(alt.index(), Some(5));
        assert_eq!(StrRef::new(0x1000_0001).index(), Some(1));
        assert_eq!(StrRef::new(42).to_string(), "42");
        assert_eq!(StrRef::NONE.to_string(), "-1");
    }

    #[test]
    fn language_ids_round_trip() {
        for id in [0, 1, 2, 3, 4, 5, 128, 129, 130, 131, 77] {
            assert_eq!(Language::from_id(id).id(), id);
        }
        assert_eq!(Language::from_id(77), Language::Other(77));
    }

    #[test]
    fn string_ids() {
        assert_eq!(string_id(Language::English, Gender::Male), 0);
        assert_eq!(string_id(Language::English, Gender::Female), 1);
        assert_eq!(string_id(Language::German, Gender::Female), 5);
        assert_eq!(from_string_id(5), (Language::German, Gender::Female));
        assert_eq!(from_string_id(262), (Language::Japanese, Gender::Male));
    }

    #[test]
    fn resolve_prefers_substring_then_table_then_fallback() {
        let mut s = LocalizedString::from_strref(StrRef::new(7));
        assert_eq!(
            s.resolve_with(Language::English, Gender::Male, |r| (r == StrRef::new(7))
                .then_some("from the table")),
            Some("from the table")
        );
        s.set_substring(Language::English, Gender::Male, "embedded");
        assert_eq!(
            s.resolve_with(Language::English, Gender::Male, |r| (r == StrRef::new(7))
                .then_some("from the table")),
            Some("embedded")
        );
        let s = LocalizedString::embedded(Language::French, Gender::Female, "fr");
        assert_eq!(
            s.resolve_with(Language::German, Gender::Female, |_| None),
            Some("fr")
        );
        assert_eq!(
            s.resolve_with(Language::German, Gender::Male, |_| None),
            None
        );
        assert_eq!(
            LocalizedString::EMPTY.resolve_with(Language::English, Gender::Male, |_| Some("x")),
            None
        );
    }

    #[test]
    fn set_substring_replaces_in_place() {
        let mut s = LocalizedString::embedded(Language::English, Gender::Male, "a");
        s.set_substring(Language::French, Gender::Male, "b");
        s.set_substring(Language::English, Gender::Male, "c");
        assert_eq!(s.substrings.len(), 2);
        assert_eq!(s.substrings[0].text, "c");
        assert_eq!(s.substring(Language::French, Gender::Male), Some("b"));
    }
}
