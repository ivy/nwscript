//! The layout facts of a model as stored, beside the layout the writer
//! gives the same model: how the bytes another tool wrote depart from
//! BioWare's, for a model the writer does not reproduce.

use crate::error::Result;
use crate::file::{Region, Trace, parse_traced};

/// How a model's MDL and MDX are laid out, measured against the layout
/// [`ModelFile::to_bytes`](crate::ModelFile::to_bytes) gives them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// Whether the model data's structures lie in the writer's order.
    pub in_order: bool,
    /// The first structure, in the writer's order, that lies out of it.
    pub first_out_of_order: Option<&'static str>,
    /// The order of the first mesh's face array, index counts, positions
    /// and index offsets, by where they lie: BioWare's is faces, counts,
    /// positions, offsets.
    pub mesh_order: Option<[&'static str; 4]>,
    /// Whether a node's controllers lie after its children (BioWare's
    /// order) or before them, in the first node with both.
    pub controllers_after_children: Option<bool>,
    /// Bytes of the model data no structure covers.
    pub unreferenced: usize,
    /// Bytes two structures share.
    pub shared: usize,
    /// Nodes whose parent offset is not their parent's.
    pub odd_parents: usize,
    /// Arrays whose allocated count differs from their used count.
    pub odd_counts: usize,
    /// Empty arrays at zero that the writer points at the next
    /// structure, and the first of them in the writer's order.
    pub empty_at_zero: (usize, Option<&'static str>),
    /// Empty arrays pointing somewhere that the writer leaves at zero,
    /// and the first of them in the writer's order.
    pub empty_pointing: (usize, Option<&'static str>),
    /// The order of the MDX blocks.
    pub mdx_order: BlockOrder,
    /// The bytes between MDX blocks.
    pub mdx_padding: Padding,
    /// Bytes after the MDX's last block.
    pub mdx_trailing: usize,
}

/// The order of a model's MDX blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockOrder {
    /// BioWare's: every mesh but the skins by node number, then the skins
    /// by node number.
    BioWare,
    /// By node number, the skins among the rest.
    Number,
    /// The order the model data's tree reaches the meshes in.
    Tree,
    /// Some other order.
    Other,
}

/// The bytes between consecutive MDX blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Padding {
    /// BioWare's: as many bytes as the MDX's length is past a multiple of
    /// 16 at the end of the block before.
    BioWare,
    /// None: each block follows the one before.
    None,
    /// To the next multiple of 16.
    Aligned,
    /// Some other rule, or several.
    Other,
}

/// The layout facts of a model's MDL and MDX.
pub fn layout(mdl: &[u8], mdx: &[u8]) -> Result<Layout> {
    let (model, read) = parse_traced(mdl, mdx)?;
    let (w_mdl, w_mdx) = model.to_bytes()?;
    let (_, written) = parse_traced(&w_mdl, &w_mdx)?;
    Ok(compare(&read, &written))
}

fn compare(read: &Trace, written: &Trace) -> Layout {
    let n = read.regions.len().min(written.regions.len());
    let (a, b) = (&read.regions[..n], &written.regions[..n]);

    // The structures with bytes, in the order each layout puts them.
    let full: Vec<usize> = (0..n).filter(|&i| a[i].len > 0).collect();
    let mut by_read = full.clone();
    by_read.sort_by_key(|&i| (a[i].at, i));
    let mut by_written = full;
    by_written.sort_by_key(|&i| (b[i].at, i));
    let first_out_of_order = by_read
        .iter()
        .zip(&by_written)
        .find(|(x, y)| x != y)
        .map(|(_, &y)| b[y].what);

    let mesh_order = mesh_order(a);
    let controllers_after_children = controllers_after_children(a);

    // Coverage of the model data as read.
    let mut spans: Vec<&Region> = a.iter().filter(|r| r.len > 0).collect();
    spans.sort_by_key(|r| r.at);
    let (mut covered_to, mut unreferenced, mut shared) = (0usize, 0usize, 0usize);
    for r in spans {
        let end = r.at + r.len;
        if r.at > covered_to {
            unreferenced += r.at - covered_to;
        } else {
            shared += covered_to.min(end) - r.at;
        }
        covered_to = covered_to.max(end);
    }
    unreferenced += read.data_len.saturating_sub(covered_to);

    let empties = |f: &dyn Fn(&Region, &Region) -> bool| -> (usize, Option<&'static str>) {
        let mut v: Vec<usize> = (0..n)
            .filter(|&i| a[i].len == 0 && f(&a[i], &b[i]))
            .collect();
        v.sort_by_key(|&i| (b[i].at, i));
        (v.len(), v.first().map(|&i| b[i].what))
    };
    let empty_at_zero = empties(&|x, y| x.at == 0 && y.at != 0);
    let empty_pointing = empties(&|x, y| x.at != 0 && y.at == 0);

    let blocks = |t: &Trace| -> Vec<usize> {
        let mut v: Vec<usize> = (0..t.blocks.len()).collect();
        v.sort_by_key(|&i| t.blocks[i].0);
        v
    };
    let (ra, rb) = (blocks(read), blocks(written));
    let mut by_number: Vec<usize> = (0..read.blocks.len()).collect();
    by_number.sort_by_key(|&i| (read.block_numbers[i], i));
    let tree: Vec<usize> = (0..read.blocks.len()).collect();
    let mdx_order = if ra == rb {
        BlockOrder::BioWare
    } else if ra == by_number {
        BlockOrder::Number
    } else if ra == tree {
        BlockOrder::Tree
    } else {
        BlockOrder::Other
    };
    let mut gaps = ra
        .windows(2)
        .map(|w| {
            let end = read.blocks[w[0]].1;
            (end, read.blocks[w[1]].0 as isize - end as isize)
        })
        .peekable();
    let mdx_padding = if gaps.peek().is_none() {
        Padding::BioWare
    } else {
        let gaps: Vec<(usize, isize)> = gaps.collect();
        let all = |f: &dyn Fn(usize) -> usize| gaps.iter().all(|&(e, g)| g == f(e) as isize);
        if all(&|e| e % 16) {
            Padding::BioWare
        } else if all(&|_| 0) {
            Padding::None
        } else if all(&|e| (16 - e % 16) % 16) {
            Padding::Aligned
        } else {
            Padding::Other
        }
    };
    let last_end = read.blocks.iter().map(|b| b.1).max().unwrap_or(0);

    Layout {
        in_order: first_out_of_order.is_none(),
        first_out_of_order,
        mesh_order,
        controllers_after_children,
        unreferenced,
        shared,
        odd_parents: read.odd_parents,
        odd_counts: read.odd_counts,
        empty_at_zero,
        empty_pointing,
        mdx_order,
        mdx_padding,
        mdx_trailing: read.mdx_len.saturating_sub(last_end),
    }
}

