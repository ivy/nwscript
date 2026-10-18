//! Textures as formats: a `.tpc` with the TXI after its pixels, a
//! standalone `.txi`, and a `.tga` round-trip through `rakata-texture`'s
//! readers and writers, the TXI's lines that depart from the written form
//! and the TGA's stored layout kept as read.

use rakata_texture::{Tga as TgaImage, Tpc as TpcTexture, Txi as TxiText};

use crate::conformance::{Format, Problem};

/// The engine's texture, `.tpc`.
pub struct Tpc;

/// A texture's instructions, `.txi`.
pub struct Txi;

/// A Truevision TARGA image, `.tga`.
pub struct Tga;

fn problem(e: rakata_texture::Error) -> Problem {
    let code = e.code();
    let text = e.to_string();
    // The error renders as `CODE: message`; the code travels apart.
    let message = text
        .strip_prefix(code)
        .and_then(|t| t.strip_prefix(": "))
        .unwrap_or(&text)
        .to_string();
    Problem::new(code, message)
}

impl Format for Tpc {
    type Model<'a> = TpcTexture<'a>;

    fn name(&self) -> &'static str {
        "tpc"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["tpc"]
    }

    fn read<'a>(&self, bytes: &'a [u8]) -> Result<TpcTexture<'a>, Problem> {
        TpcTexture::parse(bytes).map_err(problem)
    }

    fn write(&self, model: &TpcTexture<'_>) -> Result<Vec<u8>, Problem> {
        model.to_bytes().map_err(problem)
    }

    /// The same header, the header float bit for bit, the same pixels,
    /// and the same TXI instructions.
    fn semantic_eq(&self, a: &TpcTexture<'_>, b: &TpcTexture<'_>) -> bool {
        a.data_size == b.data_size
            && a.alpha_mean.to_bits() == b.alpha_mean.to_bits()
            && a.width == b.width
            && a.height == b.height
            && a.format == b.format
            && a.mip_count == b.mip_count
            && a.reserved == b.reserved
            && a.pixels == b.pixels
            && a.txi == b.txi
    }

    fn layout(&self, _bytes: &[u8], model: &TpcTexture<'_>) -> Option<String> {
        let txi = match &model.txi {
            None => "no txi".to_string(),
            Some(t) => format!("txi: {} lines departing", t.layout.departures()),
        };
        Some(format!(
            "tpc: {:?} {:?}, reserved {}, {txi}",
            model.format,
            model.layout,
            if model.reserved.iter().all(|&b| b == 0) {
                "zero"
            } else {
                "set"
            }
        ))
    }
}

impl Format for Txi {
    type Model<'a> = TxiText;

    fn name(&self) -> &'static str {
        "txi"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["txi"]
    }

    fn read(&self, bytes: &[u8]) -> Result<TxiText, Problem> {
        Ok(TxiText::parse(bytes))
    }

    fn write(&self, model: &TxiText) -> Result<Vec<u8>, Problem> {
        model.to_bytes().map_err(problem)
    }

    /// The same instructions and preamble, in order.
    fn semantic_eq(&self, a: &TxiText, b: &TxiText) -> bool {
        a == b
    }

    fn layout(&self, _bytes: &[u8], model: &TxiText) -> Option<String> {
        Some(format!(
            "txi: {} lines departing, {}",
            model.layout.departures(),
            if model.layout.final_newline() {
                "last line ended"
            } else {
                "last line unended"
            }
        ))
    }
}

impl Format for Tga {
    type Model<'a> = TgaImage;

    fn name(&self) -> &'static str {
        "tga"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["tga"]
    }

    fn read(&self, bytes: &[u8]) -> Result<TgaImage, Problem> {
        TgaImage::parse(bytes).map_err(problem)
    }

    fn write(&self, model: &TgaImage) -> Result<Vec<u8>, Problem> {
        model.to_bytes().map_err(problem)
    }

    /// The same image and header, the same image ID, colour map and tail;
    /// how run-length packets fall is layout.
    fn semantic_eq(&self, a: &TgaImage, b: &TgaImage) -> bool {
        let (la, lb) = (&a.layout, &b.layout);
        a == b
            && la.image_id == lb.image_id
            && la.color_map_type == lb.color_map_type
            && la.color_map_first == lb.color_map_first
            && la.color_map_len == lb.color_map_len
            && la.color_map_bits == lb.color_map_bits
            && la.color_map == lb.color_map
            && la.x_origin == lb.x_origin
            && la.y_origin == lb.y_origin
            && la.descriptor_high == lb.descriptor_high
            && la.attribute_bits == lb.attribute_bits
            && la.tail == lb.tail
    }

    fn layout(&self, _bytes: &[u8], model: &TgaImage) -> Option<String> {
        Some(format!(
            "tga: type {}, {} bits, {} packets, {} bytes after the pixels",
            model.image_type.byte(),
            model.bits,
            model.layout.packets.len(),
            model.layout.tail.len()
        ))
    }
}
