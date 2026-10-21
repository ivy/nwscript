//! `rakata texture show`: a TPC's header, pixel format, images and mip
//! levels and its TXI; a standalone TXI's instructions; a TGA's header.
//! `rakata texture export`: a TPC or a TGA as PNG, with its TXI beside it.
//! Both over `rakata-texture`.

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use serde_json::{Value as Json, json};

use super::Ctx;
use super::inspect::{self, LookupArgs, resource_forms};
use crate::Exit;
use rakata::diag::SourceMap;
use rakata::inspect::diagnostic;
use rakata::texture::{ImageType, Layout, Levels, Origin, PixelFormat, Png, Tga, Tpc, Txi};

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

const EXPORT: &str = concat!(
    "\
Writes a TPC or a TGA as an 8-bit RGBA PNG. A TPC's DXT1 and DXT5 blocks are
decoded as the S3TC specification defines them; its grey, RGB and RGBA pixels
are written in the channel order the file stores. Level 0, the largest, is
written unless --level names another or --all-levels asks for each. A cube
map's six faces are stacked top to bottom in the order stored, an animation
sheet's frames laid out across then down, so the PNG has the header's size;
--image N writes one face or frame alone.

The PNG goes to --out, or to NAME.png in the current directory. With
--all-levels, level N goes to STEM.N.png beside it. The texture's TXI is
written beside the PNG as STEM.txi: the text after a TPC's pixels when it
carries some, otherwise the NAME.txi found as the texture was.

",
    resource_forms!(),
    "

Examples:
  rakata texture export lhr_flr01.tpc --game DIR
  rakata texture export fnt_d16x16.tpc --game DIR --out fonts/fnt_d16x16.png
  rakata texture export lhr_flr01.tpc --game DIR --all-levels --out mips/lhr_flr01.png
  rakata texture export m01aa_01a_lm0.tga --game DIR
  rakata texture export ./cm_bright.tpc --image 0"
);

#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  rakata texture show lhr_flr01.tpc --game DIR
  rakata texture export lhr_flr01.tpc --game DIR --out lhr_flr01.png")]
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
    /// Write a TPC or a TGA as PNG, with its TXI beside it.
    #[command(after_help = EXPORT)]
    Export {
        /// The texture: NAME.tpc or NAME.tga, a PATH or an ARCHIVE!MEMBER.
        #[arg(value_name = "RESOURCE")]
        name: String,
        /// Where to write the PNG; NAME.png in the current directory by default.
        #[arg(short, long, value_name = "PATH")]
        out: Option<PathBuf>,
        /// The mip level to write, from 0.
        #[arg(long, value_name = "N", conflicts_with = "all_levels")]
        level: Option<usize>,
        /// Write every mip level, level N to STEM.N.png.
        #[arg(long)]
        all_levels: bool,
        /// One cube face or animation frame, from 0, instead of all laid out together.
        #[arg(long, value_name = "N")]
        image: Option<usize>,
        #[command(flatten)]
        lookup: LookupArgs,
    },
}

pub fn run(ctx: &Ctx, args: &TextureArgs) -> Exit {
    match &args.action {
        TextureAction::Show { name, lookup } => show(ctx, name, lookup),
        TextureAction::Export {
            name,
            out,
            level,
            all_levels,
            image,
            lookup,
        } => {
            let levels = match (level, all_levels) {
                (_, true) => Levels::All,
                (Some(l), false) => Levels::One(*l),
                (None, false) => Levels::Largest,
            };
            export(ctx, name, out.as_deref(), levels, *image, lookup)
        }
    }
}

