//! A node's controllers: the keyed values of its position, orientation
//! and other properties, borrowed from the node's controller data.

use crate::error::{Error, Result};
use crate::model::{Data, Quaternion};

/// The length of one controller record in the MDL.
pub const CONTROLLER_LEN: usize = 16;

/// The controller types of `docs/formats/mdl.md` ("Controllers"). What a
/// type means depends on the kind of node that carries it: 88 is a
/// light's [`light::RADIUS`] and an emitter's [`emitter::BIRTH_RATE`], 100
/// a mesh's [`mesh::SELF_ILLUM_COLOR`], a light's
/// [`light::VERTICAL_DISPLACEMENT`] and an emitter's [`emitter::DRAG`]. So
/// the types every node carries are named here, and the rest in a module
/// per kind; [`name`] gives a type's name on a kind of node. Any type is
/// read all the same, its keys available as rows of floats.
pub mod controller_types {
    use crate::NodeKind;

    /// Position, three floats: the node header's position in a model's
    /// geometry. In an animation the keys of most nodes lie near the
    /// origin, as offsets from the rest position would; the files do not
    /// settle how the engine applies them (`docs/formats/mdl.md`).
    pub const POSITION: u32 = 8;
    /// Orientation, a quaternion stored `x, y, z, w`, or compressed into
    /// one 32-bit word (see [`decompress_quaternion`](crate::decompress_quaternion)).
    pub const ORIENTATION: u32 = 20;
    /// Scale, one float; on meshes and on a few dummies.
    pub const SCALE: u32 = 36;
    /// A mesh's self-illumination colour: [`mesh::SELF_ILLUM_COLOR`].
    pub const SELF_ILLUM_COLOR: u32 = mesh::SELF_ILLUM_COLOR;

    /// The types of a node with a mesh header.
    pub mod mesh {
        /// Self-illumination colour, three floats. Shares 100 with
        /// [`light::VERTICAL_DISPLACEMENT`](super::light::VERTICAL_DISPLACEMENT)
        /// and [`emitter::DRAG`](super::emitter::DRAG).
        pub const SELF_ILLUM_COLOR: u32 = 100;
        /// Alpha, one float: 1 on nearly every mesh, less on water, beams
        /// and holograms. Shares 132 with
        /// [`emitter::P2P_BEZIER_3`](super::emitter::P2P_BEZIER_3).
        pub const ALPHA: u32 = 132;
    }

    /// The types of a light (`docs/formats/mdl.md`, "Controllers"), the
    /// torlack document's NWN numbers unchanged.
    pub mod light {
        /// Colour, three floats; negative on the lights that darken.
        pub const COLOR: u32 = 76;
        /// Radius, one float. Shares 88 with
        /// [`emitter::BIRTH_RATE`](super::emitter::BIRTH_RATE).
        pub const RADIUS: u32 = 88;
        /// Shadow radius, one float; on `fx_flame01`'s light only. Shares
        /// 96 with [`emitter::COMBINE_TIME`](super::emitter::COMBINE_TIME).
        pub const SHADOW_RADIUS: u32 = 96;
        /// Vertical displacement, one float; on `fx_flame01`'s light only.
        /// Shares 100 with [`mesh::SELF_ILLUM_COLOR`](super::mesh::SELF_ILLUM_COLOR)
        /// and [`emitter::DRAG`](super::emitter::DRAG).
        pub const VERTICAL_DISPLACEMENT: u32 = 100;
        /// Multiplier, one float. Shares 140 with
        /// [`emitter::RANDOM_VELOCITY`](super::emitter::RANDOM_VELOCITY).
        pub const MULTIPLIER: u32 = 140;
    }

