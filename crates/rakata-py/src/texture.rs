//! Textures (`rakata::texture`): a TPC with its mip levels and TXI, a
//! standalone TXI, and a TGA decoded to RGBA8; each read from bytes, a
//! path or the install, and written back as bytes.
//!
//! A TPC borrows its levels from the bytes it was read from, so the class
//! keeps those bytes and reads the TPC from them again when asked; only its
//! TXI, which a script may replace, is held apart. Pixel data is handed out
//! as `bytes`, the levels as stored.

use std::path::PathBuf;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use rakata::texture::{ImageType, Layout, Origin, PixelFormat, Rgba, Tga, Tpc, Txi, TxiEntry};

use crate::fail;
use crate::game::{PyInstall, load, read_file};

/// A texture's instructions: keywords, their values and the numeric rows
/// after them, in file order.
#[pyclass(name = "Txi", module = "rakata")]
#[derive(Clone)]
pub struct PyTxi {
    pub(crate) inner: Txi,
}

#[pymethods]
impl PyTxi {
    /// Read TXI text; any bytes read, as Latin-1.
    #[staticmethod]
    fn parse(data: &[u8]) -> Self {
        PyTxi {
            inner: Txi::parse(data),
        }
    }

    #[staticmethod]
    fn read(path: PathBuf) -> PyResult<Self> {
        Ok(Self::parse(&read_file(&path)?))
    }

    /// The `.txi` the install loads by this name (`"fnt_d16x16"`), in a
    /// module's capsules too when `module` names one. Raises `RE0404`.
    #[staticmethod]
    #[pyo3(signature = (install, name, module = None))]
    fn load(install: &PyInstall, name: &str, module: Option<&str>) -> PyResult<Self> {
        Ok(Self::parse(&load(
            &install.inner,
            name,
            Some("txi"),
            module,
        )?))
    }

    /// An empty TXI.
    #[staticmethod]
    fn new() -> Self {
        PyTxi {
            inner: Txi::default(),
        }
    }

    /// The numeric lines before any keyword.
    #[getter]
    fn preamble(&self) -> Vec<String> {
        self.inner.preamble.clone()
    }

    /// Every instruction as `(keyword, value, rows)`, in file order.
    #[getter]
    fn entries(&self) -> Vec<(String, String, Vec<String>)> {
        self.inner
            .entries
            .iter()
            .map(|e| (e.key.clone(), e.value.clone(), e.rows.clone()))
            .collect()
    }

    /// The value of the first instruction with this keyword, in any case.
    fn get(&self, key: &str) -> Option<String> {
        self.inner.value(key).map(str::to_owned)
    }

    /// The first word of that value as an integer.
    fn int(&self, key: &str) -> Option<i64> {
        self.inner.int(key)
    }

    /// Set the value of the first instruction with this keyword, in any
    /// case, or append the instruction; the value it had, or `None`.
    fn set(&mut self, key: &str, value: String) -> Option<String> {
        match self
            .inner
            .entries
            .iter_mut()
            .find(|e| e.key.eq_ignore_ascii_case(key))
        {
            Some(e) => Some(std::mem::replace(&mut e.value, value)),
            None => {
                self.inner.entries.push(TxiEntry {
                    key: key.to_string(),
                    value,
                    rows: Vec::new(),
                });
                None
            }
        }
    }

    /// The text; a TXI read and not changed writes back as it was read.
    /// Raises `RE2208` for text that would not read back as itself.
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.inner.to_bytes().map_err(fail)?))
    }

    fn __len__(&self) -> usize {
        self.inner.entries.len()
    }

    fn __repr__(&self) -> String {
        format!("Txi({} entries)", self.inner.entries.len())
    }
}

/// One mip level of a TPC image.
#[pyclass(name = "TpcLevel", module = "rakata", frozen, get_all)]
pub struct PyTpcLevel {
    width: u32,
    height: u32,
    /// Where the level begins in the TPC's bytes.
    offset: usize,
    /// The level's bytes as stored: DXT blocks, or grey, RGB or RGBA pixels.
    data: Py<PyBytes>,
}

#[pymethods]
impl PyTpcLevel {
    fn __repr__(&self) -> String {
        format!(
            "TpcLevel({}x{} at {})",
            self.width, self.height, self.offset
        )
    }
}

pub(crate) fn format_name(f: PixelFormat) -> &'static str {
    match f {
        PixelFormat::Grey => "grey",
        PixelFormat::Rgb => "rgb",
        PixelFormat::Rgba => "rgba",
        PixelFormat::Dxt1 => "dxt1",
        PixelFormat::Dxt5 => "dxt5",
    }
}

pub(crate) fn layout_name(l: Layout) -> &'static str {
    match l {
        Layout::Single => "single",
        Layout::Cube => "cube",
        Layout::Flipbook { .. } => "flipbook",
    }
}

