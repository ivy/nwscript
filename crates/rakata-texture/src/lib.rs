//! The textures of BioWare's Aurora-family games, KotOR 1 and 2 first.
//!
//! - [`Tpc`], the engine's own texture: its header, its pixel data cut into
//!   images (one, six cube faces, or the frames of an animation sheet) and
//!   their mip levels with the pixel format kept raw, and the [`Txi`] text
//!   it carries after the pixels. DXT1 and DXT5 levels are the blocks as
//!   stored, ready for a BC1 or BC3 upload; grey, RGB and RGBA levels are
//!   the pixels as stored.
//! - [`Txi`], a texture's instructions as keywords and values, from a TPC
//!   or a standalone `.txi`.
//! - [`Tga`], a Truevision TARGA image (the game's lightmaps), decoded to
//!   RGBA8 with rows top first.
//!
//! Each has a `parse(&[u8])` entry point and nothing else to call.
//!
//! A [`Tpc`] borrows: each [`Level`]'s `data` and the TXI bytes are slices
//! of the input, so a caller can hand a level to a GPU without a copy, and
//! the input must outlive the `Tpc`. A [`Tga`] owns its pixels, since
//! decoding to RGBA8 top-first reorders and widens them in every vanilla
//! file (each is bottom-left, BGR or BGRA, or grey). A [`Txi`] owns its
//! strings.
//!
//! The layouts are `docs/formats/tpc.md` and `docs/formats/tga.md` of this
//! repository, measured from the game's files; this crate was written
//! without sight of any other implementation.

mod error;
mod tga;
mod tpc;
mod txi;

pub use error::{Error, Result, codes};
pub use tga::{ImageType, Origin, Tga};
pub use tpc::{Image, Layout, Level, PixelFormat, Tpc};
pub use txi::{Txi, TxiEntry};

/// The TPC header's length; the pixel data begins at this offset.
pub const TPC_HEADER_LEN: usize = tpc::HEADER_LEN;
/// The TGA header's length.
pub const TGA_HEADER_LEN: usize = tga::HEADER_LEN;