    /// The types of an emitter (`docs/formats/mdl.md`, "Controllers"): the
    /// torlack document's NWN numbers, but that its two colours are
    /// elsewhere and every NWN type after them is 24 less, and the
    /// mid-life types KotOR adds. One float each but the colours, three.
    /// Many numbers here are a light's or a mesh's type too; read them by
    /// the node's kind.
    pub mod emitter {
        /// Alpha at the end of a particle's life.
        pub const ALPHA_END: u32 = 80;
        /// Alpha at the start of a particle's life.
        pub const ALPHA_START: u32 = 84;
        /// Particles born per second. Shares 88 with
        /// [`light::RADIUS`](super::light::RADIUS).
        pub const BIRTH_RATE: u32 = 88;
        /// Bounce coefficient.
        pub const BOUNCE_COEFFICIENT: u32 = 92;
        /// Combine time; non-zero in one vanilla emitter. Shares 96 with
        /// [`light::SHADOW_RADIUS`](super::light::SHADOW_RADIUS).
        pub const COMBINE_TIME: u32 = 96;
        /// Drag. Shares 100 with [`mesh::SELF_ILLUM_COLOR`](super::mesh::SELF_ILLUM_COLOR)
        /// and [`light::VERTICAL_DISPLACEMENT`](super::light::VERTICAL_DISPLACEMENT).
        pub const DRAG: u32 = 100;
        /// Frames per second of the texture's frames.
        pub const FPS: u32 = 104;
        /// The last frame, an index into the texture's grid of cells.
        pub const FRAME_END: u32 = 108;
        /// The first frame; at most [`FRAME_END`] in every vanilla emitter
        /// but one.
        pub const FRAME_START: u32 = 112;
        /// Gravity.
        pub const GRAVITY: u32 = 116;
        /// A particle's life expectancy in seconds; -1 in most.
        pub const LIFE_EXPECTANCY: u32 = 120;
        /// Mass.
        pub const MASS: u32 = 124;
        /// P2P bezier 2; non-zero only with the P2P flag.
        pub const P2P_BEZIER_2: u32 = 128;
        /// P2P bezier 3; non-zero only with the P2P flag. Shares 132 with
        /// [`mesh::ALPHA`](super::mesh::ALPHA).
        pub const P2P_BEZIER_3: u32 = 132;
        /// Particle rotation.
        pub const PARTICLE_ROTATION: u32 = 136;
        /// Random velocity. Shares 140 with
        /// [`light::MULTIPLIER`](super::light::MULTIPLIER).
        pub const RANDOM_VELOCITY: u32 = 140;
        /// Size at the start of a particle's life.
        pub const SIZE_START: u32 = 144;
        /// Size at the end.
        pub const SIZE_END: u32 = 148;
        /// Y size at the start.
        pub const SIZE_START_Y: u32 = 152;
        /// Y size at the end.
        pub const SIZE_END_Y: u32 = 156;
        /// Spread, radians.
        pub const SPREAD: u32 = 160;
        /// Threshold.
        pub const THRESHOLD: u32 = 164;
        /// Velocity.
        pub const VELOCITY: u32 = 168;
        /// The emitter's X size.
        pub const X_SIZE: u32 = 172;
        /// The emitter's Y size.
        pub const Y_SIZE: u32 = 176;
        /// Blur length.
        pub const BLUR_LENGTH: u32 = 180;
        /// Lightning delay.
        pub const LIGHTNING_DELAY: u32 = 184;
        /// Lightning radius.
        pub const LIGHTNING_RADIUS: u32 = 188;
        /// Lightning scale.
        pub const LIGHTNING_SCALE: u32 = 192;
        /// Not established; non-zero mostly on `Lightning` emitters.
        pub const TYPE_196: u32 = 196;
        /// Not established; non-zero only on `Lightning` emitters.
        pub const TYPE_200: u32 = 200;
        /// Alpha at mid-life.
        pub const ALPHA_MID: u32 = 216;
        /// The fraction of life at which the start values hold.
        pub const PERCENT_START: u32 = 220;
        /// The fraction of life at which the mid-life values hold.
        pub const PERCENT_MID: u32 = 224;
        /// The fraction of life at which the end values hold.
        pub const PERCENT_END: u32 = 228;
        /// Size at mid-life.
        pub const SIZE_MID: u32 = 232;
        /// Y size at mid-life.
        pub const SIZE_MID_Y: u32 = 236;
        /// Not established; non-zero mostly on `Fountain` and `Explosion`
        /// emitters.
        pub const TYPE_240: u32 = 240;
        /// Not established; non-zero mostly on `Lightning` emitters.
        pub const TYPE_252: u32 = 252;
        /// Not established; non-zero mostly on `Lightning` emitters.
        pub const TYPE_256: u32 = 256;
        /// Not established; non-zero mostly on `Lightning` emitters.
        pub const TYPE_260: u32 = 260;
        /// Not established; non-zero only on `Lightning` emitters.
        pub const TYPE_264: u32 = 264;
        /// Not established; non-zero mostly on `Lightning` emitters.
        pub const TYPE_268: u32 = 268;
        /// Not established; non-zero mostly on `Lightning` emitters.
        pub const TYPE_272: u32 = 272;
        /// Colour at mid-life, three floats.
        pub const COLOR_MID: u32 = 284;
        /// Colour at the end of a particle's life, three floats.
        pub const COLOR_END: u32 = 380;
        /// Colour at the start of a particle's life, three floats.
        pub const COLOR_START: u32 = 392;
    }

