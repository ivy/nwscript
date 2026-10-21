"""The data of BioWare's Aurora games (KotOR 1 and 2), from Rust: the install, GFF, 2DA, TLK, LIP,
textures (TPC, TXI, TGA), layouts (LYT, VIS), walkmeshes, models (MDL and MDX), dialogues, the
authoring API (`rakata.author`) and the scripts toolchain (`rakata.nwscript`).

Every refusal raises `RakataError` with the registered code the `rakata` binary prints."""

import os
from typing import Literal, TypeAlias

from . import author as author
from . import nwscript as nwscript
from .author import Mod as Mod

__version__: str
"""The build identity, as `rakata --version` prints it: `0.0.0+49a9b40`, with `.dirty.HASH` from a
checkout with uncommitted changes."""
VERSION: str

Profile: TypeAlias = Literal["k1", "k2"]
TexturePack: TypeAlias = Literal["tpa", "tpb", "tpc"]
Kind: TypeAlias = Literal["entry", "reply"]
GffValue: TypeAlias = (
    int | float | str | bytes | tuple[float, ...] | dict[str, object] | list[dict[str, object]]
)
"""A GFF value read: `int` for the integer types and StrRef, `float`, `str` for CExoString and
CResRef, `bytes` for VOID, a tuple for Vector and Orientation, `{"strref": int | None,
"substrings": {id: str}}` for CExoLocString, a dict of fields for a Struct, a list of them for a
List."""
Row: TypeAlias = int | str
StrPath: TypeAlias = str | os.PathLike[str]

class RakataError(Exception):
    """A refusal: `code` is the registered diagnostic code, `message` the text after it, and
    `str()` is `CODE: message`."""

    code: str
    message: str

class Install:
    """A game install: the folder holding `chitin.key`, opened once."""

    def __init__(
        self, path: StrPath, profile: Profile = "k1", texture_pack: TexturePack | None = None
    ) -> None:
        """`texture_pack` is the quality pack the texture layer searches over `swpc_tex_gui.erf`;
        `"tpa"` when none is named, since which pack the game's `Texture Quality` selects is not
        established. Raises RE0405 for any other name."""
    @property
    def root(self) -> str: ...
    @property
    def profile(self) -> Profile: ...
    @property
    def texture_pack(self) -> TexturePack: ...
    def modules(self) -> list[str]: ...
    def which(self, name: str, ext: str | None = None, module: str | None = None) -> Resolution:
        """`which("globalcat", "2da")` or `which("globalcat.2da")`; raises RE0404 when no layer
        holds it."""
    def read(self, name: str, ext: str | None = None, module: str | None = None) -> bytes:
        """The bytes of the copy the engine loads; RE0404 when no layer holds it."""

class Candidate:
    """One copy of a resource the install holds."""

    @property
    def layer(self) -> Literal["override", "module", "patch", "texturepack", "chitin"]: ...
    @property
    def location(self) -> str: ...
    @property
    def path(self) -> str: ...
    @property
    def member(self) -> str | None: ...
    @property
    def index(self) -> int | None: ...
    @property
    def len(self) -> int: ...
    @property
    def hash(self) -> str:
        """SHA-256, hex."""
    @property
    def verdict(self) -> Literal["wins", "ambiguous", "shadowed"]: ...
    @property
    def reason(self) -> str | None: ...

class Resolution:
    """Every copy of a resource, the one the engine loads first."""

    @property
    def id(self) -> str: ...
    @property
    def module(self) -> str | None: ...
    @property
    def candidates(self) -> list[Candidate]: ...
    @property
    def ambiguous(self) -> bool: ...
    @property
    def winner(self) -> Candidate: ...
    @property
    def hash(self) -> str:
        """The winner's SHA-256, hex: `rakata which --format json`'s `winner.sha256`."""

