//! `rakata texture show`: a TPC's header, pixel format, images and mip
//! levels and its TXI; a standalone TXI's instructions; a TGA's header,
//! over `rakata-texture`.

use clap::{Args, Subcommand};
use serde_json::{Value as Json, json};

use super::Ctx;
use super::inspect::{self, LookupArgs, resource_forms};
use crate::Exit;
use rakata::diag::SourceMap;
use rakata::inspect::diagnostic;
use rakata::texture::{ImageType, Layout, Origin, PixelFormat, Tga, Tpc, Txi};

const SHOW: &str = concat!(
    "\
The extension says the format. A TPC prints its size, pixel format, how its
pixels divide into images (one, six cube faces, or the frames of an animation
sheet) and the levels of each, then each level's size, length and offset, then
its TXI's instructions. A TXI prints one `KEY VALUE` line per instruction, with
the count of numeric rows that follow it. A TGA prints its size, image type,
depth, alpha bits and origin. --format json gives the same fields.

",
    resource_forms!(),
    "

Examples:
  rakata texture show lhr_flr01.tpc --game DIR
  rakata texture show m01aa_01a_lm0.tga --game DIR
  rakata --format json texture show 'TexturePacks/swpc_tex_tpa.erf!lhr_flr01.tpc' --game DIR
  rakata texture show ./lhr_flr01.txi"
);

#[derive(Args, Debug)]
#[command(after_help = "\
Example:
  rakata texture show lhr_flr01.tpc --game DIR")]
pub struct TextureArgs {
    #[command(subcommand)]
    pub action: TextureAction,
}

#[derive(Subcommand, Debug)]
pub enum TextureAction {
    /// Print a texture's header, levels and TXI: a TPC, a TXI or a TGA.
    #[command(after_help = SHOW)]
    Show {
        /// The texture: NAME.tpc, NAME.txi or NAME.tga, a PATH or an ARCHIVE!MEMBER.
        #[arg(value_name = "RESOURCE")]
        name: String,
        #[command(flatten)]
        lookup: LookupArgs,
    },
}

pub fn run(ctx: &Ctx, args: &TextureArgs) -> Exit {
    let TextureAction::Show { name, lookup } = &args.action;
    let mut env = ctx.envelope("texture show");
    let map = SourceMap::new();
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let loaded = match inspect::load(&mut env, "resource", name, &lookup.lookup(ctx)) {
        Ok(l) => l,
        Err(d) => return fail(env, d),
    };
    let read = match loaded.extension().as_deref() {
        Some("tpc") => Tpc::parse(&loaded.bytes).map(|t| tpc(&t)),
        Some("tga") => Tga::parse(&loaded.bytes).map(|t| tga(&t)),
        Some("txi") => Ok(txi_only(&Txi::parse(&loaded.bytes))),
        _ => {
            return fail(
                env,
                inspect::unread_type(&loaded, "texture show", "a .tpc, .txi or .tga"),
            );
        }
    };
    let (text, mut result) = match read {
        Ok(r) => r,
        Err(e) => {
            let d = diagnostic(e.code(), &e).note(format!("while reading `{}`", loaded.source));
            return fail(env, d);
        }
    };
    result["source"] = json!(loaded.source);
    env.result = result;
    ctx.finish(env, &text, Vec::new(), &map, Exit::Ok)
}

fn format_name(f: PixelFormat) -> &'static str {
    match f {
        PixelFormat::Grey => "grey",
        PixelFormat::Rgb => "rgb",
        PixelFormat::Rgba => "rgba",
        PixelFormat::Dxt1 => "dxt1",
        PixelFormat::Dxt5 => "dxt5",
    }
}