/// A TPC texture: its header, its images and their mip levels with the
/// pixel format kept as stored, and its TXI.
#[pyclass(name = "Tpc", module = "rakata")]
pub struct PyTpc {
    data: Vec<u8>,
    txi: Option<Txi>,
}

impl PyTpc {
    fn new(data: Vec<u8>) -> PyResult<Self> {
        let txi = Tpc::parse(&data).map_err(fail)?.txi;
        Ok(PyTpc { data, txi })
    }

    /// The TPC as read; its `txi` is the one read, not this class's.
    fn tpc(&self) -> Tpc<'_> {
        Tpc::parse(&self.data).expect("the bytes read when the class was made")
    }

    fn decode(&self, level: usize, image: Option<usize>) -> PyResult<Rgba> {
        let t = self.tpc();
        match image {
            Some(i) => t.decode_image(i, level),
            None => t.decode_level(level),
        }
        .map_err(fail)
    }
}

#[pymethods]
impl PyTpc {
    /// Read a TPC. Raises `RE2201` and the format's other codes.
    #[staticmethod]
    fn parse(data: Vec<u8>) -> PyResult<Self> {
        Self::new(data)
    }

    #[staticmethod]
    fn read(path: PathBuf) -> PyResult<Self> {
        Self::new(read_file(&path)?)
    }

    /// The `.tpc` the install loads by this name, through the texture pack
    /// it searches. Raises `RE0404`.
    #[staticmethod]
    #[pyo3(signature = (install, name, module = None))]
    fn load(install: &PyInstall, name: &str, module: Option<&str>) -> PyResult<Self> {
        Self::new(load(&install.inner, name, Some("tpc"), module)?)
    }

    /// The header's width.
    #[getter]
    fn width(&self) -> u16 {
        self.tpc().width
    }

    /// The header's height; for a cube map, six faces' worth.
    #[getter]
    fn height(&self) -> u16 {
        self.tpc().height
    }

    /// `"grey"`, `"rgb"`, `"rgba"`, `"dxt1"` or `"dxt5"`.
    #[getter]
    fn format(&self) -> &'static str {
        format_name(self.tpc().format)
    }

    /// The header's mip count; an animation sheet says 1.
    #[getter]
    fn mip_count(&self) -> u8 {
        self.tpc().mip_count
    }

    #[getter]
    fn data_size(&self) -> u32 {
        self.tpc().data_size
    }

    #[getter]
    fn alpha_mean(&self) -> f32 {
        self.tpc().alpha_mean
    }

    /// `"single"`, `"cube"` or `"flipbook"`.
    #[getter]
    fn layout(&self) -> &'static str {
        layout_name(self.tpc().layout)
    }

    /// A flipbook's frames across and down, from its TXI; `None` otherwise.
    #[getter]
    fn frames(&self) -> Option<(u32, u32)> {
        match self.tpc().layout {
            Layout::Flipbook { numx, numy } => Some((numx, numy)),
            _ => None,
        }
    }

    #[getter]
    fn has_alpha(&self) -> bool {
        self.tpc().has_alpha()
    }

    /// The size of one image (a face or a frame) at level 0.
    #[getter]
    fn image_size(&self) -> (u32, u32) {
        self.tpc().image_size()
    }

    /// The number of levels each image has.
    #[getter]
    fn level_count(&self) -> usize {
        self.tpc().level_count()
    }

    /// Each image (one, six cube faces, or the frames in row order) as its
    /// levels, largest first.
    #[getter]
    fn images(&self, py: Python<'_>) -> PyResult<Vec<Vec<Py<PyTpcLevel>>>> {
        self.tpc()
            .images
            .iter()
            .map(|img| {
                img.levels
                    .iter()
                    .map(|l| {
                        Py::new(
                            py,
                            PyTpcLevel {
                                width: l.width,
                                height: l.height,
                                offset: l.offset,
                                data: PyBytes::new(py, l.data).unbind(),
                            },
                        )
                    })
                    .collect()
            })
            .collect()
    }

    /// The pixel data whole: every image's levels, back to back.
    #[getter]
    fn pixels<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, self.tpc().pixels)
    }

    /// The TXI, a copy, or `None` when the TPC carries none. Setting it
    /// replaces the TXI `to_bytes` writes.
    #[getter]
    fn get_txi(&self) -> Option<PyTxi> {
        self.txi.clone().map(|inner| PyTxi { inner })
    }

    #[setter]
    fn set_txi(&mut self, txi: Option<PyRef<'_, PyTxi>>) {
        self.txi = txi.map(|t| t.inner.clone());
    }

    /// Level `level` as RGBA8, rows top first: the whole texture laid out
    /// at the header's size (a cube's faces stacked top to bottom, a
    /// sheet's frames across then down), or the one image `image` names.
    /// Raises `RE2210` and `RE2211`.
    #[pyo3(signature = (level = 0, image = None))]
    fn rgba<'py>(
        &self,
        py: Python<'py>,
        level: usize,
        image: Option<usize>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.decode(level, image)?.pixels))
    }

    /// What `rgba` gives, as a PNG file. Raises `RE2210`, `RE2211` and
    /// `RE2212`.
    #[pyo3(signature = (level = 0, image = None))]
    fn to_png<'py>(
        &self,
        py: Python<'py>,
        level: usize,
        image: Option<usize>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let png = self.decode(level, image)?.to_png().map_err(fail)?;
        Ok(PyBytes::new(py, &png))
    }

    /// The TPC: the header and pixels as read, and the TXI. Raises `RE2207`
    /// and `RE2208`.
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let mut t = self.tpc();
        t.txi = self.txi.clone();
        Ok(PyBytes::new(py, &t.to_bytes().map_err(fail)?))
    }

    fn __repr__(&self) -> String {
        let t = self.tpc();
        format!(
            "Tpc({}x{} {}, {} levels)",
            t.width,
            t.height,
            format_name(t.format),
            t.level_count()
        )
    }
}