    /// The name of controller type `ty` on a node of kind `kind`, in
    /// lower case with underscores (`birth_rate`); `None` for a type the
    /// kind does not carry in vanilla, or one whose meaning is not
    /// established.
    pub fn name(kind: NodeKind, ty: u32) -> Option<&'static str> {
        let common = match ty {
            POSITION => Some("position"),
            ORIENTATION => Some("orientation"),
            _ => None,
        };
        let own = match kind {
            NodeKind::Light => match ty {
                light::COLOR => Some("color"),
                light::RADIUS => Some("radius"),
                light::SHADOW_RADIUS => Some("shadow_radius"),
                light::VERTICAL_DISPLACEMENT => Some("vertical_displacement"),
                light::MULTIPLIER => Some("multiplier"),
                _ => None,
            },
            NodeKind::Emitter => emitter_name(ty),
            NodeKind::Trimesh
            | NodeKind::Skin
            | NodeKind::AnimMesh
            | NodeKind::Dangly
            | NodeKind::Aabb
            | NodeKind::Saber => match ty {
                SCALE => Some("scale"),
                mesh::SELF_ILLUM_COLOR => Some("self_illum_color"),
                mesh::ALPHA => Some("alpha"),
                _ => None,
            },
            NodeKind::Dummy | NodeKind::Reference | NodeKind::Other(_) => match ty {
                SCALE => Some("scale"),
                _ => None,
            },
        };
        common.or(own)
    }

    fn emitter_name(ty: u32) -> Option<&'static str> {
        use emitter::*;
        Some(match ty {
            ALPHA_END => "alpha_end",
            ALPHA_START => "alpha_start",
            BIRTH_RATE => "birth_rate",
            BOUNCE_COEFFICIENT => "bounce_coefficient",
            COMBINE_TIME => "combine_time",
            DRAG => "drag",
            FPS => "fps",
            FRAME_END => "frame_end",
            FRAME_START => "frame_start",
            GRAVITY => "gravity",
            LIFE_EXPECTANCY => "life_expectancy",
            MASS => "mass",
            P2P_BEZIER_2 => "p2p_bezier_2",
            P2P_BEZIER_3 => "p2p_bezier_3",
            PARTICLE_ROTATION => "particle_rotation",
            RANDOM_VELOCITY => "random_velocity",
            SIZE_START => "size_start",
            SIZE_END => "size_end",
            SIZE_START_Y => "size_start_y",
            SIZE_END_Y => "size_end_y",
            SPREAD => "spread",
            THRESHOLD => "threshold",
            VELOCITY => "velocity",
            X_SIZE => "x_size",
            Y_SIZE => "y_size",
            BLUR_LENGTH => "blur_length",
            LIGHTNING_DELAY => "lightning_delay",
            LIGHTNING_RADIUS => "lightning_radius",
            LIGHTNING_SCALE => "lightning_scale",
            ALPHA_MID => "alpha_mid",
            PERCENT_START => "percent_start",
            PERCENT_MID => "percent_mid",
            PERCENT_END => "percent_end",
            SIZE_MID => "size_mid",
            SIZE_MID_Y => "size_mid_y",
            COLOR_MID => "color_mid",
            COLOR_END => "color_end",
            COLOR_START => "color_start",
            _ => return None,
        })
    }
}

/// The bit of the column byte that marks a bezier-keyed controller.
pub const BEZIER: u8 = 0x10;