/// The first mesh's four arrays with bytes, ordered by where they lie;
/// a mesh lacking one (a saber, a mesh without vertices) is passed over.
fn mesh_order(a: &[Region]) -> Option<[&'static str; 4]> {
    let names = [
        ("face array", "faces"),
        ("index count array", "counts"),
        ("vertex array", "positions"),
        ("index offset array", "offsets"),
    ];
    for (start, _) in a
        .iter()
        .enumerate()
        .filter(|(_, r)| r.what == "mesh header")
    {
        let mut found: Vec<(usize, &'static str)> = Vec::new();
        for r in &a[start + 1..] {
            if r.what == "node header" || r.what == "mesh header" {
                break;
            }
            if let Some(&(_, short)) = names.iter().find(|(w, _)| *w == r.what)
                && r.len > 0
                && !found.iter().any(|f| f.1 == short)
            {
                found.push((r.at, short));
            }
        }
        if let [x, y, z, w] = found[..] {
            let mut v = [x, y, z, w];
            v.sort();
            return Some(v.map(|f| f.1));
        }
    }
    None
}

/// Whether the first node with controllers and children has its
/// controllers after its first child.
fn controllers_after_children(a: &[Region]) -> Option<bool> {
    for (i, r) in a.iter().enumerate() {
        if r.what != "node header" {
            continue;
        }
        // A node reads its body, its controllers, their data, its child
        // array, then its first child's header.
        let mut controllers = None;
        let mut children = None;
        for (j, s) in a.iter().enumerate().skip(i + 1) {
            match s.what {
                "node header" => break,
                "controller array" if controllers.is_none() => controllers = Some(s),
                "child array" => {
                    children = Some(j);
                    break;
                }
                _ => {}
            }
        }
        if let (Some(c), Some(j)) = (controllers, children)
            && c.len > 0
            && a[j].len > 0
            && let Some(child) = a.get(j + 1).filter(|n| n.what == "node header")
        {
            return Some(c.at > child.at);
        }
    }
    None
}

impl Layout {
    /// The MDL's facts as one line: `mdl: ` and the order of its
    /// structures, its unreferenced and shared bytes, its empty arrays
    /// placed otherwise than the writer places them, its parent offsets and
    /// array counts, and the MDX bytes its header counts past the MDX's
    /// last block.
    pub fn mdl_line(&self) -> String {
        let order = match self.first_out_of_order {
            None => "structures in BioWare's order".to_owned(),
            Some(w) => format!("structures in another order from the {w}"),
        };
        let mesh = self.mesh_order.map_or_else(
            || "no mesh".to_owned(),
            |m| format!("mesh arrays {}", m.join(", ")),
        );
        let controllers = match self.controllers_after_children {
            None => "no node with controllers and children",
            Some(true) => "controllers after children",
            Some(false) => "controllers before children",
        };
        let empty = |(n, first): (usize, Option<&str>), what: &str| match first {
            None => format!("no empty array {what}"),
            Some(w) => format!("{n} empty arrays {what}, first the {w}"),
        };
        let empty = format!(
            "{}; {}",
            empty(self.empty_at_zero, "at zero that BioWare points on"),
            empty(
                self.empty_pointing,
                "pointing on that BioWare leaves at zero"
            )
        );
        let mdx = match self.mdx_trailing {
            0 => "the MDX length its blocks give".to_owned(),
            n => format!("an MDX length {n} bytes past its last block"),
        };
        format!(
            "mdl: {order}; {mesh}; {controllers}; {} unreferenced bytes; {} shared bytes; {empty}; {} parent offsets wrong; {} allocated counts unlike used; {mdx}",
            self.unreferenced, self.shared, self.odd_parents, self.odd_counts
        )
    }

    /// The MDX's facts as one line: `mdx: ` and the order of its blocks,
    /// the padding between them and the bytes after the last.
    pub fn mdx_line(&self) -> String {
        let order = match self.mdx_order {
            BlockOrder::BioWare => "blocks in BioWare's order",
            BlockOrder::Number => "blocks by node number, the skins among the rest",
            BlockOrder::Tree => "blocks in the tree's order",
            BlockOrder::Other => "blocks in another order",
        };
        let padding = match self.mdx_padding {
            Padding::BioWare => "BioWare's padding",
            Padding::None => "no padding",
            Padding::Aligned => "padding to 16",
            Padding::Other => "other padding",
        };
        let end = match self.mdx_trailing {
            0 => "ends at its last block".to_owned(),
            n => format!("{n} bytes after its last block"),
        };
        format!("mdx: {order}; {padding}; {end}")
    }
}
