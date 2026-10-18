//! `TexturePack`: which of the quality packs under `TexturePacks/` the
//! install's texture layer searches.

use std::fmt;
use std::str::FromStr;

use crate::Error;

/// The folder the texture packs lie in, beside `chitin.key`.
pub(crate) const DIR: &str = "TexturePacks";

/// The pack of interface textures, searched after the quality pack.
pub(crate) const GUI: &str = "swpc_tex_gui.erf";

/// One of the three quality packs: `swpc_tex_tpa.erf`, `swpc_tex_tpb.erf`,
/// `swpc_tex_tpc.erf`. The vanilla KotOR 1 install's three hold the same
/// 3,294 names, all TPC, at full size in `tpa`, half in `tpb` and a quarter
/// in `tpc` for most; the game reads one of them, by its `Texture Quality`
/// option. Which value selects which pack is not established, so the
/// default is a stated choice, [`TexturePack::DEFAULT`], that a caller may
/// override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TexturePack {
    /// `swpc_tex_tpa.erf`, the largest textures.
    Tpa,
    /// `swpc_tex_tpb.erf`.
    Tpb,
    /// `swpc_tex_tpc.erf`, the smallest textures.
    Tpc,
}

impl TexturePack {
    /// Every quality pack, largest first.
    pub const ALL: [TexturePack; 3] = [TexturePack::Tpa, TexturePack::Tpb, TexturePack::Tpc];

    /// The pack searched when the caller selects none: `tpa`, the one the
    /// asset census reads and OdysseyUE's runtime loads. A choice, not a
    /// measurement of what the game selects.
    pub const DEFAULT: TexturePack = TexturePack::Tpa;

    /// The pack's short name, as `--texture-pack` spells it: `tpa`.
    pub const fn name(self) -> &'static str {
        match self {
            TexturePack::Tpa => "tpa",
            TexturePack::Tpb => "tpb",
            TexturePack::Tpc => "tpc",
        }
    }

    /// The pack's file under `TexturePacks/`: `swpc_tex_tpa.erf`.
    pub const fn file_name(self) -> &'static str {
        match self {
            TexturePack::Tpa => "swpc_tex_tpa.erf",
            TexturePack::Tpb => "swpc_tex_tpb.erf",
            TexturePack::Tpc => "swpc_tex_tpc.erf",
        }
    }
}

impl Default for TexturePack {
    fn default() -> Self {
        TexturePack::DEFAULT
    }
}

impl fmt::Display for TexturePack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for TexturePack {
    type Err = Error;

    /// `tpa`, `tpb` or `tpc`, in any case; the file name is accepted too.
    /// Anything else is [`Error::UnknownTexturePack`].
    fn from_str(s: &str) -> Result<Self, Error> {
        let lower = s.to_ascii_lowercase();
        TexturePack::ALL
            .into_iter()
            .find(|p| lower == p.name() || lower == p.file_name())
            .ok_or_else(|| Error::UnknownTexturePack { name: s.to_owned() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pack_parses_from_its_name_or_file_in_any_case() {
        assert_eq!("tpb".parse::<TexturePack>().unwrap(), TexturePack::Tpb);
        assert_eq!("TPC".parse::<TexturePack>().unwrap(), TexturePack::Tpc);
        assert_eq!(
            "swpc_tex_tpa.erf".parse::<TexturePack>().unwrap(),
            TexturePack::Tpa
        );
        let e = "tpd".parse::<TexturePack>().unwrap_err();
        assert_eq!(e.code(), crate::codes::UNKNOWN_TEXTURE_PACK);
        assert_eq!(TexturePack::default(), TexturePack::Tpa);
    }
}
