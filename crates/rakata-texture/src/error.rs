//! The errors this crate reports, each with a registered diagnostic code.

use std::fmt;

/// Diagnostic codes for textures: the `RE22xx` range of the `rakata`
/// registry. Each is a stable string a caller may match on and a user may
/// look up.
// Registered, with explanations, in `crates/rakata/src/codes.rs`; this crate
// carries only the strings.
pub mod codes {
    /// A TPC ends before its header or its pixel data does.
    pub const TPC_TRUNCATED: &str = "RE2201";
    /// A TPC's encoding byte names no pixel format this crate knows.
    pub const TPC_UNKNOWN_FORMAT: &str = "RE2202";
    /// A TPC's header describes no layout of mip levels the bytes can hold.
    pub const TPC_BAD_LAYOUT: &str = "RE2203";
    /// A TGA ends before its header or its pixels do.
    pub const TGA_TRUNCATED: &str = "RE2204";
    /// A TGA of an image type or pixel depth this crate does not decode.
    pub const TGA_UNSUPPORTED: &str = "RE2205";
    /// A run-length packet of a TGA runs past the image's last pixel.
    pub const TGA_RLE_OVERRUN: &str = "RE2206";
    /// A TPC the writer cannot write as given.
    pub const TPC_UNWRITABLE: &str = "RE2207";
    /// A TXI line the writer cannot write so that it reads back the same.
    pub const TXI_UNWRITABLE: &str = "RE2208";
    /// A TGA the writer cannot write as given.
    pub const TGA_UNWRITABLE: &str = "RE2209";
    /// A mip level the texture does not have.
    pub const LEVEL_MISSING: &str = "RE2210";
    /// An image (a cube face or an animation frame) the texture does not
    /// have.
    pub const IMAGE_MISSING: &str = "RE2211";
    /// Pixels the PNG encoder cannot write as given.
    pub const PNG_UNWRITABLE: &str = "RE2212";
}

/// An error in reading or writing a texture.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// A TPC ends before its header or its pixel data does.
    TpcTruncated {
        /// What was being read.
        what: &'static str,
        /// The length the layout requires.
        needed: usize,
        /// The length of the bytes.
        available: usize,
    },
    /// A TPC's encoding byte, with the data-size field zero or not, names
    /// no pixel format.
    TpcUnknownFormat {
        /// The encoding byte at offset 12.
        encoding: u8,
        /// Whether the data-size field at offset 0 is non-zero.
        compressed: bool,
    },
    /// A TPC's header describes no layout the bytes can hold.
    TpcBadLayout {
        /// Why.
        reason: String,
    },
    /// A TGA ends before its header or its pixels do.
    TgaTruncated {
        /// What was being read.
        what: &'static str,
        /// The length the layout requires.
        needed: usize,
        /// The length of the bytes.
        available: usize,
    },
    /// A TGA of an image type or pixel depth this crate does not decode.
    TgaUnsupported {
        /// The image type byte at offset 2.
        image_type: u8,
        /// The bits per pixel at offset 16.
        bits: u8,
    },
    /// A run-length packet runs past the image's last pixel.
    TgaRleOverrun {
        /// The pixel the packet starts at, from zero in file order.
        pixel: usize,
        /// The packet's length in pixels.
        run: usize,
        /// The image's pixel count.
        pixels: usize,
    },
    /// A TPC the writer cannot write as given.
    TpcUnwritable {
        /// Why.
        reason: String,
    },
    /// A TXI line that would not read back as written.
    TxiUnwritable {
        /// The keyword, value or row at fault.
        line: String,
        /// What is wrong with it.
        reason: String,
    },
    /// A TGA the writer cannot write as given.
    TgaUnwritable {
        /// Why.
        reason: String,
    },
    /// A mip level the texture does not have.
    LevelMissing {
        /// The level asked for, from 0.
        level: usize,
        /// The levels each image has.
        levels: usize,
    },
    /// An image the texture does not have.
    ImageMissing {
        /// The image asked for, from 0.
        image: usize,
        /// The images the texture has.
        images: usize,
    },
    /// Pixels the PNG encoder cannot write as given.
    PngUnwritable {
        /// Why.
        reason: String,
    },
}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The registered diagnostic code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Error::TpcTruncated { .. } => codes::TPC_TRUNCATED,
            Error::TpcUnknownFormat { .. } => codes::TPC_UNKNOWN_FORMAT,
            Error::TpcBadLayout { .. } => codes::TPC_BAD_LAYOUT,
            Error::TgaTruncated { .. } => codes::TGA_TRUNCATED,
            Error::TgaUnsupported { .. } => codes::TGA_UNSUPPORTED,
            Error::TgaRleOverrun { .. } => codes::TGA_RLE_OVERRUN,
            Error::TpcUnwritable { .. } => codes::TPC_UNWRITABLE,
            Error::TxiUnwritable { .. } => codes::TXI_UNWRITABLE,
            Error::TgaUnwritable { .. } => codes::TGA_UNWRITABLE,
            Error::LevelMissing { .. } => codes::LEVEL_MISSING,
            Error::ImageMissing { .. } => codes::IMAGE_MISSING,
            Error::PngUnwritable { .. } => codes::PNG_UNWRITABLE,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Error::TpcTruncated {
                what,
                needed,
                available,
            } => write!(
                f,
                "the TPC's {what} needs {needed} bytes and the file has {available}"
            ),
            Error::TpcUnknownFormat {
                encoding,
                compressed,
            } => write!(
                f,
                "TPC encoding {encoding} with a {} data-size field names no pixel format",
                if *compressed { "non-zero" } else { "zero" }
            ),
            Error::TpcBadLayout { reason } => write!(f, "TPC layout: {reason}"),
            Error::TgaTruncated {
                what,
                needed,
                available,
            } => write!(
                f,
                "the TGA's {what} needs {needed} bytes and the file has {available}"
            ),
            Error::TgaUnsupported { image_type, bits } => write!(
                f,
                "TGA image type {image_type} at {bits} bits per pixel is not decoded; types 2, 3, 10 and 11 are"
            ),
            Error::TgaRleOverrun { pixel, run, pixels } => write!(
                f,
                "a TGA run of {run} pixels at pixel {pixel} passes the image's {pixels}"
            ),
            Error::TpcUnwritable { reason } => write!(f, "cannot write the TPC: {reason}"),
            Error::TxiUnwritable { line, reason } => {
                write!(f, "cannot write the TXI line `{line}`: {reason}")
            }
            Error::TgaUnwritable { reason } => write!(f, "cannot write the TGA: {reason}"),
            Error::LevelMissing { level, levels } => write!(
                f,
                "mip level {level} asked for; the texture has {levels}, 0 to {}",
                levels.saturating_sub(1)
            ),
            Error::ImageMissing { image, images } => write!(
                f,
                "image {image} asked for; the texture has {images}, 0 to {}",
                images.saturating_sub(1)
            ),
            Error::PngUnwritable { reason } => write!(f, "cannot write the PNG: {reason}"),
        }
    }
}

impl std::error::Error for Error {}
