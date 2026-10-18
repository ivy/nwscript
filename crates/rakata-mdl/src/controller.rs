//! A node's controllers: the keyed values of its position, orientation
//! and other properties, borrowed from the node's controller data.

use crate::error::{Error, Result};
use crate::model::{Data, Quaternion};

/// The length of one controller record in the MDL.
pub const CONTROLLER_LEN: usize = 16;

/// The controller types this crate names. Any other type is read all the
/// same, its keys available as rows of floats; what a type means depends
/// on the kind of node that carries it (88 is a light's radius and an
/// emitter's birth rate).
pub mod controller_types {
    /// Position, three floats: the node header's position in a model's
    /// geometry. In an animation the keys of most nodes lie near the
    /// origin, as offsets from the rest position would; the files do not
    /// settle how the engine applies them (`docs/formats/mdl.md`).
    pub const POSITION: u32 = 8;
    /// Orientation, a quaternion stored `x, y, z, w`, or compressed into
    /// one 32-bit word (see [`decompress_quaternion`](crate::decompress_quaternion)).
    pub const ORIENTATION: u32 = 20;
    /// Scale, one float.
    pub const SCALE: u32 = 36;
    /// A mesh's self-illumination colour, three floats.
    pub const SELF_ILLUM_COLOR: u32 = 100;
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