fn export(
    ctx: &Ctx,
    name: &str,
    out: Option<&Path>,
    levels: Levels,
    image: Option<usize>,
    lookup: &LookupArgs,
) -> Exit {
    let mut env = ctx.envelope("texture export");
    let map = SourceMap::new();
    let fail = |env, d| ctx.finish(env, "", vec![d], &map, Exit::Ok);
    let lookup = lookup.lookup(ctx);
    let loaded = match inspect::load(&mut env, "resource", name, &lookup) {
        Ok(l) => l,
        Err(d) => return fail(env, d),
    };
    let ext = loaded.extension();
    let read = match ext.as_deref() {
        Some("tpc") => Tpc::parse(&loaded.bytes).and_then(|t| {
            let pngs = t.to_pngs(levels, image)?;
            let embedded = (!t.txi_bytes.is_empty()).then(|| t.txi_bytes.to_vec());
            Ok((pngs, embedded, t.level_count()))
        }),
        Some("tga") => Tga::parse(&loaded.bytes).and_then(|t| tga_png(&t, levels, image)),
        _ => {
            return fail(
                env,
                inspect::unread_type(&loaded, "texture export", "a .tpc or .tga"),
            );
        }
    };
    let (pngs, embedded, level_count) = match read {
        Ok(r) => r,
        Err(e) => {
            let d = diagnostic(e.code(), &e).note(format!("while exporting `{}`", loaded.source));
            return fail(env, d);
        }
    };
    // The TXI after a TPC's pixels, else the NAME.txi found as the texture was.
    let txi = match embedded {
        Some(bytes) => Some((bytes, None)),
        None => match inspect::with_extension(name, "txi") {
            Some(n) => match inspect::load_if_found(&mut env, "txi", &n, &lookup) {
                Ok(t) => t.map(|t| (t.bytes, Some(t.source))),
                Err(d) => return fail(env, d),
            },
            None => None,
        },
    };

    let stem = loaded
        .name
        .rsplit_once('.')
        .map_or(loaded.name.as_str(), |(s, _)| s);
    let png_path = out.map_or_else(|| PathBuf::from(format!("{stem}.png")), Path::to_path_buf);
    let base = match png_path.extension() {
        Some(e) if e.eq_ignore_ascii_case("png") => png_path.with_extension(""),
        _ => png_path.clone(),
    };
    let beside = |suffix: &str| {
        let mut s = base.clone().into_os_string();
        s.push(suffix);
        PathBuf::from(s)
    };
    let mut text = Vec::new();
    let mut written = Vec::new();
    for p in &pngs {
        let path = if levels == Levels::All {
            beside(&format!(".{}.png", p.level))
        } else {
            png_path.clone()
        };
        if let Err(d) = inspect::write(&mut env, &path, &p.bytes) {
            return fail(env, d);
        }
        text.push(format!(
            "wrote {}  {}x{}, level {} of {level_count}",
            path.display(),
            p.width,
            p.height,
            p.level
        ));
        written.push(json!({
            "path": path.display().to_string(),
            "level": p.level,
            "width": p.width,
            "height": p.height,
        }));
    }
    let txi_json = match txi {
        Some((bytes, from)) => {
            let path = beside(".txi");
            if let Err(d) = inspect::write(&mut env, &path, &bytes) {
                return fail(env, d);
            }
            text.push(match &from {
                None => format!("wrote {}  from the TPC", path.display()),
                Some(s) => format!("wrote {}  from {s}", path.display()),
            });
            json!({
                "path": path.display().to_string(),
                "embedded": from.is_none(),
                "source": from.unwrap_or_else(|| loaded.source.clone()),
            })
        }
        None => Json::Null,
    };
    env.result = json!({
        "source": loaded.source,
        "format": ext,
        "levels": level_count,
        "image": image,
        "pngs": written,
        "txi": txi_json,
    });
    ctx.finish(env, &text.join("\n"), Vec::new(), &map, Exit::Ok)
}

/// A TGA's one image as PNG: level 0 and image 0 are all it has.
fn tga_png(
    t: &Tga,
    levels: Levels,
    image: Option<usize>,
) -> rakata::texture::Result<(Vec<Png>, Option<Vec<u8>>, usize)> {
    if let Levels::One(level @ 1..) = levels {
        return Err(rakata::texture::Error::LevelMissing { level, levels: 1 });
    }
    if let Some(image @ 1..) = image {
        return Err(rakata::texture::Error::ImageMissing { image, images: 1 });
    }
    let png = Png {
        level: 0,
        width: u32::from(t.width),
        height: u32::from(t.height),
        bytes: t.to_png()?,
    };
    Ok((vec![png], None, 1))
}

fn show(ctx: &Ctx, name: &str, lookup: &LookupArgs) -> Exit {
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
