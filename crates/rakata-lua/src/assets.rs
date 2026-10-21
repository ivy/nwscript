//! `rakata.tpc`, `rakata.txi`, `rakata.tga`, `rakata.lyt`, `rakata.vis`,
//! `rakata.walkmesh` and `rakata.mdl`: the asset formats, each read from
//! bytes or the install and written back to bytes.
//!
//! The TPC, walkmesh and model readers borrow from their input. A TPC and a
//! model keep the bytes they were read from and read them again for each
//! call, a TPC's TXI held apart so a script may replace it; a walkmesh owns
//! its vertex array and trailer apart from the rest, which a script may
//! edit. Vectors are sequences `{x, y, z}`, rotations `{w, x, y, z}`, and
//! bulk arrays (pixels, a mesh's MDX block, a walkmesh's vertices) are
//! also given whole as Lua strings of the bytes as stored.

use mlua::{Lua, Table, UserData, UserDataFields, UserDataMethods, UserDataRef};
use rakata::game::ModuleName;
use rakata::lyt::{Lyt, Placement, Vis};
use rakata::mdl::{
    Animation, Attribute, BoundingBox, Classification, Controller, Emitter, Light, Mesh, Model,
    ModelFile, Node, NodeKind, Quaternion, Saber, Skin, controller_types,
};
use rakata::texture::{ImageType, Layout, Origin, PixelFormat, Rgba, Tga, Tpc, Txi, TxiEntry};
use rakata::walkmesh::{EdgeRef, Kind, SurfaceMaterials, Vertices, Walkmesh};

use crate::error::{Lift, bad};
use crate::formats::LTwoDa;
use crate::install::LInstall;
use crate::values::{bytes, res_id};

fn seq<const N: usize>(lua: &Lua, v: [f32; N]) -> mlua::Result<Table> {
    lua.create_sequence_from(v)
}

/// A rotation as `{w, x, y, z}`.
fn quat(lua: &Lua, q: Quaternion) -> mlua::Result<Table> {
    seq(lua, [q.w, q.x, q.y, q.z])
}

fn bbox(lua: &Lua, b: BoundingBox) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("min", seq(lua, b.min)?)?;
    t.set("max", seq(lua, b.max)?)?;
    Ok(t)
}

/// The bytes of the copy of `name` the install loads, `ext` the extension
/// a bare name takes; no copy is `RE0404`.
fn load(
    install: &LInstall,
    name: &str,
    ext: &str,
    module: Option<String>,
) -> mlua::Result<Vec<u8>> {
    let id = if name.contains('.') {
        res_id(name)?
    } else {
        res_id(&format!("{name}.{ext}"))?
    };
    let module = module.map(|m| ModuleName::new(&m));
    match install.0.read(id, module.as_ref()).lift()? {
        Some((_, b)) => Ok(b),
        None => Err::<Vec<u8>, _>(rakata::game::Error::NotFound {
            id,
            module: module.as_ref().map(ToString::to_string),
        })
        .lift(),
    }
}

/// `rakata.NAME.load(install, name, [module])` for a format read from one
/// resource of extension `ext`.
fn loader<T: UserData + 'static>(
    lua: &Lua,
    ext: &'static str,
    make: fn(Vec<u8>) -> mlua::Result<T>,
) -> mlua::Result<mlua::Function> {
    lua.create_function(
        move |_, (install, name, module): (UserDataRef<LInstall>, String, Option<String>)| {
            make(load(&install, &name, ext, module)?)
        },
    )
}

// ---- TXI ----

/// A texture's instructions.
#[derive(Clone)]
pub struct LTxi(pub Txi);

impl UserData for LTxi {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("preamble", |lua, this, ()| {
            lua.create_sequence_from(this.0.preamble.iter().cloned())
        });
        m.add_method("entries", |lua, this, ()| {
            let t = lua.create_table()?;
            for (i, e) in this.0.entries.iter().enumerate() {
                let et = lua.create_table()?;
                et.set("key", e.key.as_str())?;
                et.set("value", e.value.as_str())?;
                et.set("rows", lua.create_sequence_from(e.rows.iter().cloned())?)?;
                t.raw_set(i + 1, et)?;
            }
            Ok(t)
        });
        m.add_method("get", |_, this, key: String| {
            Ok(this.0.value(&key).map(str::to_owned))
        });
        m.add_method("int", |_, this, key: String| Ok(this.0.int(&key)));
        m.add_method_mut("set", |_, this, (key, value): (String, String)| match this
            .0
            .entries
            .iter_mut()
            .find(|e| e.key.eq_ignore_ascii_case(&key))
        {
            Some(e) => Ok(Some(std::mem::replace(&mut e.value, value))),
            None => {
                this.0.entries.push(TxiEntry {
                    key,
                    value,
                    rows: Vec::new(),
                });
                Ok(None)
            }
        });
        m.add_method("len", |_, this, ()| Ok(this.0.entries.len()));
        m.add_meta_method("__len", |_, this, ()| Ok(this.0.entries.len()));
        m.add_method("to_bytes", |lua, this, ()| {
            bytes(lua, &this.0.to_bytes().lift()?)
        });
        m.add_meta_method("__tostring", |_, this, ()| {
            Ok(format!("Txi({} entries)", this.0.entries.len()))
        });
    }
}

