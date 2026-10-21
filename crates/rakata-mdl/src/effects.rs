//! The headers of lights, emitters and saber blades, as
//! `docs/formats/mdl.md` decodes them ("Light header", "Emitter header",
//! "Saber header"). Fields whose meaning the files do not establish carry
//! the torlack document's NWN names where they sit at its places, and say
//! so.

use crate::error::Result;
use crate::model::{Data, texture_name};
use crate::view::{Attribute, Vec2s, Vec3s};

const LIGHT_FLARE_RADIUS: usize = 0;
const LIGHT_FLARE_SIZES: usize = 16;
const LIGHT_FLARE_POSITIONS: usize = 28;
const LIGHT_FLARE_COLORS: usize = 40;
const LIGHT_FLARE_TEXTURES: usize = 52;
const LIGHT_PRIORITY: usize = 64;

const SABER_ARRAYS: usize = 0;
const SABER_WORDS: usize = 12;

/// A light node's header.
#[derive(Debug, Clone, PartialEq)]
pub struct Light<'a> {
    /// The flare radius: non-zero exactly when the light has flares, in
    /// vanilla.
    pub flare_radius: f32,
    /// The lens flares, in stored order.
    pub flares: Vec<Flare<'a>>,
    /// The priority, 1 to 5 in vanilla (-7 once).
    pub priority: i32,
    /// Whether the light is ambient only (the word at 68).
    pub ambient_only: bool,
    /// The word at 72, 0 to 2 in vanilla: the torlack document's dynamic
    /// type, by position alone.
    pub dynamic_type: u32,
    /// The word at 76: the torlack document's affect-dynamic flag, by
    /// position alone.
    pub affect_dynamic: bool,
    /// Whether the light casts shadows (the word at 80).
    pub shadow: bool,
    /// The word at 84, where the torlack document puts the generate-flare
    /// flag: zero in every vanilla light, those with flares too.
    pub generate_flare: bool,
    /// The word at 88: the torlack document's fading flag, by position
    /// alone.
    pub fading: bool,
}

/// One lens flare of a light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flare<'a> {
    /// The flare's texture name.
    pub texture: &'a str,
    /// Its size.
    pub size: f32,
    /// Its position along the flare.
    pub position: f32,
    /// Its colour shift.
    pub color_shift: [f32; 3],
}

/// A particle emitter node's header.
#[derive(Debug, Clone, PartialEq)]
pub struct Emitter<'a> {
    /// The float at 0, the torlack document's dead space; set only on
    /// `Fountain` and `Explosion` emitters in vanilla.
    pub dead_space: f32,
    /// The blast radius; only `Explosion` emitters set it.
    pub blast_radius: f32,
    /// The blast length; only `Explosion` emitters set it.
    pub blast_length: f32,
    /// The words at 12 and 16, set only on `Lightning` emitters; their
    /// meaning is not established.
    pub lightning_words: [u32; 2],
    /// The X grid: with [`y_grid`](Self::y_grid), the cells of the
    /// texture's frames, which the frame controllers index.
    pub x_grid: u32,
    /// The Y grid.
    pub y_grid: u32,
    /// The word at 28, the torlack document's space type; 1 in 74
    /// `Fountain` emitters, else zero.
    pub space_type: u32,
    /// How particles are born: `Single`, `Fountain`, `Explosion` or
    /// `Lightning` in vanilla.
    pub update: &'a str,
    /// How particles are drawn: `Billboard_to_Local_Z`, `Normal`,
    /// `Billboard_to_World_Z`, `Motion_Blur`, `Linked`,
    /// `Aligned_to_Particle_Dir` or `Aligned_to_World_Z` in vanilla.
    pub render: &'a str,
    /// How particles blend: `Normal`, `Lighten`, `Punch-Through` or
    /// `PunchThrough` in vanilla.
    pub blend: &'a str,
    /// The texture's name; `None` when the field is empty or `NULL`.
    pub texture: Option<&'a str>,
    /// The model each particle is, if any (the chunk name).
    pub chunk: Option<&'a str>,
    /// The word at 176, the torlack document's two-sided texture flag.
    pub two_sided: bool,
    /// The word at 180, the torlack document's loop flag.
    pub looping: bool,
    /// The render order.
    pub render_order: u16,
    /// The name at 187 as stored: `NULL` on the emitters that carry the
    /// mid-life controllers, empty on the others, `fx_depth` once; what it
    /// names is not established.
    pub name_187: &'a str,
    /// The flags word (see [`emitter_flags`]).
    pub flags: u32,
}

