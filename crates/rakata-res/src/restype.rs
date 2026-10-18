//! `ResType`: the resource type table, id to extension and back.
//!
//! The ids and extensions are those of BioWare's KEY/BIF format document
//! (the table in its "Resource Types" section, which covers Neverwinter
//! Nights) and xoreos's resource-type documentation, which adds the
//! Odyssey-era types KotOR 1 and 2 ship (`LYT`, `VIS`, `RIM`, `PTH`, `LIP`,
//! `TPC`, `MDX` and their neighbours in the 3000 range). Ids this table does
//! not name are kept as [`ResType::Unknown`], never rejected, so an archive
//! with a type this crate has not met still reads and writes.

use std::fmt;

macro_rules! res_types {
    ($( $variant:ident = $id:literal, $ext:literal, $doc:literal; )*) => {
        /// A resource type: the numeric id the archives store, paired with
        /// the file extension loose files carry.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[allow(clippy::upper_case_acronyms)]
        pub enum ResType {
            $(
                #[doc = concat!("`.", $ext, "`, id ", stringify!($id), ": ", $doc)]
                $variant,
            )*
            /// An id this table does not name, kept as read.
            Unknown(u16),
        }

        impl ResType {
            /// Every type this table names, in id order.
            pub const KNOWN: &'static [ResType] = &[$(ResType::$variant),*];

            /// The type for an id as an archive stores it. An id this table
            /// does not name becomes [`ResType::Unknown`].
            pub const fn from_id(id: u16) -> Self {
                match id {
                    $( $id => ResType::$variant, )*
                    other => ResType::Unknown(other),
                }
            }

            /// The id an archive stores for this type.
            pub const fn id(self) -> u16 {
                match self {
                    $( ResType::$variant => $id, )*
                    ResType::Unknown(id) => id,
                }
            }

            /// The file extension, lowercase and without its dot; `None`
            /// for an unknown id.
            pub const fn extension(self) -> Option<&'static str> {
                match self {
                    $( ResType::$variant => Some($ext), )*
                    ResType::Unknown(_) => None,
                }
            }

            /// The type for a file extension, with or without its leading
            /// dot and in any case; `None` when no known type carries it.
            pub fn from_extension(ext: &str) -> Option<Self> {
                let ext = ext.strip_prefix('.').unwrap_or(ext);
                $(
                    if ext.eq_ignore_ascii_case($ext) {
                        return Some(ResType::$variant);
                    }
                )*
                None
            }
        }
    };
}

res_types! {
    Res = 0, "res", "a generic GFF";
    Bmp = 1, "bmp", "a Windows bitmap image";
    Mve = 2, "mve", "an Infinity Engine video";
    Tga = 3, "tga", "a Truevision TARGA image";
    Wav = 4, "wav", "a waveform audio file";
    Plt = 6, "plt", "a packed layered texture";
    Ini = 7, "ini", "a Windows INI configuration file";
    Bmu = 8, "bmu", "MP3 audio with a BioWare header";
    Mpg = 9, "mpg", "an MPEG video";
    Txt = 10, "txt", "plain text";
    Wma = 11, "wma", "Windows Media audio";
    Wmv = 12, "wmv", "Windows Media video";
    Xmv = 13, "xmv", "an Xbox video";
    Plh = 2000, "plh", "a placeholder";
    Tex = 2001, "tex", "a texture";
    Mdl = 2002, "mdl", "an Aurora model";
    Thg = 2003, "thg", "a thing";
    Fnt = 2005, "fnt", "a font";
    Lua = 2007, "lua", "Lua source";
    Slt = 2008, "slt", "a slot";
    Nss = 2009, "nss", "NWScript source";
    Ncs = 2010, "ncs", "NWScript compiled bytecode";
    Mod = 2011, "mod", "a module, an ERF";
    Are = 2012, "are", "an area's static data, GFF";
    Set = 2013, "set", "a tileset";
    Ifo = 2014, "ifo", "module information, GFF";
    Bic = 2015, "bic", "a player character, GFF";
    Wok = 2016, "wok", "an area walkmesh";
    TwoDa = 2017, "2da", "a two-dimensional array";
    Tlk = 2018, "tlk", "a talk table";
    Txi = 2022, "txi", "extra texture information";
    Git = 2023, "git", "an area's instances, GFF";
    Bti = 2024, "bti", "an item blueprint (BioWare), GFF";
    Uti = 2025, "uti", "an item blueprint, GFF";
    Btc = 2026, "btc", "a creature blueprint (BioWare), GFF";
    Utc = 2027, "utc", "a creature blueprint, GFF";
    Dlg = 2029, "dlg", "a conversation, GFF";
    Itp = 2030, "itp", "a palette, GFF";
    Btt = 2031, "btt", "a trigger blueprint (BioWare), GFF";
    Utt = 2032, "utt", "a trigger blueprint, GFF";
    Dds = 2033, "dds", "a DirectDraw Surface texture";
    Bts = 2034, "bts", "a sound blueprint (BioWare), GFF";
    Uts = 2035, "uts", "a sound blueprint, GFF";
    Ltr = 2036, "ltr", "letter-combination probabilities for name generation";
    Gff = 2037, "gff", "a generic GFF";
    Fac = 2038, "fac", "factions, GFF";
    Bte = 2039, "bte", "an encounter blueprint (BioWare), GFF";
    Ute = 2040, "ute", "an encounter blueprint, GFF";
    Btd = 2041, "btd", "a door blueprint (BioWare), GFF";
    Utd = 2042, "utd", "a door blueprint, GFF";
    Btp = 2043, "btp", "a placeable blueprint (BioWare), GFF";
    Utp = 2044, "utp", "a placeable blueprint, GFF";
    Dft = 2045, "dft", "default values, INI";
    Gic = 2046, "gic", "instance comments, GFF";
    Gui = 2047, "gui", "a GUI layout, GFF";
    Css = 2048, "css", "conditional script source";
    Ccs = 2049, "ccs", "conditional script compiled";
    Btm = 2050, "btm", "a store blueprint (BioWare), GFF";
    Utm = 2051, "utm", "a store blueprint, GFF";
    Dwk = 2052, "dwk", "a door walkmesh";
    Pwk = 2053, "pwk", "a placeable walkmesh";
    Btg = 2054, "btg", "an item generator blueprint (BioWare), GFF";
    Utg = 2055, "utg", "an item generator blueprint, GFF";
    Jrl = 2056, "jrl", "the journal, GFF";
    Sav = 2057, "sav", "a saved game, an ERF";
    Utw = 2058, "utw", "a waypoint blueprint, GFF";
    FourPc = 2059, "4pc", "a 16-bit RGBA texture";
    Ssf = 2060, "ssf", "a sound set";
    Hak = 2061, "hak", "a hak pak, an ERF";
    Nwm = 2062, "nwm", "an original-campaign module, an ERF";
    Bik = 2063, "bik", "a Bink video";
    Ndb = 2064, "ndb", "script debugger data";
    Ptm = 2065, "ptm", "a plot manager instance, GFF";
    Ptt = 2066, "ptt", "a plot wizard blueprint, GFF";
    Lyt = 3000, "lyt", "an area's room layout";
    Vis = 3001, "vis", "an area's room visibility";
    Rim = 3002, "rim", "a module's resources, a RIM";
    Pth = 3003, "pth", "path-finding data, GFF";
    Lip = 3004, "lip", "lip-sync data";
    Bwm = 3005, "bwm", "a binary walkmesh";
    Txb = 3006, "txb", "a texture";
    Tpc = 3007, "tpc", "an Odyssey texture";
    Mdx = 3008, "mdx", "a model's mesh data";
    Rsv = 3009, "rsv", "reserved";
    Sig = 3010, "sig", "a signature";
    Erf = 9997, "erf", "an encapsulated resource file";
    Bif = 9998, "bif", "a BIF of game resources";
    Key = 9999, "key", "the index over the BIFs";
}

