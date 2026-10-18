//! TPC: the header, the pixel data cut into images and mip levels, and the
//! TXI text after it. The layout is `docs/formats/tpc.md`.

use crate::txi::Txi;
use crate::{Error, Result};

/// The header's length; the pixel data begins here.
pub const HEADER_LEN: usize = 128;

/// How a level's pixels are stored. The bytes are never converted: a DXT
/// level is the 4×4 blocks as stored, an uncompressed level the pixels in
/// the channel order the file holds, rows in file order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    /// One byte per pixel (encoding 1, data-size field zero).
    Grey,
    /// Three bytes per pixel (encoding 2, data-size field zero).
    Rgb,
    /// Four bytes per pixel (encoding 4, data-size field zero).
    Rgba,
    /// DXT1 (BC1): 8 bytes per 4×4 block (encoding 2, data-size non-zero).
    Dxt1,
    /// DXT5 (BC3): 16 bytes per 4×4 block (encoding 4, data-size non-zero).
    Dxt5,
}

impl PixelFormat {
    /// The format an encoding byte names, with the data-size field zero
    /// (`compressed` false) or not.
    pub fn from_header(encoding: u8, compressed: bool) -> Option<PixelFormat> {
        Some(match (encoding, compressed) {
            (1, false) => PixelFormat::Grey,
            (2, false) => PixelFormat::Rgb,
            (4, false) => PixelFormat::Rgba,
            (2, true) => PixelFormat::Dxt1,
            (4, true) => PixelFormat::Dxt5,
            _ => return None,
        })
    }

    /// The encoding byte the header stores.
    pub fn encoding(self) -> u8 {
        match self {
            PixelFormat::Grey => 1,
            PixelFormat::Rgb | PixelFormat::Dxt1 => 2,
            PixelFormat::Rgba | PixelFormat::Dxt5 => 4,
        }
    }

    /// Whether the pixels are in 4×4 blocks.
    pub fn is_compressed(self) -> bool {
        matches!(self, PixelFormat::Dxt1 | PixelFormat::Dxt5)
    }

    /// Whether the format carries an alpha channel. DXT1 is `false`
    /// although a DXT1 block may mark texels transparent; see
    /// `docs/formats/tpc.md` for how many vanilla files do.
    pub fn has_alpha(self) -> bool {
        matches!(self, PixelFormat::Rgba | PixelFormat::Dxt5)
    }

    /// The bytes one level of `width` × `height` pixels takes: a DXT level
    /// is whole blocks, at least one.
    pub fn level_len(self, width: u32, height: u32) -> usize {
        let (w, h) = (width as usize, height as usize);
        match self {
            PixelFormat::Grey => w * h,
            PixelFormat::Rgb => w * h * 3,
            PixelFormat::Rgba => w * h * 4,
            PixelFormat::Dxt1 => w.div_ceil(4).max(1) * h.div_ceil(4).max(1) * 8,
            PixelFormat::Dxt5 => w.div_ceil(4).max(1) * h.div_ceil(4).max(1) * 16,
        }
    }
}

/// How the pixel data divides into images.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// One image.
    Single,
    /// Six square faces, stacked so the header's height is six times its
    /// width; each face has its own mip chain. The TXI says `cube 1`.
    Cube,
    /// An animation sheet: the header's width × height divided into
    /// `numx` × `numy` frames, each stored whole with a mip chain to 1×1,
    /// frame after frame. The TXI says `proceduretype cycle` and gives
    /// `numx` and `numy`.
    Flipbook {
        /// Frames across, from the TXI.
        numx: u32,
        /// Frames down, from the TXI.
        numy: u32,
    },
}

/// One mip level: its size in pixels and its bytes, borrowed from the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level<'a> {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Where the level begins in the bytes parsed.
    pub offset: usize,
    /// The level's bytes as stored.
    pub data: &'a [u8],
}