class Gff:
    @staticmethod
    def parse(data: bytes) -> Gff: ...
    @staticmethod
    def new(file_type: str) -> Gff: ...
    @property
    def file_type(self) -> str: ...
    def get(self, path: str) -> GffValue:
        """`Tag`, `ItemList[0].Dropable`; RE0606, RE0607."""
    def field_type(self, path: str) -> str: ...
    def set(self, path: str, value: object, type: str | None = None) -> GffValue | None:
        """The field's own type, or `type` (`"BYTE"`, `"CResRef"`) for one the struct lacks;
        RE1406 for another type. Returns what the field held."""
    def to_bytes(self) -> bytes: ...

class TwoDa:
    @staticmethod
    def parse(data: bytes) -> TwoDa: ...
    @staticmethod
    def new(columns: list[str]) -> TwoDa: ...
    @property
    def columns(self) -> list[str]: ...
    @property
    def rows(self) -> list[tuple[str, list[str]]]: ...
    def column_index(self, name: str) -> int | None: ...
    def row_index(self, row: Row) -> int | None: ...
    def get(self, row: Row, column: str) -> str: ...
    def set(self, row: Row, column: str, value: str) -> None: ...
    def add_row(self, label: str, changes: dict[str, str] | None = None) -> int: ...
    def add_row_copying(
        self, source: Row, new_label: str, changes: dict[str, str] | None = None
    ) -> int: ...
    def add_column(self, name: str, fill: str = "") -> int: ...
    def to_bytes(self) -> bytes: ...
    def __len__(self) -> int: ...

class Tlk:
    @staticmethod
    def open(path: StrPath) -> Tlk: ...
    @staticmethod
    def parse(data: bytes) -> Tlk: ...
    @staticmethod
    def new(language: int = 0) -> Tlk: ...
    @property
    def language(self) -> int: ...
    def text(self, strref: int) -> str | None: ...
    def append(self, text: str, sound: str | None = None, length: float = 0.0) -> int: ...
    def to_bytes(self) -> bytes: ...
    def save(self, path: StrPath) -> None: ...
    def __len__(self) -> int: ...

class Lip:
    @staticmethod
    def parse(data: bytes) -> Lip: ...
    @staticmethod
    def read(path: StrPath) -> Lip: ...
    @staticmethod
    def from_cues(cues: list[tuple[float, str]], length: float) -> Lip:
        """Shapes by label: rest, ee, eh, ah, oh, oo, y, s, f, n, th, m, t, sh, l, k."""
    @property
    def length(self) -> float: ...
    @property
    def keys(self) -> list[tuple[float, str]]: ...
    def to_bytes(self) -> bytes: ...
    def write(self, path: StrPath) -> None: ...

Vec3: TypeAlias = tuple[float, float, float]
Quat: TypeAlias = tuple[float, float, float, float]
"""A rotation as `(w, x, y, z)`."""
Box: TypeAlias = tuple[Vec3, Vec3]
"""A bounding box as `(min, max)`."""

class Txi:
    """A texture's instructions, from a `.txi` or a TPC: keywords, their values and the numeric
    rows after them, in file order."""

    @staticmethod
    def parse(data: bytes) -> Txi:
        """Never fails: bytes are read as Latin-1."""
    @staticmethod
    def read(path: StrPath) -> Txi: ...
    @staticmethod
    def load(install: Install, name: str, module: str | None = None) -> Txi:
        """The `.txi` the install loads by `name`; RE0404 when no layer holds it."""
    @staticmethod
    def new() -> Txi: ...
    @property
    def preamble(self) -> list[str]: ...
    @property
    def entries(self) -> list[tuple[str, str, list[str]]]:
        """`(keyword, value, rows)`, in file order."""
    def get(self, key: str) -> str | None:
        """The first value of `key`, in any case."""
    def int(self, key: str) -> int | None: ...
    def set(self, key: str, value: str) -> str | None:
        """Set the first instruction with `key`, or append one; returns the value it had."""
    def to_bytes(self) -> bytes:
        """RE2208 for text that would not read back as itself."""
    def __len__(self) -> int: ...

