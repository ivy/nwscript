//! TGA: Truevision's published format, the true-colour and greyscale
//! kinds, raw or run-length encoded, decoded to RGBA8.

use crate::{Error, Result};

/// The header's length.
pub const HEADER_LEN: usize = 18;

/// The image types this crate decodes, by the byte at offset 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageType {
    /// 2: true colour, raw.
    TrueColor,
    /// 3: greyscale, raw.
    Grey,
    /// 10: true colour, run-length encoded.
    TrueColorRle,
    /// 11: greyscale, run-length encoded.
    GreyRle,
}

impl ImageType {
    /// The byte the header stores.
    pub fn byte(self) -> u8 {
        match self {
            ImageType::TrueColor => 2,
            ImageType::Grey => 3,
            ImageType::TrueColorRle => 10,
            ImageType::GreyRle => 11,
        }
    }

    fn is_rle(self) -> bool {
        matches!(self, ImageType::TrueColorRle | ImageType::GreyRle)
    }

    fn is_grey(self) -> bool {
        matches!(self, ImageType::Grey | ImageType::GreyRle)
    }
}

/// The corner the file's first pixel belongs to, from bits 4 and 5 of the
/// descriptor byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// Neither bit: rows run bottom to top, pixels left to right.
    BottomLeft,
    /// Bit 4.
    BottomRight,
    /// Bit 5.
    TopLeft,
    /// Bits 4 and 5.
    TopRight,
}

/// A TGA decoded to RGBA8, rows top first and pixels left to right
/// whatever the file's origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tga {
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// The image type.
    pub image_type: ImageType,
    /// Bits per pixel as stored: 8 or 16 for grey, 16, 24 or 32 for true
    /// colour.
    pub bits: u8,
    /// The alpha bits the descriptor declares (its low four bits).
    pub alpha_bits: u8,
    /// Where the file's first pixel sits.
    pub origin: Origin,
    /// `width × height × 4` bytes, R, G, B, A. A 32-bit pixel's fourth byte
    /// is its alpha whatever `alpha_bits` says; pixels without an alpha
    /// channel are opaque.
    pub rgba: Vec<u8>,
}

impl Tga {
    /// Read and decode a TGA. Bytes after the pixels (an extension area,
    /// a TGA 2.0 footer) are ignored; a colour map before them is skipped.
    pub fn parse(bytes: &[u8]) -> Result<Tga> {
        if bytes.len() < HEADER_LEN {
            return Err(Error::TgaTruncated {
                what: "header",
                needed: HEADER_LEN,
                available: bytes.len(),
            });
        }
        let id_len = usize::from(bytes[0]);
        let map_type = bytes[1];
        let ty = bytes[2];
        let map_len = usize::from(u16::from_le_bytes([bytes[5], bytes[6]]));
        let map_bits = usize::from(bytes[7]);
        let width = u16::from_le_bytes([bytes[12], bytes[13]]);
        let height = u16::from_le_bytes([bytes[14], bytes[15]]);
        let bits = bytes[16];
        let desc = bytes[17];
        let image_type = match ty {
            2 => ImageType::TrueColor,
            3 => ImageType::Grey,
            10 => ImageType::TrueColorRle,
            11 => ImageType::GreyRle,
            _ => {
                return Err(Error::TgaUnsupported {
                    image_type: ty,
                    bits,
                });
            }
        };
        let supported = if image_type.is_grey() {
            matches!(bits, 8 | 16)
        } else {
            matches!(bits, 16 | 24 | 32)
        };
        if !supported {
            return Err(Error::TgaUnsupported {
                image_type: ty,
                bits,
            });
        }
        let alpha_bits = desc & 0x0f;
        let origin = match (desc >> 4) & 3 {
            0 => Origin::BottomLeft,
            1 => Origin::BottomRight,
            2 => Origin::TopLeft,
            _ => Origin::TopRight,
        };
        let map_size = if map_type != 0 {
            map_len * map_bits.div_ceil(8)
        } else {
            0
        };
        let start = HEADER_LEN + id_len + map_size;
        if bytes.len() < start {
            return Err(Error::TgaTruncated {
                what: "image id and colour map",
                needed: start,
                available: bytes.len(),
            });
        }
        let bpp = usize::from(bits / 8);
        let count = usize::from(width) * usize::from(height);
        let raw = if image_type.is_rle() {
            unpack_rle(&bytes[start..], start, count, bpp)?
        } else {
            let end = start + count * bpp;
            if bytes.len() < end {
                return Err(Error::TgaTruncated {
                    what: "pixels",
                    needed: end,
                    available: bytes.len(),
                });
            }
            bytes[start..end].to_vec()
        };

        let (w, h) = (usize::from(width), usize::from(height));
        let mut rgba = vec![0u8; count * 4];
        for (i, px) in raw.chunks_exact(bpp).enumerate() {
            let (fx, fy) = (i % w, i / w);
            let x = match origin {
                Origin::BottomRight | Origin::TopRight => w - 1 - fx,
                _ => fx,
            };
            let y = match origin {
                Origin::BottomLeft | Origin::BottomRight => h - 1 - fy,
                _ => fy,
            };
            let out = &mut rgba[(y * w + x) * 4..][..4];
            out.copy_from_slice(&decode(image_type, alpha_bits, px));
        }
        Ok(Tga {
            width,
            height,
            image_type,
            bits,
            alpha_bits,
            origin,
            rgba,
        })
    }
}

/// One stored pixel as R, G, B, A.
fn decode(ty: ImageType, alpha_bits: u8, px: &[u8]) -> [u8; 4] {
    if ty.is_grey() {
        let a = px.get(1).copied().unwrap_or(255);
        return [px[0], px[0], px[0], a];
    }
    match px.len() {
        2 => {
            let v = u16::from_le_bytes([px[0], px[1]]);
            let five = |s: u16| {
                let c = ((v >> s) & 31) as u8;
                (c << 3) | (c >> 2)
            };
            let a = if alpha_bits == 0 || v & 0x8000 != 0 {
                255
            } else {
                0
            };
            [five(10), five(5), five(0), a]
        }
        3 => [px[2], px[1], px[0], 255],
        _ => [px[2], px[1], px[0], px[3]],
    }
}

/// Expand run-length packets until `count` pixels of `bpp` bytes are out.
fn unpack_rle(data: &[u8], start: usize, count: usize, bpp: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(count * bpp);
    let mut at = 0;
    let truncated = |at: usize| Error::TgaTruncated {
        what: "run-length packets",
        needed: start + at,
        available: start + data.len(),
    };
    while out.len() < count * bpp {
        let head = *data.get(at).ok_or_else(|| truncated(at + 1))?;
        at += 1;
        let run = usize::from(head & 0x7f) + 1;
        let pixel = out.len() / bpp;
        if pixel + run > count {
            return Err(Error::TgaRleOverrun {
                pixel,
                run,
                pixels: count,
            });
        }
        if head & 0x80 != 0 {
            let px = data.get(at..at + bpp).ok_or_else(|| truncated(at + bpp))?;
            at += bpp;
            for _ in 0..run {
                out.extend_from_slice(px);
            }
        } else {
            let len = run * bpp;
            let px = data.get(at..at + len).ok_or_else(|| truncated(at + len))?;
            at += len;
            out.extend_from_slice(px);
        }
    }
    Ok(out)
}