/// One image (the texture, a cube face, or a flipbook frame) and its mip
/// levels, largest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image<'a> {
    /// The levels, level 0 first.
    pub levels: Vec<Level<'a>>,
}

/// A TPC texture, borrowing its pixel data from the bytes it was read from.
#[derive(Debug, Clone, PartialEq)]
pub struct Tpc<'a> {
    /// The header's width (offset 8).
    pub width: u16,
    /// The header's height (offset 10); for a cube, six faces' worth.
    pub height: u16,
    /// The pixel format.
    pub format: PixelFormat,
    /// The header's mip count (offset 13). A flipbook says 1 though each
    /// frame has a full chain; [`Image::levels`] has the real count.
    pub mip_count: u8,
    /// The header's data-size field (offset 0): zero for an uncompressed
    /// format; otherwise level 0's length, at least 16, or for a flipbook
    /// the length of the whole pixel data.
    pub data_size: u32,
    /// The float at offset 4: 1.0 for formats without alpha, otherwise
    /// in nearly every vanilla file the mean alpha of level 0 over 255.
    pub alpha_mean: f32,
    /// How the pixel data divides into images.
    pub layout: Layout,
    /// One image, six faces, or `numx` × `numy` frames in row order.
    pub images: Vec<Image<'a>>,
    /// The pixel data whole: every image's levels, back to back.
    pub pixels: &'a [u8],
    /// The bytes after the pixel data: the TXI text, often empty.
    pub txi_bytes: &'a [u8],
    /// The TXI parsed, or `None` when there are no bytes after the pixels.
    pub txi: Option<Txi>,
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("four bytes"))
}

fn bad(reason: impl Into<String>) -> Error {
    Error::TpcBadLayout {
        reason: reason.into(),
    }
}

/// The sizes of `count` levels from `width` × `height`, halving to 1.
fn chain(width: u32, height: u32, count: usize) -> Vec<(u32, u32)> {
    (0..count)
        .map(|i| {
            let s = u32::try_from(i).unwrap_or(u32::MAX).min(31);
            ((width >> s).max(1), (height >> s).max(1))
        })
        .collect()
}

/// The number of levels from `width` × `height` down to 1×1.
fn full_chain(width: u32, height: u32) -> usize {
    (32 - width.max(height).leading_zeros()) as usize
}