class TpcLevel:
    """One mip level of a TPC image."""

    @property
    def width(self) -> int: ...
    @property
    def height(self) -> int: ...
    @property
    def offset(self) -> int:
        """Where the level begins in the TPC's bytes."""
    @property
    def data(self) -> bytes:
        """The level as stored: DXT1 or DXT5 blocks, or grey, RGB or RGBA pixels."""

class Tpc:
    """A TPC texture: the header, its images and their mip levels with the pixel format kept as
    stored, and its TXI."""

    @staticmethod
    def parse(data: bytes) -> Tpc:
        """RE2201, RE2202, RE2203."""
    @staticmethod
    def read(path: StrPath) -> Tpc: ...
    @staticmethod
    def load(install: Install, name: str, module: str | None = None) -> Tpc:
        """The `.tpc` the install loads by `name`, through its texture pack; RE0404."""
    @property
    def width(self) -> int: ...
    @property
    def height(self) -> int:
        """For a cube map, six faces' worth."""
    @property
    def format(self) -> Literal["grey", "rgb", "rgba", "dxt1", "dxt5"]: ...
    @property
    def mip_count(self) -> int:
        """The header's count; an animation sheet says 1."""
    @property
    def data_size(self) -> int: ...
    @property
    def alpha_mean(self) -> float: ...
    @property
    def layout(self) -> Literal["single", "cube", "flipbook"]: ...
    @property
    def frames(self) -> tuple[int, int] | None:
        """A flipbook's `(numx, numy)`."""
    @property
    def has_alpha(self) -> bool: ...
    @property
    def image_size(self) -> tuple[int, int]: ...
    @property
    def level_count(self) -> int: ...
    @property
    def images(self) -> list[list[TpcLevel]]:
        """One image, six cube faces or the frames in row order, each its levels largest first."""
    @property
    def pixels(self) -> bytes: ...
    @property
    def txi(self) -> Txi | None:
        """A copy; setting it replaces the TXI `to_bytes` writes."""
    @txi.setter
    def txi(self, value: Txi | None) -> None: ...
    def rgba(self, level: int = 0, image: int | None = None) -> bytes:
        """Level `level` as RGBA8, four bytes a pixel, rows top first: the whole texture at the
        header's size (a cube's faces stacked top to bottom, a sheet's frames across then down),
        or the one face or frame `image` names. DXT1 and DXT5 decoded; grey, RGB and RGBA bytes
        widened as stored. RE2210, RE2211."""
    def to_png(self, level: int = 0, image: int | None = None) -> bytes:
        """What `rgba` gives, as an 8-bit RGBA PNG file; RE2210, RE2211, RE2212."""
    def to_bytes(self) -> bytes:
        """RE2207, RE2208."""

class Tga:
    """A TARGA image decoded to RGBA8, rows top first."""

    @staticmethod
    def parse(data: bytes) -> Tga:
        """RE2204, RE2205, RE2206."""
    @staticmethod
    def read(path: StrPath) -> Tga: ...
    @staticmethod
    def load(install: Install, name: str, module: str | None = None) -> Tga: ...
    @property
    def width(self) -> int: ...
    @property
    def height(self) -> int: ...
    @property
    def image_type(self) -> Literal["truecolor", "grey", "truecolor_rle", "grey_rle"]: ...
    @property
    def bits(self) -> int: ...
    @property
    def alpha_bits(self) -> int: ...
    @property
    def origin(self) -> Literal["bottom_left", "bottom_right", "top_left", "top_right"]: ...
    @property
    def rgba(self) -> bytes:
        """Four bytes a pixel, rows top first; setting it takes `width * height * 4` bytes
        (`ValueError` otherwise)."""
    @rgba.setter
    def rgba(self, value: bytes) -> None: ...
    def to_png(self) -> bytes:
        """The pixels as an 8-bit RGBA PNG file; RE2212."""
    def to_bytes(self) -> bytes:
        """In the stored depth, order and packing; RE2209."""

class Placement:
    """A room, swoop track or obstacle of a layout."""

    @property
    def model(self) -> str: ...
    @property
    def position(self) -> Vec3: ...