// ---- TPC ----

/// A TPC: the bytes it was read from, and the TXI it writes.
pub struct LTpc {
    data: Vec<u8>,
    txi: Option<Txi>,
}

impl LTpc {
    fn new(data: Vec<u8>) -> mlua::Result<Self> {
        let txi = Tpc::parse(&data).lift()?.txi;
        Ok(LTpc { data, txi })
    }

    fn tpc(&self) -> Tpc<'_> {
        Tpc::parse(&self.data).expect("the bytes read when the TPC was made")
    }

    /// Level `level` (0 when nil) of the whole texture, or of the one image
    /// `image` names, as RGBA8.
    fn decode(&self, level: Option<usize>, image: Option<usize>) -> mlua::Result<Rgba> {
        let t = self.tpc();
        let level = level.unwrap_or(0);
        match image {
            Some(i) => t.decode_image(i, level),
            None => t.decode_level(level),
        }
        .lift()
    }
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

impl UserData for LTpc {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("width", |_, this| Ok(this.tpc().width));
        f.add_field_method_get("height", |_, this| Ok(this.tpc().height));
        f.add_field_method_get("format", |_, this| Ok(format_name(this.tpc().format)));
        f.add_field_method_get("mip_count", |_, this| Ok(this.tpc().mip_count));
        f.add_field_method_get("data_size", |_, this| Ok(this.tpc().data_size));
        f.add_field_method_get("alpha_mean", |_, this| Ok(this.tpc().alpha_mean));
        f.add_field_method_get("has_alpha", |_, this| Ok(this.tpc().has_alpha()));
        f.add_field_method_get("layout", |_, this| {
            Ok(match this.tpc().layout {
                Layout::Single => "single",
                Layout::Cube => "cube",
                Layout::Flipbook { .. } => "flipbook",
            })
        });
        f.add_field_method_get("frames", |lua, this| match this.tpc().layout {
            Layout::Flipbook { numx, numy } => {
                let t = lua.create_table()?;
                t.set("numx", numx)?;
                t.set("numy", numy)?;
                Ok(Some(t))
            }
            _ => Ok(None),
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("images", |lua, this, ()| {
            let out = lua.create_table()?;
            for (i, img) in this.tpc().images.iter().enumerate() {
                let levels = lua.create_table()?;
                for (j, l) in img.levels.iter().enumerate() {
                    let lt = lua.create_table()?;
                    lt.set("width", l.width)?;
                    lt.set("height", l.height)?;
                    lt.set("offset", l.offset)?;
                    lt.set("data", bytes(lua, l.data)?)?;
                    levels.raw_set(j + 1, lt)?;
                }
                out.raw_set(i + 1, levels)?;
            }
            Ok(out)
        });
        m.add_method("pixels", |lua, this, ()| bytes(lua, this.tpc().pixels));
        m.add_method(
            "rgba",
            |lua, this, (level, image): (Option<usize>, Option<usize>)| {
                bytes(lua, &this.decode(level, image)?.pixels)
            },
        );
        m.add_method(
            "to_png",
            |lua, this, (level, image): (Option<usize>, Option<usize>)| {
                bytes(lua, &this.decode(level, image)?.to_png().lift()?)
            },
        );
        m.add_method("txi", |_, this, ()| Ok(this.txi.clone().map(LTxi)));
        m.add_method_mut("set_txi", |_, this, txi: Option<UserDataRef<LTxi>>| {
            this.txi = txi.map(|t| t.0.clone());
            Ok(())
        });
        m.add_method("to_bytes", |lua, this, ()| {
            let mut t = this.tpc();
            t.txi = this.txi.clone();
            bytes(lua, &t.to_bytes().lift()?)
        });
        m.add_meta_method("__tostring", |_, this, ()| {
            let t = this.tpc();
            Ok(format!(
                "Tpc({}x{} {}, {} levels)",
                t.width,
                t.height,
                format_name(t.format),
                t.level_count()
            ))
        });
    }
}

// ---- TGA ----

/// A TARGA image, decoded to RGBA8.
pub struct LTga(pub Tga);

impl UserData for LTga {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("width", |_, this| Ok(this.0.width));
        f.add_field_method_get("height", |_, this| Ok(this.0.height));
        f.add_field_method_get("bits", |_, this| Ok(this.0.bits));
        f.add_field_method_get("alpha_bits", |_, this| Ok(this.0.alpha_bits));
        f.add_field_method_get("image_type", |_, this| {
            Ok(match this.0.image_type {
                ImageType::TrueColor => "truecolor",
                ImageType::Grey => "grey",
                ImageType::TrueColorRle => "truecolor_rle",
                ImageType::GreyRle => "grey_rle",
            })
        });
        f.add_field_method_get("origin", |_, this| {
            Ok(match this.0.origin {
                Origin::BottomLeft => "bottom_left",
                Origin::BottomRight => "bottom_right",
                Origin::TopLeft => "top_left",
                Origin::TopRight => "top_right",
            })
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("rgba", |lua, this, ()| bytes(lua, &this.0.rgba));
        m.add_method_mut("set_rgba", |_, this, b: mlua::LuaString| {
            let want = usize::from(this.0.width) * usize::from(this.0.height) * 4;
            let b = b.as_bytes();
            if b.len() != want {
                return Err(bad(format!(
                    "a {}x{} image's pixels are {want} bytes, not {}",
                    this.0.width,
                    this.0.height,
                    b.len()
                )));
            }
            this.0.rgba = b.to_vec();
            Ok(())
        });
        m.add_method("to_png", |lua, this, ()| {
            bytes(lua, &this.0.to_png().lift()?)
        });
        m.add_method("to_bytes", |lua, this, ()| {
            bytes(lua, &this.0.to_bytes().lift()?)
        });
        m.add_meta_method("__tostring", |_, this, ()| {
            Ok(format!("Tga({}x{})", this.0.width, this.0.height))
        });
    }
}

// ---- LYT and VIS ----

/// A room layout.
pub struct LLyt(pub Lyt);

fn placements(lua: &Lua, ps: &[Placement]) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    for (i, p) in ps.iter().enumerate() {
        t.raw_set(i + 1, placement(lua, p)?)?;
    }
    Ok(t)
}

fn placement(lua: &Lua, p: &Placement) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("model", p.model.as_str())?;
    t.set("position", seq(lua, p.position)?)?;
    Ok(t)
}