impl<'a> Tpc<'a> {
    /// Read a TPC. The levels and the TXI bytes borrow from `bytes`; only
    /// the parsed TXI is owned.
    pub fn parse(bytes: &'a [u8]) -> Result<Tpc<'a>> {
        if bytes.len() < HEADER_LEN {
            return Err(Error::TpcTruncated {
                what: "header",
                needed: HEADER_LEN,
                available: bytes.len(),
            });
        }
        let data_size = u32_at(bytes, 0);
        let alpha_mean = f32::from_le_bytes(bytes[4..8].try_into().expect("four bytes"));
        let width = u16_at(bytes, 8);
        let height = u16_at(bytes, 10);
        let encoding = bytes[12];
        let mip_count = bytes[13];
        let format =
            PixelFormat::from_header(encoding, data_size != 0).ok_or(Error::TpcUnknownFormat {
                encoding,
                compressed: data_size != 0,
            })?;
        if width == 0 || height == 0 {
            return Err(bad(format!("the size is {width}×{height}")));
        }
        if mip_count == 0 {
            return Err(bad("the mip count is zero"));
        }
        let (w, h) = (u32::from(width), u32::from(height));
        let cube = h == 6 * w;
        let face_h = if cube { w } else { h };
        let level0 = format.level_len(w, face_h);
        let flipbook = format.is_compressed()
            && mip_count == 1
            && !cube
            && data_size as usize > level0.max(16);

        // The images' sizes and level counts, and the pixel data's length.
        let (layout, frame, count, levels) = if flipbook {
            (None, (w, h), 0, 0)
        } else if cube {
            (Some(Layout::Cube), (w, w), 6, usize::from(mip_count))
        } else {
            (Some(Layout::Single), (w, h), 1, usize::from(mip_count))
        };
        let (layout, (fw, fh), count, levels, pixel_len, txi_bytes, txi) = match layout {
            Some(layout) => {
                let per = chain(frame.0, frame.1, levels)
                    .iter()
                    .map(|&(a, b)| format.level_len(a, b))
                    .sum::<usize>();
                let pixel_len = per * count;
                let end = HEADER_LEN + pixel_len;
                if bytes.len() < end {
                    return Err(Error::TpcTruncated {
                        what: "pixel data",
                        needed: end,
                        available: bytes.len(),
                    });
                }
                let rest = &bytes[end..];
                let txi = (!rest.is_empty()).then(|| Txi::parse(rest));
                (layout, frame, count, levels, pixel_len, rest, txi)
            }
            None => {
                let pixel_len = data_size as usize;
                let end = HEADER_LEN + pixel_len;
                if bytes.len() < end {
                    return Err(Error::TpcTruncated {
                        what: "pixel data",
                        needed: end,
                        available: bytes.len(),
                    });
                }
                let rest = &bytes[end..];
                let txi = Txi::parse(rest);
                let numx = txi.int("numx").unwrap_or(0);
                let numy = txi.int("numy").unwrap_or(0);
                if numx < 1 || numy < 1 {
                    return Err(bad(format!(
                        "the data-size field ({data_size}) passes level 0 ({level0}) with one mip level, \
                         which is an animation sheet, but the TXI gives no numx and numy"
                    )));
                }
                let (nx, ny) = (numx as u64, numy as u64);
                if !u64::from(w).is_multiple_of(nx) || !u64::from(h).is_multiple_of(ny) {
                    return Err(bad(format!(
                        "{w}×{h} does not divide into {numx}×{numy} frames"
                    )));
                }
                let (nx, ny) = (nx as u32, ny as u32);
                let (fw, fh) = (w / nx, h / ny);
                let levels = full_chain(fw, fh);
                let per = chain(fw, fh, levels)
                    .iter()
                    .map(|&(a, b)| format.level_len(a, b))
                    .sum::<usize>();
                let count = (nx * ny) as usize;
                if per * count != pixel_len {
                    return Err(bad(format!(
                        "{count} frames of {fw}×{fh} with {levels} levels take {} bytes, the data-size field says {pixel_len}",
                        per * count
                    )));
                }
                (
                    Layout::Flipbook { numx: nx, numy: ny },
                    (fw, fh),
                    count,
                    levels,
                    pixel_len,
                    rest,
                    Some(txi),
                )
            }
        };

        let sizes = chain(fw, fh, levels);
        let mut at = HEADER_LEN;
        let images = (0..count)
            .map(|_| Image {
                levels: sizes
                    .iter()
                    .map(|&(lw, lh)| {
                        let len = format.level_len(lw, lh);
                        let level = Level {
                            width: lw,
                            height: lh,
                            offset: at,
                            data: &bytes[at..at + len],
                        };
                        at += len;
                        level
                    })
                    .collect(),
            })
            .collect();
        Ok(Tpc {
            width,
            height,
            format,
            mip_count,
            data_size,
            alpha_mean,
            layout,
            images,
            pixels: &bytes[HEADER_LEN..HEADER_LEN + pixel_len],
            txi_bytes,
            txi,
        })
    }

    /// Whether the pixel format has an alpha channel.
    pub fn has_alpha(&self) -> bool {
        self.format.has_alpha()
    }

    /// The size of one image (a face or a frame), as level 0 has it.
    pub fn image_size(&self) -> (u32, u32) {
        self.images[0]
            .levels
            .first()
            .map(|l| (l.width, l.height))
            .unwrap_or((u32::from(self.width), u32::from(self.height)))
    }

    /// The number of levels each image has.
    pub fn level_count(&self) -> usize {
        self.images[0].levels.len()
    }
}