class DoorHook:
    @property
    def room(self) -> str: ...
    @property
    def name(self) -> str: ...
    @property
    def unknown(self) -> int: ...
    @property
    def position(self) -> Vec3: ...
    @property
    def orientation(self) -> Quat: ...

class Lyt:
    """An area's room layout."""

    @staticmethod
    def parse(data: bytes) -> Lyt:
        """RE2301 to RE2306."""
    @staticmethod
    def read(path: StrPath) -> Lyt: ...
    @staticmethod
    def load(install: Install, name: str, module: str | None = None) -> Lyt:
        """The `.lyt` by the area's name (`"m01aa"`); RE0404."""
    @property
    def file_dependency(self) -> str | None: ...
    @property
    def rooms(self) -> list[Placement]: ...
    @property
    def tracks(self) -> list[Placement]: ...
    @property
    def obstacles(self) -> list[Placement]: ...
    @property
    def door_hooks(self) -> list[DoorHook]: ...
    def room(self, model: str) -> Placement | None: ...
    def semantic_eq(self, other: Lyt) -> bool: ...
    def to_bytes(self) -> bytes:
        """RE2307."""

class Vis:
    """An area's room visibility."""

    @staticmethod
    def parse(data: bytes) -> Vis:
        """RE2311, RE2312."""
    @staticmethod
    def read(path: StrPath) -> Vis: ...
    @staticmethod
    def load(install: Install, name: str, module: str | None = None) -> Vis: ...
    @property
    def rooms(self) -> list[tuple[str, list[str]]]:
        """`(room, visible rooms)`, in file order."""
    def visible(self, room: str) -> list[str] | None: ...
    def semantic_eq(self, other: Vis) -> bool: ...
    def to_bytes(self) -> bytes: ...
    def __len__(self) -> int: ...

Edge: TypeAlias = tuple[int, int]
"""A walkmesh edge as `(face, edge)`: edge `e` runs from the face's vertex `e` to `(e + 1) % 3`."""

class WalkmeshFace:
    @property
    def vertices(self) -> tuple[int, int, int]: ...
    @property
    def material(self) -> int:
        """The face's row of `surfacemat.2da`."""
    @property
    def normal(self) -> Vec3: ...
    @property
    def distance(self) -> float: ...

class AabbNode:
    @property
    def min(self) -> Vec3: ...
    @property
    def max(self) -> Vec3: ...
    @property
    def face(self) -> int | None: ...
    @property
    def unknown(self) -> int: ...
    @property
    def plane(self) -> int: ...
    @property
    def left(self) -> int | None: ...
    @property
    def right(self) -> int | None: ...

class Walkmesh:
    """A room's WOK, a placeable's PWK or a door's DWK."""

    @staticmethod
    def parse(data: bytes) -> Walkmesh:
        """RE2501 to RE2504."""
    @staticmethod
    def read(path: StrPath) -> Walkmesh: ...
    @staticmethod
    def load(install: Install, name: str, module: str | None = None) -> Walkmesh:
        """A bare name is the room's `.wok`; name a `.pwk` or `.dwk` with its extension."""
    @property
    def kind(self) -> Literal["area", "object"]: ...
    @property
    def relative_points(self) -> tuple[Vec3, Vec3]: ...
    @property
    def absolute_points(self) -> tuple[Vec3, Vec3]: ...
    @property
    def position(self) -> Vec3: ...
    @property
    def unknown(self) -> int: ...
    @property
    def vertices(self) -> list[Vec3]: ...
    @property
    def vertex_bytes(self) -> bytes:
        """Twelve bytes a vertex, three little-endian floats."""
    @property
    def faces(self) -> list[WalkmeshFace]: ...
    @property
    def adjacency(self) -> list[tuple[Edge | None, Edge | None, Edge | None]]: ...
    @property
    def perimeter_edges(self) -> list[tuple[Edge, int | None]]:
        """`(edge, transition)`: the LYT room index the edge leads into, or None."""
    @property
    def perimeters(self) -> list[int]: ...
    @property
    def aabb(self) -> list[AabbNode]: ...
    def walkable_faces(self, surfacemat: TwoDa) -> list[int]:
        """RE2505 for a table without a `walk` column."""
    def set_material(self, face: int, material: int) -> int:
        """Returns the material the face had; `IndexError` past the last face."""
    def semantic_eq(self, other: Walkmesh) -> bool: ...
    def to_bytes(self) -> bytes:
        """RE2506."""

