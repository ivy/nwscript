//! Pixels decoded to RGBA8, and RGBA8 encoded as PNG.
//!
//! DXT1 and DXT5 are decoded as the S3TC specification defines them
//! (Khronos `EXT_texture_compression_s3tc`, the DXT1 and DXT5 sections;
//! Microsoft's BC1 and BC3 pages describe the same blocks): a block is
//! 4×4 texels, its texels in row order, the first row the top. Each 5- or
//! 6-bit endpoint channel stands for the fraction of its maximum; a
//! palette entry is blended from those fractions and rounded once to the
//! nearest 8-bit value, and so is each DXT5 alpha. The
//! uncompressed formats are widened as stored: a grey byte to three equal
//! channels, RGB and RGBA bytes in the order the file holds them, since
//! `docs/formats/tpc.md` leaves their channel order unestablished.

use crate::tga::Tga;
use crate::tpc::{Layout, Level, PixelFormat, Tpc};
use crate::{Error, Result};

/// An image as RGBA8: `width × height × 4` bytes, R, G, B, A, rows top
/// first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The pixels, four bytes each.
    pub pixels: Vec<u8>,
}

/// Which mip levels [`Tpc::to_pngs`] encodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Levels {
    /// Level 0 alone, the largest.
    #[default]
    Largest,
    /// The one level given, from 0.
    One(usize),
    /// Every level, largest first.
    All,
}

/// One level encoded as PNG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Png {
    /// The mip level, from 0.
    pub level: usize,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The PNG file.
    pub bytes: Vec<u8>,
}

/// `(a × wa + b × wb) / (wa + wb)`, rounded to nearest.
fn mix(a: u8, b: u8, wa: u32, wb: u32) -> u8 {
    let n = wa + wb;
    ((u32::from(a) * wa + u32::from(b) * wb + n / 2) / n) as u8
}

/// The channels of an RGB565 colour, red in bits 15..11, green 10..5 and
/// blue 4..0, each with its maximum.
fn rgb565(c: u16) -> [(u32, u32); 3] {
    let c = u32::from(c);
    [(c >> 11, 31), ((c >> 5) & 0x3f, 63), (c & 0x1f, 31)]
}

/// The opaque 8-bit colour `(a × wa + b × wb) / (wa + wb)` of two RGB565
/// colours: each channel read as the fraction `v / max` it stands for,
/// blended, and rounded once to the nearest of 0 to 255.
fn blend(a: u16, b: u16, wa: u32, wb: u32) -> [u8; 4] {
    let (a, b) = (rgb565(a), rgb565(b));
    let ch = |i: usize| {
        let ((va, max), (vb, _)) = (a[i], b[i]);
        let den = max * (wa + wb);
        ((255 * (va * wa + vb * wb) + den / 2) / den) as u8
    };
    [ch(0), ch(1), ch(2), 255]
}

/// The sixteen texels of a DXT1 colour block, in row order. `dxt5` treats
/// the block as though color0 > color1, as DXT5's colour block always is.
fn color_block(b: &[u8], dxt5: bool) -> [[u8; 4]; 16] {
    let c0 = u16::from_le_bytes([b[0], b[1]]);
    let c1 = u16::from_le_bytes([b[2], b[3]]);
    let bits = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    let palette = if dxt5 || c0 > c1 {
        [
            blend(c0, c1, 1, 0),
            blend(c0, c1, 0, 1),
            blend(c0, c1, 2, 1),
            blend(c0, c1, 1, 2),
        ]
    } else {
        // Code 3 is black; in RGBA DXT1 its alpha is 0, the transparent
        // texel of the three-colour mode.
        [
            blend(c0, c1, 1, 0),
            blend(c0, c1, 0, 1),
            blend(c0, c1, 1, 1),
            [0, 0, 0, 0],
        ]
    };
    std::array::from_fn(|i| palette[((bits >> (2 * i)) & 3) as usize])
}

/// The sixteen alpha values of a DXT5 alpha block, in row order.
fn alpha_block(b: &[u8]) -> [u8; 16] {
    let (a0, a1) = (b[0], b[1]);
    let mut bits = 0u64;
    for (i, &byte) in b[2..8].iter().enumerate() {
        bits |= u64::from(byte) << (8 * i);
    }
    let palette: [u8; 8] = if a0 > a1 {
        std::array::from_fn(|code| match code {
            0 => a0,
            1 => a1,
            k => mix(a0, a1, 8 - k as u32, k as u32 - 1),
        })
    } else {
        std::array::from_fn(|code| match code {
            0 => a0,
            1 => a1,
            6 => 0,
            7 => 255,
            k => mix(a0, a1, 6 - k as u32, k as u32 - 1),
        })
    };
    std::array::from_fn(|i| palette[((bits >> (3 * i)) & 7) as usize])
}

