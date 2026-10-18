//! What a talk table contributes to an index: one [`StringFact`] per entry
//! that has text or a voice-over, and a resref [`Ref`] for each voice-over.
//!
//! An entry with neither text nor a sound resref is the table's padding and
//! has no fact. A voice-over's site is `strref N`. The function is pure: it
//! reads the table and nothing else.

use rakata_res::{Ref, ResRef};

use crate::Tlk;

/// One entry of the table.
#[derive(Debug, Clone, PartialEq)]
pub struct StringFact<'a> {
    /// The entry's strref: its index in the table.
    pub strref: u32,
    /// The text, empty when the entry has none.
    pub text: &'a str,
    /// The voice-over's resref.
    pub sound: Option<ResRef>,
    pub sound_length: f32,
    pub flags: u32,
}

/// The facts of one table.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facts<'a> {
    pub strings: Vec<StringFact<'a>>,
    pub refs: Vec<Ref>,
}

/// The facts of `tlk`.
pub fn facts(tlk: &Tlk) -> Facts<'_> {
    let mut out = Facts::default();
    for (i, e) in tlk.entries.iter().enumerate() {
        let strref = i as u32;
        let text = e.text.as_str();
        if text.is_empty() && e.sound.is_none() {
            continue;
        }
        if let Some(sound) = e.sound {
            let name = String::from_utf8_lossy(sound.as_bytes()).into_owned();
            out.refs
                .push(Ref::resref(&name, format!("strref {strref}")));
        }
        out.strings.push(StringFact {
            strref,
            text,
            sound: e.sound,
            sound_length: e.sound_length,
            flags: e.flags,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Entry;
    use rakata_res::{Language, RefKind};

    #[test]
    fn entries_with_text_or_sound_are_facts() {
        let mut tlk = Tlk::new(Language::English);
        tlk.append(Entry::new("Bad Dog"));
        tlk.append(Entry {
            flags: 0,
            ..Entry::new("")
        });
        tlk.append(Entry::new("I am Juhani.").with_sound(ResRef::new("N_Juhani_01").unwrap(), 2.0));
        let f = facts(&tlk);
        let got: Vec<(u32, &str)> = f.strings.iter().map(|s| (s.strref, s.text)).collect();
        assert_eq!(got, [(0, "Bad Dog"), (2, "I am Juhani.")]);
        assert_eq!(f.refs.len(), 1);
        assert_eq!(f.refs[0].kind, RefKind::ResRef);
        assert_eq!(f.refs[0].value, "n_juhani_01");
        assert_eq!(f.refs[0].site, "strref 2");
    }
}
