//! `Profile`: which game, and the folders and capsule names that follow.

/// The game an install belongs to. The two differ in which capsules a
/// module ships as and whether a `patch.erf` sits over the BIFs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profile {
    /// Knights of the Old Republic.
    K1,
    /// Knights of the Old Republic II: The Sith Lords.
    K2,
}

impl Profile {
    /// The suffixes a module's capsules carry under `modules/`, in the
    /// engine's precedence: `STEM.mod` replaces the pair `STEM.rim` and
    /// `STEM_s.rim` when present; KotOR 2 adds `STEM_dlg.erf` for the
    /// dialogues.
    pub const fn module_suffixes(self) -> &'static [&'static str] {
        match self {
            Profile::K1 => &[".mod", ".rim", "_s.rim"],
            Profile::K2 => &[".mod", ".rim", "_s.rim", "_dlg.erf"],
        }
    }

    /// The suffix of a module's lip-sync pack under `lips/`.
    pub const fn lips_suffix(self) -> &'static str {
        "_loc.mod"
    }

    /// Whether the install carries a `patch.erf` beside `chitin.key`,
    /// which KotOR 1's updates shipped as a layer over the BIFs.
    pub const fn has_patch_erf(self) -> bool {
        matches!(self, Profile::K1)
    }

    /// The profile's short name, as `--profile` spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Profile::K1 => "k1",
            Profile::K2 => "k2",
        }
    }
}