impl UserData for LLyt {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("file_dependency", |_, this| {
            Ok(this.0.file_dependency.clone())
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("rooms", |lua, this, ()| placements(lua, &this.0.rooms));
        m.add_method("tracks", |lua, this, ()| placements(lua, &this.0.tracks));
        m.add_method("obstacles", |lua, this, ()| {
            placements(lua, &this.0.obstacles)
        });
        m.add_method("door_hooks", |lua, this, ()| {
            let t = lua.create_table()?;
            for (i, d) in this.0.door_hooks.iter().enumerate() {
                let dt = lua.create_table()?;
                dt.set("room", d.room.as_str())?;
                dt.set("name", d.name.as_str())?;
                dt.set("unknown", d.unknown)?;
                dt.set("position", seq(lua, d.position)?)?;
                dt.set("orientation", seq(lua, d.orientation.to_array())?)?;
                t.raw_set(i + 1, dt)?;
            }
            Ok(t)
        });
        m.add_method("room", |lua, this, model: String| {
            this.0.room(&model).map(|p| placement(lua, p)).transpose()
        });
        m.add_method("semantic_eq", |_, this, other: UserDataRef<LLyt>| {
            Ok(this.0.semantic_eq(&other.0))
        });
        m.add_method("to_bytes", |lua, this, ()| {
            bytes(lua, &this.0.to_bytes().lift()?)
        });
        m.add_meta_method("__tostring", |_, this, ()| {
            Ok(format!("Lyt({} rooms)", this.0.rooms.len()))
        });
    }
}

/// A room visibility file.
pub struct LVis(pub Vis);

impl UserData for LVis {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("rooms", |lua, this, ()| {
            let t = lua.create_table()?;
            for (i, r) in this.0.rooms.iter().enumerate() {
                let rt = lua.create_table()?;
                rt.set("room", r.room.as_str())?;
                rt.set(
                    "visible",
                    lua.create_sequence_from(r.visible.iter().cloned())?,
                )?;
                t.raw_set(i + 1, rt)?;
            }
            Ok(t)
        });
        m.add_method("visible", |lua, this, room: String| {
            this.0
                .visible(&room)
                .map(|v| lua.create_sequence_from(v.iter().cloned()))
                .transpose()
        });
        m.add_method("len", |_, this, ()| Ok(this.0.rooms.len()));
        m.add_meta_method("__len", |_, this, ()| Ok(this.0.rooms.len()));
        m.add_method("semantic_eq", |_, this, other: UserDataRef<LVis>| {
            Ok(this.0.semantic_eq(&other.0))
        });
        m.add_method("to_bytes", |lua, this, ()| {
            bytes(lua, &this.0.to_bytes().lift()?)
        });
        m.add_meta_method("__tostring", |_, this, ()| {
            Ok(format!("Vis({} rooms)", this.0.rooms.len()))
        });
    }
}