fn image_type_name(t: ImageType) -> &'static str {
    match t {
        ImageType::TrueColor => "truecolor",
        ImageType::Grey => "grey",
        ImageType::TrueColorRle => "truecolor_rle",
        ImageType::GreyRle => "grey_rle",
    }
}

fn origin_name(o: Origin) -> &'static str {
    match o {
        Origin::BottomLeft => "bottom_left",
        Origin::BottomRight => "bottom_right",
        Origin::TopLeft => "top_left",
        Origin::TopRight => "top_right",
    }
}

/// A TARGA image decoded to RGBA8, rows top first.
#[pyclass(name = "Tga", module = "rakata")]
#[derive(Clone)]
pub struct PyTga {
    inner: Tga,
}

#[pymethods]
impl PyTga {
    /// Read a TGA. Raises `RE2204` and the format's other codes.
    #[staticmethod]
    fn parse(data: &[u8]) -> PyResult<Self> {
        Ok(PyTga {
            inner: Tga::parse(data).map_err(fail)?,
        })
    }

    #[staticmethod]
    fn read(path: PathBuf) -> PyResult<Self> {
        Self::parse(&read_file(&path)?)
    }

    /// The `.tga` the install loads by this name. Raises `RE0404`.
    #[staticmethod]
    #[pyo3(signature = (install, name, module = None))]
    fn load(install: &PyInstall, name: &str, module: Option<&str>) -> PyResult<Self> {
        Self::parse(&load(&install.inner, name, Some("tga"), module)?)
    }

    #[getter]
    fn width(&self) -> u16 {
        self.inner.width
    }

    #[getter]
    fn height(&self) -> u16 {
        self.inner.height
    }

    /// `"truecolor"`, `"grey"`, `"truecolor_rle"` or `"grey_rle"`.
    #[getter]
    fn image_type(&self) -> &'static str {
        image_type_name(self.inner.image_type)
    }

    /// The stored bits per pixel.
    #[getter]
    fn bits(&self) -> u8 {
        self.inner.bits
    }

    #[getter]
    fn alpha_bits(&self) -> u8 {
        self.inner.alpha_bits
    }

    /// The stored first pixel's corner: `"bottom_left"`, `"bottom_right"`,
    /// `"top_left"` or `"top_right"`.
    #[getter]
    fn origin(&self) -> &'static str {
        origin_name(self.inner.origin)
    }

    /// The pixels, four bytes each (R, G, B, A), rows top first. Setting
    /// them takes `width * height * 4` bytes, which `to_bytes` encodes back
    /// to the stored depth and order.
    #[getter]
    fn get_rgba<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.rgba)
    }

    #[setter]
    fn set_rgba(&mut self, rgba: Vec<u8>) -> PyResult<()> {
        let want = usize::from(self.inner.width) * usize::from(self.inner.height) * 4;
        if rgba.len() != want {
            return Err(PyValueError::new_err(format!(
                "a {}x{} image's pixels are {want} bytes, not {}",
                self.inner.width,
                self.inner.height,
                rgba.len()
            )));
        }
        self.inner.rgba = rgba;
        Ok(())
    }

    /// The pixels as a PNG file. Raises `RE2212`.
    fn to_png<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.inner.to_png().map_err(fail)?))
    }

    /// The TGA in its stored depth, order and packing. Raises `RE2209`.
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, &self.inner.to_bytes().map_err(fail)?))
    }

    fn __repr__(&self) -> String {
        format!(
            "Tga({}x{} {})",
            self.inner.width,
            self.inner.height,
            image_type_name(self.inner.image_type)
        )
    }
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTxi>()?;
    m.add_class::<PyTpc>()?;
    m.add_class::<PyTpcLevel>()?;
    m.add_class::<PyTga>()?;
    Ok(())
}