class MeshFace:
    @property
    def normal(self) -> Vec3: ...
    @property
    def distance(self) -> float: ...
    @property
    def surface(self) -> int: ...
    @property
    def adjacent(self) -> tuple[int, int, int]:
        """The faces across each edge, or -1."""
    @property
    def vertices(self) -> tuple[int, int, int]: ...

class Mesh:
    @property
    def texture(self) -> str | None: ...
    @property
    def lightmap(self) -> str | None: ...
    @property
    def texture_count(self) -> int: ...
    @property
    def lightmapped(self) -> bool: ...
    @property
    def render(self) -> bool: ...
    @property
    def shadow(self) -> bool: ...
    @property
    def transparency_hint(self) -> int: ...
    @property
    def diffuse(self) -> Vec3: ...
    @property
    def ambient(self) -> Vec3: ...
    @property
    def bounding_box(self) -> Box: ...
    @property
    def radius(self) -> float: ...
    @property
    def average(self) -> Vec3: ...
    @property
    def vertex_count(self) -> int: ...
    @property
    def mdx_flags(self) -> int: ...
    @property
    def stride(self) -> int: ...
    @property
    def vertex_buffer(self) -> bytes:
        """The MDX block as stored, `vertex_count * stride` bytes."""
    @property
    def offsets(self) -> dict[str, int]:
        """Each attribute read from `vertex_buffer` (`position`, `normal`, `uv1`, `uv2`) and its
        offset within the stride."""
    @property
    def positions(self) -> list[Vec3]: ...
    @property
    def normals(self) -> list[Vec3] | None: ...
    @property
    def uv1(self) -> list[tuple[float, float]] | None: ...
    @property
    def uv2(self) -> list[tuple[float, float]] | None: ...
    @property
    def faces(self) -> list[MeshFace]: ...
    @property
    def indices(self) -> list[int] | None: ...

class Skin:
    @property
    def bones(self) -> list[int]:
        """The bones' node numbers, by local bone index."""
    @property
    def weights(self) -> list[Quat]:
        """Each vertex's four weights."""
    @property
    def bone_indices(self) -> list[Quat]:
        """Each vertex's four local bone indices, -1 for none."""
    def bone_index(self, node: int) -> int | None: ...
    def qbone(self, node: int) -> Quat | None: ...
    def tbone(self, node: int) -> Vec3 | None: ...

class Light:
    @property
    def flare_radius(self) -> float: ...
    @property
    def flares(self) -> list[tuple[str, float, float, Vec3]]:
        """Each flare as `(texture, size, position, colour shift)`."""
    @property
    def priority(self) -> int: ...
    @property
    def ambient_only(self) -> bool: ...
    @property
    def dynamic_type(self) -> int:
        """The torlack document's dynamic type, by position alone."""
    @property
    def affect_dynamic(self) -> bool:
        """The torlack document's affect-dynamic flag, by position alone."""
    @property
    def shadow(self) -> bool: ...
    @property
    def generate_flare(self) -> bool:
        """Where the torlack document puts the generate-flare flag; false in every vanilla light."""
    @property
    def fading(self) -> bool:
        """The torlack document's fading flag, by position alone."""