/// The emitter flags' bits, with the torlack document's names for NWN's
/// (`docs/formats/mdl.md`, "Emitter header"). The files support
/// [`P2P`](emitter_flags::P2P) and [`BOUNCE`](emitter_flags::BOUNCE); the
/// others are named by position alone.
pub mod emitter_flags {
    /// Point to point; the P2P bezier controllers act only with it.
    pub const P2P: u32 = 0x0001;
    /// P2P Sel; set on nearly every emitter.
    pub const P2P_SEL: u32 = 0x0002;
    /// Affected by wind.
    pub const AFFECTED_BY_WIND: u32 = 0x0004;
    /// Is tinted.
    pub const IS_TINTED: u32 = 0x0008;
    /// Bounce; most emitters with a bounce coefficient have it.
    pub const BOUNCE: u32 = 0x0010;
    /// Random.
    pub const RANDOM: u32 = 0x0020;
    /// Inherit.
    pub const INHERIT: u32 = 0x0040;
    /// Inherit velocity.
    pub const INHERIT_VEL: u32 = 0x0080;
    /// Inherit local.
    pub const INHERIT_LOCAL: u32 = 0x0100;
    /// Splat.
    pub const SPLAT: u32 = 0x0200;
    /// Inherit part.
    pub const INHERIT_PART: u32 = 0x0400;
}

/// A saber blade's vertices, from the saber header's three per-vertex
/// arrays; [`blade_triangles`](Saber::blade_triangles) gives its surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Saber<'a> {
    /// The positions (equal to the mesh's own positions in vanilla).
    pub positions: Vec3s<'a>,
    /// The texture coordinates.
    pub uv: Vec2s<'a>,
    /// The normals.
    pub normals: Vec3s<'a>,
    /// The words at 12 and 16, 86 to 98 in vanilla and never equal; their
    /// meaning is not established.
    pub words: [u32; 2],
}

impl Saber<'_> {
    /// The blade's surface as twelve triangles, wound as a mesh's faces
    /// are (counter-clockwise about the stored normal). The vertices form
    /// 44 columns of four, one row each along the blade; in every vanilla
    /// blade the columns at vertex 0 and vertex 88 are its centre line,
    /// the column at 4 its right edge (texture u 1) and the column at 92
    /// its left edge (u 0), and the other forty repeat the centre line
    /// (`docs/formats/mdl.md`, "Saber header"). The triangles are the two
    /// strips between them, centre to right and centre to left, three
    /// quads each. The mesh's stored faces are not the blade: in vanilla
    /// they are sequential triples of vertices 0 to 33. Empty for a blade
    /// of fewer than 96 vertices.
    pub fn blade_triangles(&self) -> Vec<[u16; 3]> {
        if self.positions.len() < 96 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(12);
        for r in 0..3 {
            // Centre to right edge: centre, edge, next edge, next centre.
            let (c, e) = (r, 4 + r);
            out.push([c, e, e + 1]);
            out.push([c, e + 1, c + 1]);
        }
        for r in 0..3 {
            // Centre to left edge, the other way round to face the same
            // side.
            let (c, l) = (88 + r, 92 + r);
            out.push([c, c + 1, l + 1]);
            out.push([c, l + 1, l]);
        }
        out
    }
}