// ---- Walkmesh ----

/// A walkmesh, its vertices and trailer owned apart from the rest.
pub struct LWalkmesh {
    /// Everything but the vertices and the trailer, which are empty here.
    parts: Walkmesh<'static>,
    vertices: Vec<u8>,
    trailer: Vec<u8>,
}

impl LWalkmesh {
    fn new(data: Vec<u8>) -> mlua::Result<Self> {
        let w = Walkmesh::parse(&data).lift()?;
        Ok(LWalkmesh {
            vertices: w.vertices.as_bytes().to_vec(),
            trailer: w.trailer.to_vec(),
            parts: Walkmesh {
                kind: w.kind,
                relative_points: w.relative_points,
                absolute_points: w.absolute_points,
                position: w.position,
                vertices: Vertices::from_bytes(&[]).expect("no vertices"),
                faces: w.faces,
                adjacency: w.adjacency,
                perimeter_edges: w.perimeter_edges,
                perimeters: w.perimeters,
                aabb: w.aabb,
                unknown: w.unknown,
                trailer: &[],
            },
        })
    }

    fn walkmesh(&self) -> Walkmesh<'_> {
        Walkmesh {
            vertices: Vertices::from_bytes(&self.vertices).expect("twelve bytes a vertex"),
            trailer: &self.trailer,
            ..self.parts.clone()
        }
    }
}

fn edge(lua: &Lua, e: EdgeRef) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("face", e.face)?;
    t.set("edge", e.edge)?;
    Ok(t)
}

impl UserData for LWalkmesh {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("kind", |_, this| {
            Ok(match this.parts.kind {
                Kind::Area => "area",
                Kind::Object => "object",
            })
        });
        f.add_field_method_get("position", |lua, this| seq(lua, this.parts.position));
        f.add_field_method_get("relative_points", |lua, this| {
            let [a, b] = this.parts.relative_points;
            lua.create_sequence_from([seq(lua, a)?, seq(lua, b)?])
        });
        f.add_field_method_get("absolute_points", |lua, this| {
            let [a, b] = this.parts.absolute_points;
            lua.create_sequence_from([seq(lua, a)?, seq(lua, b)?])
        });
        f.add_field_method_get("unknown", |_, this| Ok(this.parts.unknown));
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("vertices", |lua, this, ()| {
            let t = lua.create_table()?;
            for (i, v) in this.walkmesh().vertices.iter().enumerate() {
                t.raw_set(i + 1, seq(lua, v)?)?;
            }
            Ok(t)
        });
        m.add_method("vertex_bytes", |lua, this, ()| bytes(lua, &this.vertices));
        m.add_method("faces", |lua, this, ()| {
            let t = lua.create_table()?;
            for (i, f) in this.parts.faces.iter().enumerate() {
                let ft = lua.create_table()?;
                ft.set("vertices", lua.create_sequence_from(f.vertices)?)?;
                ft.set("material", f.material)?;
                ft.set("normal", seq(lua, f.normal)?)?;
                ft.set("distance", f.distance)?;
                t.raw_set(i + 1, ft)?;
            }
            Ok(t)
        });
        m.add_method("adjacency", |lua, this, ()| {
            let t = lua.create_table()?;
            for (i, edges) in this.parts.adjacency.iter().enumerate() {
                let et = lua.create_table()?;
                for (k, e) in edges.iter().enumerate() {
                    if let Some(e) = e {
                        et.raw_set(k + 1, edge(lua, *e)?)?;
                    }
                }
                t.raw_set(i + 1, et)?;
            }
            Ok(t)
        });
        m.add_method("perimeter_edges", |lua, this, ()| {
            let t = lua.create_table()?;
            for (i, p) in this.parts.perimeter_edges.iter().enumerate() {
                let pt = edge(lua, p.edge)?;
                pt.set("transition", p.transition)?;
                t.raw_set(i + 1, pt)?;
            }
            Ok(t)
        });
        m.add_method("perimeters", |lua, this, ()| {
            lua.create_sequence_from(this.parts.perimeters.iter().copied())
        });
        m.add_method("aabb", |lua, this, ()| {
            let t = lua.create_table()?;
            for (i, n) in this.parts.aabb.iter().enumerate() {
                let nt = lua.create_table()?;
                nt.set("min", seq(lua, n.min)?)?;
                nt.set("max", seq(lua, n.max)?)?;
                nt.set("face", n.face)?;
                nt.set("unknown", n.unknown)?;
                nt.set("plane", n.plane)?;
                nt.set("left", n.left)?;
                nt.set("right", n.right)?;
                t.raw_set(i + 1, nt)?;
            }
            Ok(t)
        });
        m.add_method("walkable_faces", |lua, this, table: UserDataRef<LTwoDa>| {
            let table = SurfaceMaterials::from_2da(&table.0).lift()?;
            let w = this.walkmesh();
            lua.create_sequence_from(w.walkable_faces(&table))
        });
        m.add_method_mut("set_material", |_, this, (face, material): (usize, u32)| {
            let n = this.parts.faces.len();
            let f = this
                .parts
                .faces
                .get_mut(face)
                .ok_or_else(|| bad(format!("face {face} of a walkmesh with {n} faces")))?;
            Ok(std::mem::replace(&mut f.material, material))
        });
        m.add_method("semantic_eq", |_, this, other: UserDataRef<LWalkmesh>| {
            Ok(this.walkmesh().semantic_eq(&other.walkmesh()))
        });
        m.add_method("to_bytes", |lua, this, ()| {
            bytes(lua, &this.walkmesh().to_bytes().lift()?)
        });
        m.add_meta_method("__tostring", |_, this, ()| {
            Ok(format!(
                "Walkmesh({} vertices, {} faces)",
                this.vertices.len() / 12,
                this.parts.faces.len()
            ))
        });
    }
}