class Emitter:
    @property
    def dead_space(self) -> float: ...
    @property
    def blast_radius(self) -> float: ...
    @property
    def blast_length(self) -> float: ...
    @property
    def lightning_words(self) -> tuple[int, int]:
        """The words at 12 and 16, set only on `Lightning` emitters; not established."""
    @property
    def x_grid(self) -> int: ...
    @property
    def y_grid(self) -> int: ...
    @property
    def space_type(self) -> int: ...
    @property
    def update(self) -> str:
        """`Single`, `Fountain`, `Explosion` or `Lightning` in vanilla."""
    @property
    def render(self) -> str: ...
    @property
    def blend(self) -> str: ...
    @property
    def texture(self) -> str | None: ...
    @property
    def chunk(self) -> str | None:
        """The model each particle is, if any."""
    @property
    def two_sided(self) -> bool: ...
    @property
    def looping(self) -> bool: ...
    @property
    def render_order(self) -> int: ...
    @property
    def name_187(self) -> str:
        """The name at 187 as stored: `NULL`, empty or `fx_depth` in vanilla; not established."""
    @property
    def flags(self) -> int: ...

class Saber:
    @property
    def positions(self) -> list[Vec3]: ...
    @property
    def uv(self) -> list[tuple[float, float]]: ...
    @property
    def normals(self) -> list[Vec3]: ...
    @property
    def words(self) -> tuple[int, int]:
        """The words at 12 and 16; not established."""

class Controller:
    @property
    def kind(self) -> int:
        """8 position, 20 orientation, 36 scale, ..."""
    @property
    def name(self) -> str | None:
        """The type's name on the kind of node that carries it (`radius` on a light, `birth_rate` on an
        emitter); `None` when not established."""
    @property
    def field(self) -> int: ...
    @property
    def columns(self) -> int: ...
    @property
    def is_bezier(self) -> bool: ...
    @property
    def is_compressed(self) -> bool: ...
    @property
    def keys(self) -> list[tuple[float, list[float]]]:
        """`(time, values)`: an orientation as `(w, x, y, z)`, compressed ones decoded; any other
        controller's row of floats as stored."""

class ModelNode:
    @property
    def name(self) -> str: ...
    @property
    def kind(
        self,
    ) -> Literal[
        "dummy",
        "light",
        "emitter",
        "reference",
        "trimesh",
        "skin",
        "animmesh",
        "dangly",
        "aabb",
        "saber",
        "other",
    ]: ...
    @property
    def flags(self) -> int: ...
    @property
    def number(self) -> int: ...
    @property
    def supernode(self) -> int: ...
    @property
    def parent(self) -> int | None:
        """The parent's index in the list the node is in."""
    @property
    def children(self) -> list[int]: ...
    @property
    def position(self) -> Vec3: ...
    @property
    def orientation(self) -> Quat: ...
    @property
    def mesh(self) -> Mesh | None: ...
    @property
    def skin(self) -> Skin | None: ...
    @property
    def light(self) -> Light | None: ...
    @property
    def emitter(self) -> Emitter | None: ...
    @property
    def saber(self) -> Saber | None: ...
    @property
    def controllers(self) -> list[Controller]: ...

class Animation:
    @property
    def name(self) -> str: ...
    @property
    def length(self) -> float: ...
    @property
    def transition(self) -> float: ...
    @property
    def root(self) -> str: ...
    @property
    def events(self) -> list[tuple[float, str]]: ...
    @property
    def nodes(self) -> list[ModelNode]: ...
    def node(self, name: str) -> ModelNode | None: ...

class Model:
    """A binary model: its MDL's header and node tree, and its MDX's vertices."""

    @staticmethod
    def parse(mdl: bytes, mdx: bytes = b"") -> Model:
        """RE2401 to RE2409."""
    @staticmethod
    def read(mdl: StrPath, mdx: StrPath | None = None) -> Model:
        """`mdx` defaults to the `.mdx` beside `mdl`."""
    @staticmethod
    def load(install: Install, name: str, module: str | None = None) -> Model:
        """The `.mdl` and `.mdx` the install loads by `name`; RE0404."""
    @property
    def name(self) -> str: ...
    @property
    def classification(
        self,
    ) -> Literal[
        "other", "effect", "tile", "character", "door", "lightsaber", "placeable", "unknown"
    ]: ...
    @property
    def subclassification(self) -> int: ...
    @property
    def fog(self) -> bool: ...
    @property
    def supermodel(self) -> str | None: ...
    @property
    def bounding_box(self) -> Box: ...
    @property
    def radius(self) -> float: ...
    @property
    def animation_scale(self) -> float: ...
    @property
    def nodes(self) -> list[ModelNode]:
        """Every node of the geometry, the root first."""
    @property
    def animations(self) -> list[Animation]: ...
    def node(self, name: str) -> ModelNode | None: ...
    def animation(self, name: str) -> Animation | None: ...
    def to_bytes(self) -> tuple[bytes, bytes]:
        """`(mdl, mdx)`, laid out as BioWare's compiler lays them out; RE2414 to RE2416."""