/// The light header at `h`.
pub(crate) fn read_light<'a>(d: &Data<'a>, h: usize) -> Result<Light<'a>> {
    d.slice(h, crate::LIGHT_HEADER_LEN, "light header")?;
    let floats = |field: usize, per: usize, what: &'static str| -> Result<(usize, usize)> {
        let (off, n) = d.array(h + field, what)?;
        d.slice(off, n * 4 * per, what)?;
        Ok((off, n))
    };
    let (sizes, ns) = floats(LIGHT_FLARE_SIZES, 1, "flare sizes")?;
    let (positions, np) = floats(LIGHT_FLARE_POSITIONS, 1, "flare positions")?;
    let (colors, nc) = floats(LIGHT_FLARE_COLORS, 3, "flare colours")?;
    let (names, nt) = floats(LIGHT_FLARE_TEXTURES, 1, "flare textures")?;
    let n = ns.min(np).min(nc).min(nt);
    let flares = (0..n)
        .map(|i| {
            let p = d.u32(names + 4 * i, "flare texture pointer")? as usize;
            Ok(Flare {
                texture: d.c_str(p, "flare texture")?,
                size: d.f32(sizes + 4 * i)?,
                position: d.f32(positions + 4 * i)?,
                color_shift: d.f32s::<3>(colors + 12 * i)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let word = |i: usize| d.u32(h + LIGHT_PRIORITY + 4 * i, "light word");
    Ok(Light {
        flare_radius: d.f32(h + LIGHT_FLARE_RADIUS)?,
        flares,
        priority: word(0)? as i32,
        ambient_only: word(1)? != 0,
        dynamic_type: word(2)?,
        affect_dynamic: word(3)? != 0,
        shadow: word(4)? != 0,
        generate_flare: word(5)? != 0,
        fading: word(6)? != 0,
    })
}

/// The emitter header at `h`.
pub(crate) fn read_emitter<'a>(d: &Data<'a>, h: usize) -> Result<Emitter<'a>> {
    d.slice(h, crate::EMITTER_HEADER_LEN, "emitter header")?;
    let w = |at: usize| d.u32(h + at, "emitter word");
    let name = |at: usize, len: usize, what: &'static str| d.field_str(h + at, len, what);
    Ok(Emitter {
        dead_space: d.f32(h)?,
        blast_radius: d.f32(h + 4)?,
        blast_length: d.f32(h + 8)?,
        lightning_words: [w(12)?, w(16)?],
        x_grid: w(20)?,
        y_grid: w(24)?,
        space_type: w(28)?,
        update: name(32, 32, "emitter update")?,
        render: name(64, 32, "emitter render")?,
        blend: name(96, 32, "emitter blend")?,
        texture: texture_name(name(128, 32, "emitter texture")?),
        chunk: texture_name(name(160, 16, "emitter chunk")?),
        two_sided: w(176)? != 0,
        looping: w(180)? != 0,
        render_order: d.u16(h + 184)?,
        name_187: name(187, 32, "emitter name")?,
        flags: w(220)?,
    })
}

/// The saber header at `h`, of a mesh of `vertices` vertices.
pub(crate) fn read_saber<'a>(d: &Data<'a>, h: usize, vertices: usize) -> Result<Saber<'a>> {
    d.slice(h, crate::SABER_HEADER_LEN, "saber header")?;
    let array = |i: usize, floats: usize| -> Result<&'a [u8]> {
        let off = d.u32(h + SABER_ARRAYS + 4 * i, "saber array")? as usize;
        if vertices == 0 {
            return Ok(&[]);
        }
        d.slice(off, vertices * 4 * floats, "saber array")
    };
    Ok(Saber {
        positions: Attribute::new(array(0, 3)?, 12, 0, vertices),
        uv: Attribute::new(array(1, 2)?, 8, 0, vertices),
        normals: Attribute::new(array(2, 3)?, 12, 0, vertices),
        words: [
            d.u32(h + SABER_WORDS, "saber word")?,
            d.u32(h + SABER_WORDS + 4, "saber word")?,
        ],
    })
}