// ---- MDL ----

/// A model: its MDL and MDX, read again for each call.
pub struct LModel {
    mdl: Vec<u8>,
    mdx: Vec<u8>,
}

impl LModel {
    fn new(mdl: Vec<u8>, mdx: Vec<u8>) -> mlua::Result<Self> {
        Model::parse(&mdl, &mdx).lift()?;
        Ok(LModel { mdl, mdx })
    }

    fn model(&self) -> Model<'_> {
        Model::parse(&self.mdl, &self.mdx).expect("the bytes read when the model was made")
    }
}

fn classification_name(c: Classification) -> &'static str {
    match c {
        Classification::Other => "other",
        Classification::Effect => "effect",
        Classification::Tile => "tile",
        Classification::Character => "character",
        Classification::Door => "door",
        Classification::Lightsaber => "lightsaber",
        Classification::Placeable => "placeable",
        Classification::Unknown(_) => "unknown",
    }
}

fn kind_name(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Dummy => "dummy",
        NodeKind::Light => "light",
        NodeKind::Emitter => "emitter",
        NodeKind::Reference => "reference",
        NodeKind::Trimesh => "trimesh",
        NodeKind::Skin => "skin",
        NodeKind::AnimMesh => "animmesh",
        NodeKind::Dangly => "dangly",
        NodeKind::Aabb => "aabb",
        NodeKind::Saber => "saber",
        NodeKind::Other(_) => "other",
    }
}

fn attribute<const N: usize>(lua: &Lua, a: &Attribute<'_, N>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    for (i, v) in a.iter().enumerate() {
        t.raw_set(i + 1, seq(lua, v)?)?;
    }
    Ok(t)
}