/// One controller of a node: a type, and rows of keys, each a time and a
/// value. The times and values are read in place from the node's
/// controller data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Controller<'a> {
    /// The controller type (see [`controller_types`]).
    pub kind: u32,
    /// The u16 after the type: 0xFFFF in every controller of a vanilla
    /// model's geometry; in an animation, 16 on position controllers and 28
    /// on orientation controllers (the offsets of those fields in the node
    /// header), 0xFFFF on the rest.
    pub field: u16,
    /// The column byte as stored: the value's floats, with [`BEZIER`] set
    /// on a bezier-keyed controller.
    pub columns: u8,
    rows: usize,
    width: usize,
    times: &'a [u8],
    values: &'a [u8],
}

/// One key of a controller: a time and a value; a bezier key also holds
/// the two further values its row stores after the value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key<T> {
    /// The time, in seconds from the animation's start; zero in a
    /// model's geometry.
    pub time: f32,
    /// The value.
    pub value: T,
    /// A bezier key's two control values, in stored order.
    pub bezier: Option<[T; 2]>,
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// The rotation a compressed orientation key's word holds. The word packs
/// the quaternion's `x` in its low 11 bits, `y` in the next 11 and `z` in
/// the high 10, each mapping 0 to -1 and its greatest value to 1; `w` is
/// the non-negative root that makes the quaternion unit length (zero when
/// the three already reach it).
pub fn decompress_quaternion(word: u32) -> Quaternion {
    let x = (word & 0x7FF) as f32 / 1023.5 - 1.0;
    let y = ((word >> 11) & 0x7FF) as f32 / 1023.5 - 1.0;
    let z = (word >> 22) as f32 / 511.5 - 1.0;
    let m = x * x + y * y + z * z;
    let w = if m < 1.0 { (1.0 - m).sqrt() } else { 0.0 };
    Quaternion { w, x, y, z }
}

impl<'a> Controller<'a> {
    /// The number of keys.
    pub fn len(&self) -> usize {
        self.rows
    }

    /// Whether there are no keys.
    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }

    /// Whether the controller is bezier keyed: each row stores three
    /// values' worth of floats, the value and two control values.
    pub fn is_bezier(&self) -> bool {
        self.columns & BEZIER != 0
    }

    /// Whether this is an orientation controller whose keys are each one
    /// compressed word (two columns as stored).
    pub fn is_compressed(&self) -> bool {
        self.kind == controller_types::ORIENTATION && self.columns == 2
    }

    /// The floats one value holds: the column count without the bezier
    /// bit (4 for a compressed orientation, whose word stands for four).
    pub fn value_len(&self) -> usize {
        if self.is_compressed() {
            4
        } else {
            (self.columns & 0x0F) as usize
        }
    }

    /// The 4-byte words each row of values occupies.
    pub fn row_len(&self) -> usize {
        self.width
    }

    /// The time keys as stored, four bytes each.
    pub fn time_bytes(&self) -> &'a [u8] {
        self.times
    }

    /// The values as stored, [`row_len`](Self::row_len) words per key.
    pub fn value_bytes(&self) -> &'a [u8] {
        self.values
    }

    /// Key `i`'s time.
    pub fn time(&self, i: usize) -> Option<f32> {
        (i < self.rows).then(|| f32_at(self.times, 4 * i))
    }

    /// Every key's time in order.
    pub fn times(&self) -> impl ExactSizeIterator<Item = f32> + '_ {
        (0..self.rows).map(|i| f32_at(self.times, 4 * i))
    }

    /// Key `i`'s row of values as stored: [`row_len`](Self::row_len)
    /// little-endian words.
    pub fn row(&self, i: usize) -> Option<&'a [u8]> {
        (i < self.rows).then(|| &self.values[4 * self.width * i..4 * self.width * (i + 1)])
    }

    /// Key `i`'s row read as floats (a compressed orientation's word reads
    /// as the float of the same bits; use
    /// [`orientation`](Self::orientation) for its rotation).
    pub fn floats(&self, i: usize) -> Option<impl ExactSizeIterator<Item = f32> + 'a> {
        let row = self.row(i)?;
        Some((0..self.width).map(move |k| f32_at(row, 4 * k)))
    }

    fn vector<const N: usize>(&self, i: usize) -> Key<[f32; N]> {
        let row = &self.values[4 * self.width * i..];
        let read = |k: usize| -> [f32; N] {
            let mut v = [0.0; N];
            for (j, x) in v.iter_mut().enumerate() {
                *x = f32_at(row, 4 * (N * k + j));
            }
            v
        };
        Key {
            time: f32_at(self.times, 4 * i),
            value: read(0),
            bezier: self.is_bezier().then(|| [read(1), read(2)]),
        }
    }

    /// The keys of a controller of `N`-float values (3 for a position or
    /// a colour, 1 for a scale), linear or bezier; `None` when its values
    /// are not `N` floats or it is a compressed orientation.
    pub fn keys<const N: usize>(
        &self,
    ) -> Option<impl ExactSizeIterator<Item = Key<[f32; N]>> + '_> {
        (self.value_len() == N && !self.is_compressed())
            .then(|| (0..self.rows).map(|i| self.vector::<N>(i)))
    }

    /// The keys of a position controller (or any three-float one).
    pub fn vector_keys(&self) -> Option<impl ExactSizeIterator<Item = Key<[f32; 3]>> + '_> {
        self.keys::<3>()
    }

    /// The keys of a one-float controller.
    pub fn scalar_keys(&self) -> Option<impl ExactSizeIterator<Item = Key<f32>> + '_> {
        self.keys::<1>().map(|it| {
            it.map(|k| Key {
                time: k.time,
                value: k.value[0],
                bezier: k.bezier.map(|[a, b]| [a[0], b[0]]),
            })
        })
    }

    /// Key `i` of an orientation controller as a rotation: four floats
    /// `x, y, z, w`, or a compressed word decoded; `None` past the end or
    /// on a controller that is neither.
    pub fn orientation(&self, i: usize) -> Option<Key<Quaternion>> {
        if self.kind != controller_types::ORIENTATION || i >= self.rows {
            return None;
        }
        let row = self.row(i)?;
        let value = if self.is_compressed() {
            decompress_quaternion(u32::from_le_bytes([row[0], row[1], row[2], row[3]]))
        } else if self.width == 4 {
            Quaternion {
                x: f32_at(row, 0),
                y: f32_at(row, 4),
                z: f32_at(row, 8),
                w: f32_at(row, 12),
            }
        } else {
            return None;
        };
        Some(Key {
            time: f32_at(self.times, 4 * i),
            value,
            bezier: None,
        })
    }

    /// The keys of an orientation controller; `None` on any other.
    pub fn orientation_keys(&self) -> Option<impl ExactSizeIterator<Item = Key<Quaternion>> + '_> {
        (self.kind == controller_types::ORIENTATION && (self.is_compressed() || self.width == 4))
            .then(|| (0..self.rows).map(|i| self.orientation(i).expect("an orientation key")))
    }
}

