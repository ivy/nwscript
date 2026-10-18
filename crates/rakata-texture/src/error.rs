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
}

/// An error in reading a texture.
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
        }
    }
}

impl std::error::Error for Error {}