fn mesh_table(lua: &Lua, m: &Mesh<'_>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("texture", m.texture)?;
    t.set("lightmap", m.lightmap)?;
    t.set("texture_count", m.texture_count)?;
    t.set("lightmapped", m.lightmapped)?;
    t.set("render", m.render)?;
    t.set("shadow", m.shadow)?;
    t.set("transparency_hint", m.transparency_hint)?;
    if let Some(tm) = &m.texture_motion {
        let mt = lua.create_table()?;
        mt.set("u_rate", tm.u_rate)?;
        mt.set("v_rate", tm.v_rate)?;
        mt.set("floats_244_248", seq(lua, tm.floats_244_248)?)?;
        t.set("texture_motion", mt)?;
    }
    t.set("area", m.area)?;
    t.set("diffuse", seq(lua, m.diffuse)?)?;
    t.set("ambient", seq(lua, m.ambient)?)?;
    t.set("bounding_box", bbox(lua, m.bounding_box)?)?;
    t.set("radius", m.radius)?;
    t.set("average", seq(lua, m.average)?)?;
    t.set("vertex_count", m.vertex_count)?;
    t.set("mdx_flags", m.mdx_flags)?;
    t.set("stride", m.stride)?;
    t.set("vertex_buffer", bytes(lua, m.vertex_buffer)?)?;
    let offsets = lua.create_table()?;
    let from_block =
        |b: &[u8]| !m.vertex_buffer.is_empty() && b.as_ptr() == m.vertex_buffer.as_ptr();
    if from_block(m.positions.buffer()) {
        offsets.set("position", m.positions.offset())?;
    }
    t.set("positions", attribute(lua, &m.positions)?)?;
    if let Some(a) = &m.normals {
        if from_block(a.buffer()) {
            offsets.set("normal", a.offset())?;
        }
        t.set("normals", attribute(lua, a)?)?;
    }
    if let Some(a) = &m.uv1 {
        if from_block(a.buffer()) {
            offsets.set("uv1", a.offset())?;
        }
        t.set("uv1", attribute(lua, a)?)?;
    }
    if let Some(a) = &m.uv2 {
        if from_block(a.buffer()) {
            offsets.set("uv2", a.offset())?;
        }
        t.set("uv2", attribute(lua, a)?)?;
    }
    t.set("offsets", offsets)?;
    let faces = lua.create_table()?;
    for (i, f) in m.faces.iter().enumerate() {
        let ft = lua.create_table()?;
        ft.set("normal", seq(lua, f.normal)?)?;
        ft.set("distance", f.distance)?;
        ft.set("surface", f.surface)?;
        ft.set("adjacent", lua.create_sequence_from(f.adjacent)?)?;
        ft.set("vertices", lua.create_sequence_from(f.vertices)?)?;
        faces.raw_set(i + 1, ft)?;
    }
    t.set("faces", faces)?;
    if let Some(ix) = &m.indices {
        t.set("indices", lua.create_sequence_from(ix.iter())?)?;
    }
    Ok(t)
}

fn skin_table(lua: &Lua, s: &Skin<'_>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("bones", lua.create_sequence_from(s.bones.iter().copied())?)?;
    t.set("weights", attribute(lua, &s.weights)?)?;
    t.set("bone_indices", attribute(lua, &s.bone_indices)?)?;
    let bind = lua.create_table()?;
    for &node in &s.bones {
        let b = lua.create_table()?;
        if let Some(q) = s.qbone(node) {
            b.set("rotation", quat(lua, q)?)?;
        }
        if let Some(v) = s.tbone(node) {
            b.set("translation", seq(lua, v)?)?;
        }
        bind.raw_set(node, b)?;
    }
    t.set("bind", bind)?;
    Ok(t)
}

fn light_table(lua: &Lua, l: &Light<'_>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("flare_radius", l.flare_radius)?;
    let flares = lua.create_table()?;
    for (i, f) in l.flares.iter().enumerate() {
        let ft = lua.create_table()?;
        ft.set("texture", f.texture)?;
        ft.set("size", f.size)?;
        ft.set("position", f.position)?;
        ft.set("color_shift", seq(lua, f.color_shift)?)?;
        flares.raw_set(i + 1, ft)?;
    }
    t.set("flares", flares)?;
    t.set("priority", l.priority)?;
    t.set("ambient_only", l.ambient_only)?;
    t.set("dynamic_type", l.dynamic_type)?;
    t.set("affect_dynamic", l.affect_dynamic)?;
    t.set("shadow", l.shadow)?;
    t.set("generate_flare", l.generate_flare)?;
    t.set("fading", l.fading)?;
    Ok(t)
}

fn emitter_table(lua: &Lua, e: &Emitter<'_>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("dead_space", e.dead_space)?;
    t.set("blast_radius", e.blast_radius)?;
    t.set("blast_length", e.blast_length)?;
    t.set(
        "lightning_words",
        lua.create_sequence_from(e.lightning_words)?,
    )?;
    t.set("x_grid", e.x_grid)?;
    t.set("y_grid", e.y_grid)?;
    t.set("space_type", e.space_type)?;
    t.set("update", e.update)?;
    t.set("render", e.render)?;
    t.set("blend", e.blend)?;
    t.set("texture", e.texture)?;
    t.set("chunk", e.chunk)?;
    t.set("two_sided", e.two_sided)?;
    t.set("looping", e.looping)?;
    t.set("render_order", e.render_order)?;
    t.set("name_187", e.name_187)?;
    t.set("flags", e.flags)?;
    Ok(t)
}