class NodeRef:
    """`E3` is `EntryList[3]`, `R0` `ReplyList[0]`."""

    def __init__(self, kind: Kind, index: int) -> None: ...
    @property
    def kind(self) -> Kind: ...
    @property
    def index(self) -> int: ...

class NodeKey:
    @staticmethod
    def parse(text: str) -> NodeKey:
        """`voice:RESREF`, `strref:N`, `text:TEXT`, `E3`, `R0`; RE1104 otherwise."""
    @staticmethod
    def voice(resref: str) -> NodeKey: ...
    @staticmethod
    def strref(strref: int) -> NodeKey: ...
    @staticmethod
    def text(text: str) -> NodeKey: ...
    @staticmethod
    def index(kind: Kind, index: int) -> NodeKey: ...
    @staticmethod
    def of(dialog: Dialog, at: NodeRef) -> NodeKey: ...
    def is_fragile(self) -> bool: ...
    def resolve(self, dialog: Dialog) -> NodeRef:
        """RE1102 when no node matches, RE1103 when several do."""

Key: TypeAlias = NodeKey | NodeRef | str
"""A node key: a `NodeKey`, a `NodeRef` (its index key), or the text `NodeKey.parse` takes."""

class Link:
    @property
    def target(self) -> NodeRef: ...
    @property
    def position(self) -> int: ...
    @property
    def condition(self) -> str | None: ...
    @property
    def is_child(self) -> bool: ...

class Node:
    @property
    def at(self) -> NodeRef: ...
    @property
    def speaker(self) -> str | None: ...
    @property
    def listener(self) -> str | None: ...
    @property
    def strref(self) -> int | None: ...
    @property
    def text(self) -> str | None:
        """The text it embeds."""
    @property
    def voice(self) -> str | None: ...
    @property
    def sound(self) -> str | None: ...
    @property
    def script(self) -> str | None: ...
    @property
    def delay(self) -> int | None: ...
    @property
    def animations(self) -> list[tuple[str, int]]: ...
    @property
    def camera_angle(self) -> int | None: ...
    @property
    def camera_id(self) -> int | None: ...
    @property
    def quest(self) -> str | None: ...
    @property
    def quest_entry(self) -> int | None: ...
    @property
    def comment(self) -> str | None: ...
    @property
    def links(self) -> list[Link]: ...

class Dialog:
    @staticmethod
    def parse(name: str, data: bytes) -> Dialog: ...
    @staticmethod
    def new(name: str, gff: Gff) -> Dialog: ...
    @property
    def name(self) -> str: ...
    @property
    def gff(self) -> Gff:
        """A copy."""
    def len(self, kind: Kind) -> int: ...
    def node(self, at: NodeRef) -> Node | None: ...
    def iter(self, kind: Kind) -> list[Node]: ...
    def starts(self) -> list[Link]: ...
    def to_bytes(self) -> bytes: ...

class Finding:
    """A reference of a dialogue the install cannot satisfy."""

    @property
    def code(self) -> str: ...
    @property
    def place(self) -> str: ...
    @property
    def key(self) -> NodeKey | None:
        """The node's stable key; None for the dialogue's end script."""
    @property
    def subject(self) -> str: ...
    @property
    def via(self) -> str | None: ...
    def message(self, dialog: str) -> str: ...
