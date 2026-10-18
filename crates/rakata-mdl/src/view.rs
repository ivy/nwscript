//! Borrowed views of a mesh's arrays: vertex attributes strided across an
//! interleaved buffer, the faces, and the triangle index list.

/// One attribute of a mesh's vertices, `N` little-endian `f32`s per
/// vertex, read in place from an interleaved buffer: vertex `i`'s value
/// is the `4 × N` bytes at `offset + i × stride` of [`buffer`](Self::buffer).
///
/// The buffer is a slice of the input (the MDX block of the mesh, or the
/// MDL's own position array when the mesh has no MDX block), so a caller
/// may upload it as it stands and describe the attribute to the GPU by
/// [`stride`](Self::stride) and [`offset`](Self::offset).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attribute<'a, const N: usize> {
    buffer: &'a [u8],
    stride: usize,
    offset: usize,
    count: usize,
}

/// A three-component attribute: positions or normals.
pub type Vec3s<'a> = Attribute<'a, 3>;
/// A two-component attribute: texture coordinates.
pub type Vec2s<'a> = Attribute<'a, 2>;

impl<'a, const N: usize> Attribute<'a, N> {
    /// A view of `count` values; the caller has checked that the last one
    /// lies inside `buffer`.
    pub(crate) fn new(buffer: &'a [u8], stride: usize, offset: usize, count: usize) -> Self {
        debug_assert!(count == 0 || (count - 1) * stride + offset + 4 * N <= buffer.len());
        Attribute {
            buffer,
            stride,
            offset,
            count,
        }
    }

    /// The number of vertices.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether there are no vertices.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The bytes from one vertex to the next.
    pub fn stride(&self) -> usize {
        self.stride
    }

    /// The attribute's offset within each vertex.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// The interleaved buffer the attribute is read from.
    pub fn buffer(&self) -> &'a [u8] {
        self.buffer
    }

    /// Vertex `i`'s value.
    pub fn get(&self, i: usize) -> Option<[f32; N]> {
        if i >= self.count {
            return None;
        }
        let at = i * self.stride + self.offset;
        let mut out = [0.0; N];
        for (k, v) in out.iter_mut().enumerate() {
            let b = &self.buffer[at + 4 * k..at + 4 * k + 4];
            *v = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        }
        Some(out)
    }

    /// Every vertex's value in order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = [f32; N]> + '_ {
        (0..self.count).map(|i| self.get(i).expect("in range"))
    }
}

/// The size of one face record in the MDL.
pub const FACE_LEN: usize = 32;

/// One triangle of a mesh, as the MDL's face record stores it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    /// The plane's unit normal.
    pub normal: [f32; 3],
    /// The plane's distance term.
    pub distance: f32,
    /// The surface (material) id.
    pub surface: u32,
    /// The faces across each edge, or -1.
    pub adjacent: [i16; 3],
    /// The three vertex indices.
    pub vertices: [u16; 3],
}

/// A mesh's faces, read in place from the MDL's 32-byte face records.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Faces<'a> {
    bytes: &'a [u8],
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

impl<'a> Faces<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        debug_assert_eq!(bytes.len() % FACE_LEN, 0);
        Faces { bytes }
    }

    /// The number of faces.
    pub fn len(&self) -> usize {
        self.bytes.len() / FACE_LEN
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The face records as stored, 32 bytes each.
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Face `i`.
    pub fn get(&self, i: usize) -> Option<Face> {
        let r = self.bytes.get(i * FACE_LEN..(i + 1) * FACE_LEN)?;
        let i16_at = |at: usize| i16::from_le_bytes([r[at], r[at + 1]]);
        let u16_at = |at: usize| u16::from_le_bytes([r[at], r[at + 1]]);
        Some(Face {
            normal: [f32_at(r, 0), f32_at(r, 4), f32_at(r, 8)],
            distance: f32_at(r, 12),
            surface: u32::from_le_bytes([r[16], r[17], r[18], r[19]]),
            adjacent: [i16_at(20), i16_at(22), i16_at(24)],
            vertices: [u16_at(26), u16_at(28), u16_at(30)],
        })
    }

    /// Every face in order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = Face> + '_ {
        (0..self.len()).map(|i| self.get(i).expect("in range"))
    }
}

/// A mesh's triangle list: little-endian `u16` vertex indices, three per
/// face, read in place from the MDL. In every vanilla mesh that has one it
/// is its faces' vertex indices in face order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Indices<'a> {
    bytes: &'a [u8],
}

impl<'a> Indices<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        debug_assert_eq!(bytes.len() % 2, 0);
        Indices { bytes }
    }

    /// The number of indices.
    pub fn len(&self) -> usize {
        self.bytes.len() / 2
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The list as stored, ready for a 16-bit index buffer.
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Index `i`.
    pub fn get(&self, i: usize) -> Option<u16> {
        let b = self.bytes.get(2 * i..2 * i + 2)?;
        Some(u16::from_le_bytes([b[0], b[1]]))
    }

    /// Every index in order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = u16> + '_ {
        (0..self.len()).map(|i| self.get(i).expect("in range"))
    }
}