impl ResType {
    /// Whether this is a type the table names.
    pub const fn is_known(self) -> bool {
        !matches!(self, ResType::Unknown(_))
    }
}

impl From<u16> for ResType {
    fn from(id: u16) -> Self {
        Self::from_id(id)
    }
}

impl From<ResType> for u16 {
    fn from(ty: ResType) -> Self {
        ty.id()
    }
}

impl fmt::Display for ResType {
    /// The extension for a known type; `type<id>` for an unknown one.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.extension() {
            Some(ext) => f.write_str(ext),
            None => write!(f, "type{}", self.id()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn the_two_ids_xtask_knows_agree() {
        assert_eq!(ResType::from_id(2009), ResType::Nss);
        assert_eq!(ResType::from_id(2010), ResType::Ncs);
        assert_eq!(ResType::Nss.extension(), Some("nss"));
        assert_eq!(ResType::Ncs.id(), 2010);
    }

    #[test]
    fn ids_and_extensions_round_trip() {
        for &ty in ResType::KNOWN {
            assert_eq!(ResType::from_id(ty.id()), ty);
            let ext = ty.extension().unwrap();
            assert_eq!(ResType::from_extension(ext), Some(ty), "{ext}");
            assert_eq!(ty.to_string(), ext);
        }
    }

    #[test]
    fn ids_and_extensions_are_unique() {
        let ids: HashSet<u16> = ResType::KNOWN.iter().map(|t| t.id()).collect();
        assert_eq!(ids.len(), ResType::KNOWN.len());
        let exts: HashSet<&str> = ResType::KNOWN
            .iter()
            .filter_map(|t| t.extension())
            .collect();
        assert_eq!(exts.len(), ResType::KNOWN.len());
    }

    #[test]
    fn extension_lookup_ignores_case_and_dot() {
        assert_eq!(ResType::from_extension("UTC"), Some(ResType::Utc));
        assert_eq!(ResType::from_extension(".Tpc"), Some(ResType::Tpc));
        assert_eq!(ResType::from_extension("2DA"), Some(ResType::TwoDa));
        assert_eq!(ResType::from_extension("nope"), None);
    }

    #[test]
    fn unknown_ids_are_kept() {
        let ty = ResType::from_id(4242);
        assert_eq!(ty, ResType::Unknown(4242));
        assert_eq!(ty.id(), 4242);
        assert_eq!(ty.extension(), None);
        assert!(!ty.is_known());
        assert_eq!(ty.to_string(), "type4242");
        assert_eq!(u16::from(ty), 4242);
    }
}