/// Decode one level of `format` and `width × height` pixels from `data`,
/// its bytes as stored. Bytes past the level's length are ignored; fewer
/// are `RE2201`.
pub fn decode(format: PixelFormat, width: u32, height: u32, data: &[u8]) -> Result<Rgba> {
    let needed = format.level_len(width, height);
    if data.len() < needed {
        return Err(Error::TpcTruncated {
            what: "mip level",
            needed,
            available: data.len(),
        });
    }
    let (w, h) = (width as usize, height as usize);
    let mut pixels = vec![0u8; w * h * 4];
    match format {
        PixelFormat::Grey => {
            for (out, &v) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(data) {
                *out = [v, v, v, 255];
            }
        }
        PixelFormat::Rgb => {
            let rgb = data.as_chunks::<3>().0;
            for (out, &[r, g, b]) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(rgb) {
                *out = [r, g, b, 255];
            }
        }
        PixelFormat::Rgba => pixels.copy_from_slice(&data[..needed]),
        PixelFormat::Dxt1 | PixelFormat::Dxt5 => {
            let dxt5 = format == PixelFormat::Dxt5;
            let size = if dxt5 { 16 } else { 8 };
            let across = w.div_ceil(4).max(1);
            for (n, block) in data[..needed].chunks_exact(size).enumerate() {
                let (bx, by) = (n % across * 4, n / across * 4);
                let (colors, alpha) = if dxt5 {
                    (color_block(&block[8..], true), Some(alpha_block(block)))
                } else {
                    (color_block(block, false), None)
                };
                for (i, mut texel) in colors.into_iter().enumerate() {
                    let (x, y) = (bx + i % 4, by + i / 4);
                    if x >= w || y >= h {
                        continue;
                    }
                    if let Some(a) = alpha {
                        texel[3] = a[i];
                    }
                    let at = (y * w + x) * 4;
                    pixels[at..at + 4].copy_from_slice(&texel);
                }
            }
        }
    }
    Ok(Rgba {
        width,
        height,
        pixels,
    })
}

impl Level<'_> {
    /// This level's pixels as RGBA8, read as `format`.
    pub fn decode(&self, format: PixelFormat) -> Result<Rgba> {
        decode(format, self.width, self.height, self.data)
    }
}

impl Rgba {
    /// The image as a PNG file: 8-bit RGBA, not interlaced. Refuses
    /// (`RE2212`) pixels whose length is not `width × height × 4`, or a
    /// size the encoder does not take.
    pub fn to_png(&self) -> Result<Vec<u8>> {
        let unwritable = |reason: String| Error::PngUnwritable { reason };
        let expected = self.width as usize * self.height as usize * 4;
        if self.pixels.len() != expected {
            return Err(unwritable(format!(
                "{} bytes of pixels for {}×{} RGBA, which takes {expected}",
                self.pixels.len(),
                self.width,
                self.height
            )));
        }
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| unwritable(e.to_string()))?;
        writer
            .write_image_data(&self.pixels)
            .map_err(|e| unwritable(e.to_string()))?;
        writer.finish().map_err(|e| unwritable(e.to_string()))?;
        Ok(out)
    }

    /// Copy `tile` into this image with its top-left corner at `(x, y)`.
    fn blit(&mut self, tile: &Rgba, x: u32, y: u32) {
        let row = tile.width as usize * 4;
        for ty in 0..tile.height as usize {
            let from = ty * row;
            let to = ((y as usize + ty) * self.width as usize + x as usize) * 4;
            self.pixels[to..to + row].copy_from_slice(&tile.pixels[from..from + row]);
        }
    }
}