/// The controllers of the node at `at`, from its controller array and its
/// controller data.
pub(crate) fn read_controllers<'a>(
    d: &Data<'a>,
    at: usize,
    node: &str,
) -> Result<Vec<Controller<'a>>> {
    let (co, cn) = d.array(at + 56, "controller array")?;
    let records = d.slice(co, cn * CONTROLLER_LEN, "controller array")?;
    let (dof, dn) = d.array(at + 68, "controller data")?;
    let data = d.slice(dof, dn * 4, "controller data")?;
    let mut out = Vec::with_capacity(cn);
    for r in records.as_chunks::<CONTROLLER_LEN>().0 {
        let u16_at = |i: usize| u16::from_le_bytes([r[i], r[i + 1]]) as usize;
        let kind = u32::from_le_bytes([r[0], r[1], r[2], r[3]]);
        let field = u16_at(4) as u16;
        let (rows, tk, vk, columns) = (u16_at(6), u16_at(8), u16_at(10), r[12]);
        let width = if kind == controller_types::ORIENTATION && columns == 2 {
            1
        } else if columns & BEZIER != 0 {
            3 * (columns & 0x0F) as usize
        } else {
            (columns & 0x0F) as usize
        };
        let bad = |what: &str, from: usize, len: usize| Error::BadController {
            node: node.to_owned(),
            kind,
            reason: format!(
                "its {what} at float {from} run {len} floats and the node's data has {dn}"
            ),
        };
        let times = data
            .get(4 * tk..4 * (tk + rows))
            .ok_or_else(|| bad("time keys", tk, rows))?;
        let values = data
            .get(4 * vk..4 * (vk + rows * width))
            .ok_or_else(|| bad("values", vk, rows * width))?;
        out.push(Controller {
            kind,
            field,
            columns,
            rows,
            width,
            times,
            values,
        });
    }
    Ok(out)
}
