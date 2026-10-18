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
///
/// Equality compares the decoded image and the header fields above;
/// [`Tga::layout`], how the file stored it, is not compared.
#[derive(Debug, Clone, Eq)]
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
    /// What the file holds beyond the decoded image, so that a TGA read
    /// and written again is the bytes it came from. [`TgaLayout::default`]
    /// for an image made from nothing.
    pub layout: TgaLayout,
}

impl PartialEq for Tga {
    fn eq(&self, other: &Tga) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.image_type == other.image_type
            && self.bits == other.bits
            && self.alpha_bits == other.alpha_bits
            && self.origin == other.origin
            && self.rgba == other.rgba
    }
}

/// The parts of a TGA file the decoded image does not carry: header fields
/// it does not use, the image ID, the colour map, how run-length packets
/// fell, and the bytes after the pixels. Every field is as read, and is
/// written back as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TgaLayout {
    /// The image ID, the bytes after the header (at most 255).
    pub image_id: Vec<u8>,
    /// The colour map type (offset 1): 0 for none.
    pub color_map_type: u8,
    /// The colour map's first entry index (offset 3).
    pub color_map_first: u16,
    /// The colour map's entry count (offset 5).
    pub color_map_len: u16,
    /// The colour map's bits per entry (offset 7).
    pub color_map_bits: u8,
    /// The colour map's bytes, after the image ID; empty when the type is
    /// 0.
    pub color_map: Vec<u8>,
    /// The image's x origin (offset 8).
    pub x_origin: u16,
    /// The image's y origin (offset 10).
    pub y_origin: u16,
    /// Bits 6 and 7 of the descriptor (offset 17), in place; the others
    /// are [`Tga::alpha_bits`] and [`Tga::origin`].
    pub descriptor_high: u8,
    /// The head byte of each run-length packet in file order, for a
    /// run-length encoded image: the writer packs the pixels the same way
    /// while they still allow it. Empty for a raw image.
    pub packets: Vec<u8>,
    /// For a 16-bit true-colour image that declares no alpha bits, whether
    /// each pixel's top bit was set, in file order; the decoded image
    /// cannot carry that bit. Empty otherwise.
    pub attribute_bits: Vec<bool>,
    /// The bytes after the pixel data: an extension area and a TGA 2.0
    /// footer, or anything else.
    pub tail: Vec<u8>,
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
        let (raw, packets, end) = if image_type.is_rle() {
            let (raw, packets, used) = unpack_rle(&bytes[start..], start, count, bpp)?;
            (raw, packets, start + used)
        } else {
            let end = start + count * bpp;
            if bytes.len() < end {
                return Err(Error::TgaTruncated {
                    what: "pixels",
                    needed: end,
                    available: bytes.len(),
                });
            }
            (bytes[start..end].to_vec(), Vec::new(), end)
        };
        let attribute_bits = if !image_type.is_grey() && bpp == 2 && alpha_bits == 0 {
            raw.as_chunks::<2>()
                .0
                .iter()
                .map(|px| px[1] & 0x80 != 0)
                .collect()
        } else {
            Vec::new()
        };
        let layout = TgaLayout {
            image_id: bytes[HEADER_LEN..HEADER_LEN + id_len].to_vec(),
            color_map_type: map_type,
            color_map_first: u16::from_le_bytes([bytes[3], bytes[4]]),
            color_map_len: u16::from_le_bytes([bytes[5], bytes[6]]),
            color_map_bits: bytes[7],
            color_map: bytes[HEADER_LEN + id_len..start].to_vec(),
            x_origin: u16::from_le_bytes([bytes[8], bytes[9]]),
            y_origin: u16::from_le_bytes([bytes[10], bytes[11]]),
            descriptor_high: desc & 0xc0,
            packets,
            attribute_bits,
            tail: bytes[end..].to_vec(),
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
            layout,
        })
    }

    /// Write the TGA: the header from the fields and [`Tga::layout`], the
    /// image ID and colour map, the pixels encoded back to the stored
    /// depth in the stored order, and the tail. A run-length encoded image
    /// is packed as [`TgaLayout::packets`] says while those packets still
    /// fit the pixels (each run's pixels the same), and otherwise anew: a
    /// run for two or more equal pixels, raw packets between, no packet
    /// across a row.
    ///
    /// Refuses (`RE2209`) pixels of a length other than `width × height ×
    /// 4`, an image ID longer than 255 bytes, alpha bits above 15 or bits
    /// other than 6 and 7 in `descriptor_high`, and (`RE2205`) a type and
    /// depth the reader does not decode.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let unwritable = |reason: String| Err(Error::TgaUnwritable { reason });
        let supported = if self.image_type.is_grey() {
            matches!(self.bits, 8 | 16)
        } else {
            matches!(self.bits, 16 | 24 | 32)
        };
        if !supported {
            return Err(Error::TgaUnsupported {
                image_type: self.image_type.byte(),
                bits: self.bits,
            });
        }
        let (w, h) = (usize::from(self.width), usize::from(self.height));
        let count = w * h;
        if self.rgba.len() != count * 4 {
            return unwritable(format!(
                "{} bytes of pixels for {w}×{h}, which takes {}",
                self.rgba.len(),
                count * 4
            ));
        }
        let l = &self.layout;
        let Ok(id_len) = u8::try_from(l.image_id.len()) else {
            return unwritable(format!("an image ID of {} bytes", l.image_id.len()));
        };
        if self.alpha_bits > 15 {
            return unwritable(format!("{} alpha bits", self.alpha_bits));
        }
        if l.descriptor_high & !0xc0 != 0 {
            return unwritable(format!(
                "descriptor bits {:#04x} beyond bits 6 and 7",
                l.descriptor_high
            ));
        }
        let origin = match self.origin {
            Origin::BottomLeft => 0,
            Origin::BottomRight => 1,
            Origin::TopLeft => 2,
            Origin::TopRight => 3,
        };
        let mut out = Vec::with_capacity(HEADER_LEN + count * 4 + l.tail.len());
        out.push(id_len);
        out.push(l.color_map_type);
        out.push(self.image_type.byte());
        out.extend_from_slice(&l.color_map_first.to_le_bytes());
        out.extend_from_slice(&l.color_map_len.to_le_bytes());
        out.push(l.color_map_bits);
        out.extend_from_slice(&l.x_origin.to_le_bytes());
        out.extend_from_slice(&l.y_origin.to_le_bytes());
        out.extend_from_slice(&self.width.to_le_bytes());
        out.extend_from_slice(&self.height.to_le_bytes());
        out.push(self.bits);
        out.push(self.alpha_bits | (origin << 4) | l.descriptor_high);
        out.extend_from_slice(&l.image_id);
        out.extend_from_slice(&l.color_map);

        // The pixels as stored, in file order.
        let bpp = usize::from(self.bits / 8);
        let mut stored = Vec::with_capacity(count * bpp);
        for i in 0..count {
            let (fx, fy) = (i % w, i / w);
            let x = match self.origin {
                Origin::BottomRight | Origin::TopRight => w - 1 - fx,
                _ => fx,
            };
            let y = match self.origin {
                Origin::BottomLeft | Origin::BottomRight => h - 1 - fy,
                _ => fy,
            };
            let px = &self.rgba[(y * w + x) * 4..][..4];
            let attribute = l.attribute_bits.get(i).copied().unwrap_or(false);
            self.encode(px, attribute, &mut stored);
        }
        if self.image_type.is_rle() {
            if !pack_as(&stored, bpp, &l.packets, &mut out) {
                pack_anew(&stored, bpp, w, &mut out);
            }
        } else {
            out.extend_from_slice(&stored);
        }
        out.extend_from_slice(&l.tail);
        Ok(out)
    }

    /// One R, G, B, A pixel at the stored depth.
    fn encode(&self, px: &[u8], attribute: bool, out: &mut Vec<u8>) {
        let [r, g, b, a] = [px[0], px[1], px[2], px[3]];
        if self.image_type.is_grey() {
            out.push(r);
            if self.bits == 16 {
                out.push(a);
            }
            return;
        }
        match self.bits {
            16 => {
                let five = |c: u8, s: u16| u16::from(c >> 3) << s;
                let top = if self.alpha_bits == 0 {
                    attribute
                } else {
                    a >= 128
                };
                let v = five(r, 10) | five(g, 5) | five(b, 0) | if top { 0x8000 } else { 0 };
                out.extend_from_slice(&v.to_le_bytes());
            }
            24 => out.extend_from_slice(&[b, g, r]),
            _ => out.extend_from_slice(&[b, g, r, a]),
        }
    }
}