impl Tpc<'_> {
    /// Level `level` of image `image` (0 for a single image; a cube face
    /// or an animation frame otherwise) as RGBA8. An image the texture
    /// lacks is `RE2211`, a level it lacks `RE2210`.
    pub fn decode_image(&self, image: usize, level: usize) -> Result<Rgba> {
        let img = self.images.get(image).ok_or(Error::ImageMissing {
            image,
            images: self.images.len(),
        })?;
        let l = img.levels.get(level).ok_or(Error::LevelMissing {
            level,
            levels: img.levels.len(),
        })?;
        l.decode(self.format)
    }

    /// Level `level` of the whole texture as RGBA8, its images laid out as
    /// the header's width × height lays them at level 0: a single image as
    /// it is; a cube's six faces stacked top to bottom in the order stored;
    /// an animation sheet's frames `numx` across and `numy` down, filled
    /// row by row in the order stored. A level the texture lacks is
    /// `RE2210`.
    pub fn decode_level(&self, level: usize) -> Result<Rgba> {
        let (across, down) = match self.layout {
            Layout::Single => (1, 1),
            Layout::Cube => (1, 6),
            Layout::Flipbook { numx, numy } => (numx, numy),
        };
        let levels = self.level_count();
        let Some(first) = self.images[0].levels.get(level) else {
            return Err(Error::LevelMissing { level, levels });
        };
        if self.images.len() == 1 {
            return first.decode(self.format);
        }
        let (tw, th) = (first.width, first.height);
        let mut sheet = Rgba {
            width: tw * across,
            height: th * down,
            pixels: vec![0; (tw * across) as usize * (th * down) as usize * 4],
        };
        for (n, img) in self.images.iter().enumerate() {
            let tile = img.levels[level].decode(self.format)?;
            let n = n as u32;
            sheet.blit(&tile, n % across * tw, n / across * th);
        }
        Ok(sheet)
    }

    /// The levels `levels` names of the whole texture (see
    /// [`Tpc::decode_level`]), or of one image when `image` is given, each
    /// encoded as PNG. A level the texture lacks is `RE2210`, an image
    /// `RE2211`.
    pub fn to_pngs(&self, levels: Levels, image: Option<usize>) -> Result<Vec<Png>> {
        if let Some(i) = image
            && i >= self.images.len()
        {
            return Err(Error::ImageMissing {
                image: i,
                images: self.images.len(),
            });
        }
        let wanted: Vec<usize> = match levels {
            Levels::Largest => vec![0],
            Levels::One(l) => vec![l],
            Levels::All => (0..self.level_count()).collect(),
        };
        wanted
            .into_iter()
            .map(|level| {
                let rgba = match image {
                    Some(i) => self.decode_image(i, level)?,
                    None => self.decode_level(level)?,
                };
                Ok(Png {
                    level,
                    width: rgba.width,
                    height: rgba.height,
                    bytes: rgba.to_png()?,
                })
            })
            .collect()
    }
}

impl Tga {
    /// The image as RGBA8, rows top first.
    pub fn to_rgba(&self) -> Rgba {
        Rgba {
            width: u32::from(self.width),
            height: u32::from(self.height),
            pixels: self.rgba.clone(),
        }
    }