fn saber_table(lua: &Lua, s: &Saber<'_>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("positions", attribute(lua, &s.positions)?)?;
    t.set("uv", attribute(lua, &s.uv)?)?;
    t.set("normals", attribute(lua, &s.normals)?)?;
    let tris = lua.create_table()?;
    for (i, tri) in s.blade_triangles().into_iter().enumerate() {
        tris.raw_set(i + 1, lua.create_sequence_from(tri)?)?;
    }
    t.set("triangles", tris)?;
    t.set("words", lua.create_sequence_from(s.words)?)?;
    Ok(t)
}

fn controller_table(lua: &Lua, c: &Controller<'_>, node: NodeKind) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("kind", c.kind)?;
    t.set("name", controller_types::name(node, c.kind))?;
    t.set("field", c.field)?;
    t.set("columns", c.columns)?;
    t.set("is_bezier", c.is_bezier())?;
    t.set("is_compressed", c.is_compressed())?;
    let keys = lua.create_table()?;
    match c.orientation_keys() {
        Some(ks) => {
            for (i, k) in ks.enumerate() {
                let kt = lua.create_table()?;
                kt.set("time", k.time)?;
                kt.set("value", quat(lua, k.value)?)?;
                keys.raw_set(i + 1, kt)?;
            }
        }
        None => {
            for i in 0..c.len() {
                let kt = lua.create_table()?;
                kt.set("time", c.time(i))?;
                if let Some(fs) = c.floats(i) {
                    kt.set("value", lua.create_sequence_from(fs)?)?;
                }
                keys.raw_set(i + 1, kt)?;
            }
        }
    }
    t.set("keys", keys)?;
    Ok(t)
}

fn node_table(lua: &Lua, n: &Node<'_>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("name", n.name)?;
    t.set("kind", kind_name(n.kind))?;
    t.set("flags", n.flags)?;
    t.set("number", n.number)?;
    t.set("supernode", n.supernode)?;
    t.set("parent", n.parent)?;
    t.set(
        "children",
        lua.create_sequence_from(n.children.iter().copied())?,
    )?;
    t.set("position", seq(lua, n.position)?)?;
    t.set("orientation", quat(lua, n.orientation)?)?;
    if let Some(m) = &n.mesh {
        t.set("mesh", mesh_table(lua, m)?)?;
    }
    if let Some(s) = &n.skin {
        t.set("skin", skin_table(lua, s)?)?;
    }
    if let Some(l) = &n.light {
        t.set("light", light_table(lua, l)?)?;
    }
    if let Some(e) = &n.emitter {
        t.set("emitter", emitter_table(lua, e)?)?;
    }
    if let Some(s) = &n.saber {
        t.set("saber", saber_table(lua, s)?)?;
    }
    let cs = lua.create_table()?;
    for (i, c) in n.controllers.iter().enumerate() {
        cs.raw_set(i + 1, controller_table(lua, c, n.kind)?)?;
    }
    t.set("controllers", cs)?;
    Ok(t)
}

fn nodes_table(lua: &Lua, ns: &[Node<'_>]) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    for (i, n) in ns.iter().enumerate() {
        t.raw_set(i + 1, node_table(lua, n)?)?;
    }
    Ok(t)
}

fn animation_table(lua: &Lua, a: &Animation<'_>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("name", a.name)?;
    t.set("length", a.length)?;
    t.set("transition", a.transition)?;
    t.set("root", a.root)?;
    let events = lua.create_table()?;
    for (i, e) in a.events.iter().enumerate() {
        let et = lua.create_table()?;
        et.set("time", e.time)?;
        et.set("name", e.name)?;
        events.raw_set(i + 1, et)?;
    }
    t.set("events", events)?;
    t.set("nodes", nodes_table(lua, &a.nodes)?)?;
    Ok(t)
}

