//! BioWare's Generic File Format, version 3.2: the container of KotOR's
//! blueprints (UTC, UTI, UTP, UTD, UTT, UTE, UTM, UTS, UTW), areas (ARE,
//! GIT, IFO), dialogues (DLG), journals, factions, GUI layouts, paths and the
//! members of a save.
//!
//! A [`Gff`] is a tree: a root [`Struct`] of labelled [`Field`]s whose
//! [`Value`]s are the nineteen types the engine stores, nested structs and
//! lists among them. Reading keeps everything a byte-identical write needs
//! and the tree alone does not say (the label table, each struct's and
//! field's position in the file's arrays, and the stale index lists
//! BioWare's incremental writer leaves behind), so a file read from the
//! game writes back as the bytes it came from; a file built with the API
//! writes in the one-pass layout BioWare's tool uses. A [`Path`] names one
//! field (`EntryList[3].RepliesList[0].Index`) for [`Gff::get`] and
//! [`Gff::set`]; [`diff`] lists the paths on which two files differ.
//!
//! Strings are kept as bytes in the game's code page, floats by their bits,
//! and a `CExoLocString`'s strref as the `u32` the file holds, `0xFFFFFFFF`
//! meaning none. The layout follows BioWare's GFF document as xoreos
//! publishes it and the measured facts in `docs/formats/gff.md`; this crate
//! was written without sight of any other implementation.

#![forbid(unsafe_code)]

mod diff;
mod error;
pub mod facts;
mod model;
mod path;
mod read;
mod survey;
mod write;

pub use diff::{Change, diff};
pub use error::{Error, Result, codes};
pub use model::{Field, FieldType, Gff, LABEL_LEN, Label, LocString, Struct, SubString, Value};
pub use path::{Path, Segment};
pub use read::Header;
pub use survey::{Survey, survey};