fn tpc(t: &Tpc<'_>) -> (String, Json) {
    let (iw, ih) = t.image_size();
    let levels = t.level_count();
    let (layout, layout_json) = match t.layout {
        Layout::Single => (
            format!("one image, {levels} levels"),
            json!({"kind": "single"}),
        ),
        Layout::Cube => (
            format!("cube of 6 faces {iw}x{ih}, {levels} levels each"),
            json!({"kind": "cube"}),
        ),
        Layout::Flipbook { numx, numy } => (
            format!("flipbook of {numx}x{numy} frames {iw}x{ih}, {levels} levels each"),
            json!({"kind": "flipbook", "numx": numx, "numy": numy}),
        ),
    };
    let mut text = format!(
        "tpc {}x{} {}, {layout}\nalpha mean {}, data size {}, header mip count {}",
        t.width,
        t.height,
        format_name(t.format),
        t.alpha_mean,
        t.data_size,
        t.mip_count
    );
    text.push_str("\nlevels");
    for (i, l) in t.images[0].levels.iter().enumerate() {
        text.push_str(&format!(
            "\n  {i}  {}x{}  {} bytes  at {}",
            l.width,
            l.height,
            l.data.len(),
            l.offset
        ));
    }
    let txi = t.txi.as_ref().map(txi_json);
    if let Some(x) = &t.txi {
        text.push_str("\ntxi");
        txi_lines(&mut text, x, "  ");
    }
    let images: Vec<Json> = t
        .images
        .iter()
        .map(|img| {
            json!(
                img.levels
                    .iter()
                    .map(|l| json!({
                        "width": l.width,
                        "height": l.height,
                        "offset": l.offset,
                        "len": l.data.len(),
                    }))
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    let json = json!({
        "format": "tpc",
        "width": t.width,
        "height": t.height,
        "pixel_format": format_name(t.format),
        "encoding": t.format.encoding(),
        "compressed": t.format.is_compressed(),
        "alpha": t.has_alpha(),
        "mip_count": t.mip_count,
        "data_size": t.data_size,
        "alpha_mean": t.alpha_mean,
        "layout": layout_json,
        "image_width": iw,
        "image_height": ih,
        "images": images,
        "txi_bytes": t.txi_bytes.len(),
        "txi": txi,
    });
    (text, json)
}

fn txi_lines(text: &mut String, x: &Txi, indent: &str) {
    if !x.preamble.is_empty() {
        text.push_str(&format!(
            "\n{indent}({} numeric lines before any keyword)",
            x.preamble.len()
        ));
    }
    for e in &x.entries {
        text.push_str(&format!("\n{indent}{}", e.key));
        if !e.value.is_empty() {
            text.push_str(&format!(" {}", e.value));
        }
        if !e.rows.is_empty() {
            text.push_str(&format!("  ({} rows)", e.rows.len()));
        }
    }
}

fn txi_json(x: &Txi) -> Json {
    json!({
        "preamble": x.preamble,
        "entries": x.entries.iter().map(|e| json!({
            "key": e.key,
            "value": e.value,
            "rows": e.rows,
        })).collect::<Vec<_>>(),
    })
}

fn txi_only(x: &Txi) -> (String, Json) {
    let mut text = format!("txi, {} instructions", x.entries.len());
    txi_lines(&mut text, x, "");
    let mut json = txi_json(x);
    json["format"] = json!("txi");
    (text, json)
}

fn tga(t: &Tga) -> (String, Json) {
    let kind = match t.image_type {
        ImageType::TrueColor => "true colour",
        ImageType::Grey => "grey",
        ImageType::TrueColorRle => "true colour, run-length encoded",
        ImageType::GreyRle => "grey, run-length encoded",
    };
    let origin = match t.origin {
        Origin::BottomLeft => "bottom left",
        Origin::BottomRight => "bottom right",
        Origin::TopLeft => "top left",
        Origin::TopRight => "top right",
    };
    let text = format!(
        "tga {}x{} {kind}, {} bits, {} alpha bits, origin {origin}",
        t.width, t.height, t.bits, t.alpha_bits
    );
    let json = json!({
        "format": "tga",
        "width": t.width,
        "height": t.height,
        "image_type": t.image_type.byte(),
        "kind": kind,
        "bits": t.bits,
        "alpha_bits": t.alpha_bits,
        "origin": origin,
        "image_id": t.layout.image_id.len(),
        "color_map": t.layout.color_map_type,
        "tail": t.layout.tail.len(),
    });
    (text, json)
}