impl UserData for LModel {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("name", |_, this| Ok(this.model().name.to_owned()));
        f.add_field_method_get("classification", |_, this| {
            Ok(classification_name(this.model().classification))
        });
        f.add_field_method_get("subclassification", |_, this| {
            Ok(this.model().subclassification)
        });
        f.add_field_method_get("fog", |_, this| Ok(this.model().fog));
        f.add_field_method_get("supermodel", |_, this| {
            Ok(this.model().supermodel.map(str::to_owned))
        });
        f.add_field_method_get("radius", |_, this| Ok(this.model().radius));
        f.add_field_method_get("animation_scale", |_, this| {
            Ok(this.model().animation_scale)
        });
        f.add_field_method_get("bounding_box", |lua, this| {
            bbox(lua, this.model().bounding_box)
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("nodes", |lua, this, ()| {
            nodes_table(lua, &this.model().nodes)
        });
        m.add_method("node", |lua, this, name: String| {
            this.model()
                .node(&name)
                .map(|n| node_table(lua, n))
                .transpose()
        });
        m.add_method("animations", |lua, this, ()| {
            let t = lua.create_table()?;
            for (i, a) in this.model().animations.iter().enumerate() {
                t.raw_set(i + 1, animation_table(lua, a)?)?;
            }
            Ok(t)
        });
        m.add_method("animation", |lua, this, name: String| {
            this.model()
                .animation(&name)
                .map(|a| animation_table(lua, a))
                .transpose()
        });
        m.add_method("to_bytes", |lua, this, ()| {
            let (mdl, mdx) = ModelFile::parse(&this.mdl, &this.mdx)
                .lift()?
                .to_bytes()
                .lift()?;
            Ok((bytes(lua, &mdl)?, bytes(lua, &mdx)?))
        });
        m.add_meta_method("__tostring", |_, this, ()| {
            let m = this.model();
            Ok(format!("Model({}, {} nodes)", m.name, m.nodes.len()))
        });
    }
}

/// The asset tables.
pub fn install(lua: &Lua, rakata: &Table) -> mlua::Result<()> {
    let t = lua.create_table()?;
    t.set(
        "parse",
        lua.create_function(|_, b: mlua::LuaString| Ok(LTxi(Txi::parse(&b.as_bytes()))))?,
    )?;
    t.set(
        "new",
        lua.create_function(|_, ()| Ok(LTxi(Txi::default())))?,
    )?;
    t.set("load", loader(lua, "txi", |b| Ok(LTxi(Txi::parse(&b))))?)?;
    rakata.set("txi", t)?;

    let t = lua.create_table()?;
    t.set(
        "parse",
        lua.create_function(|_, b: mlua::LuaString| LTpc::new(b.as_bytes().to_vec()))?,
    )?;
    t.set("load", loader(lua, "tpc", LTpc::new)?)?;
    rakata.set("tpc", t)?;

    let t = lua.create_table()?;
    t.set(
        "parse",
        lua.create_function(|_, b: mlua::LuaString| Ok(LTga(Tga::parse(&b.as_bytes()).lift()?)))?,
    )?;
    t.set(
        "load",
        loader(lua, "tga", |b| Ok(LTga(Tga::parse(&b).lift()?)))?,
    )?;
    rakata.set("tga", t)?;

    let t = lua.create_table()?;
    t.set(
        "parse",
        lua.create_function(|_, b: mlua::LuaString| Ok(LLyt(Lyt::parse(&b.as_bytes()).lift()?)))?,
    )?;
    t.set(
        "load",
        loader(lua, "lyt", |b| Ok(LLyt(Lyt::parse(&b).lift()?)))?,
    )?;
    rakata.set("lyt", t)?;

    let t = lua.create_table()?;
    t.set(
        "parse",
        lua.create_function(|_, b: mlua::LuaString| Ok(LVis(Vis::parse(&b.as_bytes()).lift()?)))?,
    )?;
    t.set(
        "load",
        loader(lua, "vis", |b| Ok(LVis(Vis::parse(&b).lift()?)))?,
    )?;
    rakata.set("vis", t)?;

    let t = lua.create_table()?;
    t.set(
        "parse",
        lua.create_function(|_, b: mlua::LuaString| LWalkmesh::new(b.as_bytes().to_vec()))?,
    )?;
    t.set("load", loader(lua, "wok", LWalkmesh::new)?)?;
    rakata.set("walkmesh", t)?;

    let t = lua.create_table()?;
    t.set(
        "parse",
        lua.create_function(
            |_, (mdl, mdx): (mlua::LuaString, Option<mlua::LuaString>)| {
                let mdx = mdx.map(|b| b.as_bytes().to_vec()).unwrap_or_default();
                LModel::new(mdl.as_bytes().to_vec(), mdx)
            },
        )?,
    )?;
    t.set(
        "load",
        lua.create_function(
            |_, (install, name, module): (UserDataRef<LInstall>, String, Option<String>)| {
                let stem = name.split_once('.').map_or(name.as_str(), |(s, _)| s);
                let mdl = load(&install, stem, "mdl", module.clone())?;
                let mdx = load(&install, stem, "mdx", module)?;
                LModel::new(mdl, mdx)
            },
        )?,
    )?;
    rakata.set("mdl", t)?;
    Ok(())
}