/// Pack `stored` with the recorded packet heads, if they cover every pixel
/// and each run's pixels are the same; `false`, writing nothing, if not.
fn pack_as(stored: &[u8], bpp: usize, packets: &[u8], out: &mut Vec<u8>) -> bool {
    let total: usize = packets.iter().map(|&h| usize::from(h & 0x7f) + 1).sum();
    if packets.is_empty() || total * bpp != stored.len() {
        return false;
    }
    let mut at = 0;
    for &head in packets {
        let len = (usize::from(head & 0x7f) + 1) * bpp;
        let span = &stored[at..at + len];
        if head & 0x80 != 0 && span.chunks_exact(bpp).any(|p| p != &span[..bpp]) {
            return false;
        }
        at += len;
    }
    let mut at = 0;
    for &head in packets {
        let len = (usize::from(head & 0x7f) + 1) * bpp;
        out.push(head);
        if head & 0x80 != 0 {
            out.extend_from_slice(&stored[at..at + bpp]);
        } else {
            out.extend_from_slice(&stored[at..at + len]);
        }
        at += len;
    }
    true
}

/// Pack `stored` row by row: a run packet for two or more equal pixels, a
/// raw packet for the pixels between, at most 128 pixels a packet.
fn pack_anew(stored: &[u8], bpp: usize, width: usize, out: &mut Vec<u8>) {
    for row in stored.chunks(width.max(1) * bpp) {
        let px: Vec<&[u8]> = row.chunks_exact(bpp).collect();
        let mut i = 0;
        while i < px.len() {
            let mut run = 1;
            while i + run < px.len() && run < 128 && px[i + run] == px[i] {
                run += 1;
            }
            if run >= 2 {
                out.push(0x80 | (run - 1) as u8);
                out.extend_from_slice(px[i]);
                i += run;
                continue;
            }
            let mut raw = 1;
            while i + raw < px.len()
                && raw < 128
                && !(i + raw + 1 < px.len() && px[i + raw] == px[i + raw + 1])
            {
                raw += 1;
            }
            out.push((raw - 1) as u8);
            for p in &px[i..i + raw] {
                out.extend_from_slice(p);
            }
            i += raw;
        }
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

/// Expand run-length packets until `count` pixels of `bpp` bytes are out:
/// the pixels, the packets' head bytes, and the bytes of `data` they took.
fn unpack_rle(
    data: &[u8],
    start: usize,
    count: usize,
    bpp: usize,
) -> Result<(Vec<u8>, Vec<u8>, usize)> {
    let mut out = Vec::with_capacity(count * bpp);
    let mut heads = Vec::new();
    let mut at = 0;
    let truncated = |at: usize| Error::TgaTruncated {
        what: "run-length packets",
        needed: start + at,
        available: start + data.len(),
    };
    while out.len() < count * bpp {
        let head = *data.get(at).ok_or_else(|| truncated(at + 1))?;
        at += 1;
        heads.push(head);
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
    Ok((out, heads, at))
}