    /// The image as a PNG file; see [`Rgba::to_png`].
    pub fn to_png(&self) -> Result<Vec<u8>> {
        self.to_rgba().to_png()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_endpoint_is_its_fraction_rounded_to_eight_bits() {
        for v in 0..32u16 {
            let want = (f64::from(v) / 31.0 * 255.0).round() as u8;
            assert_eq!(blend(v << 11, 0, 1, 0)[0], want, "red {v}");
            assert_eq!(blend(v, 0, 1, 0)[2], want, "blue {v}");
        }
        for v in 0..64u16 {
            let want = (f64::from(v) / 63.0 * 255.0).round() as u8;
            assert_eq!(blend(v << 5, 0, 1, 0)[1], want, "green {v}");
        }
    }

    #[test]
    fn a_blend_is_rounded_once_from_the_fractions() {
        // Red 3/31 and 4/31 at two thirds and one third: 255 × 10 / 93 is
        // 27.42, where blending the endpoints already rounded to 8 bits
        // (25 and 33) would give 27.67, so 28.
        assert_eq!(blend(3 << 11, 4 << 11, 2, 1)[0], 27);
    }

    #[test]
    fn dxt1_four_colour_mode_interpolates_thirds() {
        // color0 pure red (0xf800) > color1 pure blue (0x001f).
        let mut b = vec![0x00, 0xf8, 0x1f, 0x00];
        // Texels 0..3 take codes 0, 1, 2, 3; the rest code 0.
        b.extend(0b11_10_01_00u32.to_le_bytes());
        let px = decode(PixelFormat::Dxt1, 4, 4, &b).unwrap().pixels;
        assert_eq!(&px[0..4], &[255, 0, 0, 255]);
        assert_eq!(&px[4..8], &[0, 0, 255, 255]);
        assert_eq!(&px[8..12], &[170, 0, 85, 255]);
        assert_eq!(&px[12..16], &[85, 0, 170, 255]);
        assert_eq!(&px[16..20], &[255, 0, 0, 255]);
    }

    #[test]
    fn dxt1_three_colour_mode_has_a_midpoint_and_transparent_black() {
        // color0 blue (0x001f) <= color1 red (0xf800).
        let mut b = vec![0x1f, 0x00, 0x00, 0xf8];
        b.extend(0b11_10_01_00u32.to_le_bytes());
        let px = decode(PixelFormat::Dxt1, 4, 4, &b).unwrap().pixels;
        assert_eq!(&px[8..12], &[128, 0, 128, 255]);
        assert_eq!(&px[12..16], &[0, 0, 0, 0]);
    }

    #[test]
    fn dxt5_colour_is_always_four_colour_and_alpha_interpolates() {
        // Alpha block: a0 = 255 > a1 = 0, texel i takes code i % 8.
        let mut bits = 0u64;
        for i in 0..16u64 {
            bits |= (i % 8) << (3 * i);
        }
        let mut b = vec![255, 0];
        b.extend(&bits.to_le_bytes()[..6]);
        // Colour block with color0 <= color1, codes 3 on texel 0.
        b.extend([0x1f, 0x00, 0x00, 0xf8]);
        b.extend(3u32.to_le_bytes());
        let px = decode(PixelFormat::Dxt5, 4, 4, &b).unwrap().pixels;
        // Code 3 is the 1/3, 2/3 blend, opaque colour, never black.
        assert_eq!(&px[0..3], &[170, 0, 85]);
        let alphas: Vec<u8> = px.chunks(4).map(|p| p[3]).take(8).collect();
        assert_eq!(alphas, [255, 0, 219, 182, 146, 109, 73, 36]);

        // a0 <= a1: codes 6 and 7 are 0 and 255, 2..5 fifths.
        let mut b = vec![0, 255];
        b.extend(&bits.to_le_bytes()[..6]);
        b.extend([0u8; 8]);
        let px = decode(PixelFormat::Dxt5, 4, 4, &b).unwrap().pixels;
        let alphas: Vec<u8> = px.chunks(4).map(|p| p[3]).take(8).collect();
        assert_eq!(alphas, [0, 255, 51, 102, 153, 204, 0, 255]);
    }

    #[test]
    fn a_level_smaller_than_a_block_is_cropped() {
        let mut b = vec![0x00, 0xf8, 0x1f, 0x00];
        // Texel (1, 1) is index 5: code 1.
        b.extend((1u32 << 10).to_le_bytes());
        let img = decode(PixelFormat::Dxt1, 2, 2, &b).unwrap();
        assert_eq!(img.pixels.len(), 16);
        assert_eq!(&img.pixels[12..16], &[0, 0, 255, 255]);
    }

    #[test]
    fn blocks_tile_left_to_right_then_down() {
        // 8×4: two blocks, the first all color0 red, the second all blue.
        let mut b = vec![0x00, 0xf8, 0x1f, 0x00, 0, 0, 0, 0];
        b.extend([0x00, 0xf8, 0x1f, 0x00, 0x55, 0x55, 0x55, 0x55]);
        let img = decode(PixelFormat::Dxt1, 8, 4, &b).unwrap();
        assert_eq!(&img.pixels[0..4], &[255, 0, 0, 255]);
        assert_eq!(&img.pixels[16..20], &[0, 0, 255, 255]);
        assert_eq!(&img.pixels[(8 * 3 + 7) * 4..][..4], &[0, 0, 255, 255]);
    }

    #[test]
    fn uncompressed_formats_widen_as_stored() {
        let g = decode(PixelFormat::Grey, 2, 1, &[7, 9]).unwrap();
        assert_eq!(g.pixels, [7, 7, 7, 255, 9, 9, 9, 255]);
        let rgb = decode(PixelFormat::Rgb, 1, 1, &[1, 2, 3]).unwrap();
        assert_eq!(rgb.pixels, [1, 2, 3, 255]);
        let rgba = decode(PixelFormat::Rgba, 1, 1, &[1, 2, 3, 4]).unwrap();
        assert_eq!(rgba.pixels, [1, 2, 3, 4]);
        assert_eq!(
            decode(PixelFormat::Rgb, 2, 2, &[0; 11]).unwrap_err().code(),
            "RE2201"
        );
    }

    #[test]
    fn a_png_reads_back_as_the_pixels() {
        let img = Rgba {
            width: 3,
            height: 2,
            pixels: (0..24).collect(),
        };
        let bytes = img.to_png().unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!((info.width, info.height), (3, 2));
        assert_eq!(&buf[..info.buffer_size()], &img.pixels[..]);
        let short = Rgba {
            width: 3,
            height: 2,
            pixels: vec![0; 5],
        };
        assert_eq!(short.to_png().unwrap_err().code(), "RE2212");
    }
}
