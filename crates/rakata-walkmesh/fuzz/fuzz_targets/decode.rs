//! `parse` on arbitrary bytes: it returns a walkmesh or an error, never
//! panics, and what it returns answers its accessors; and
//! `SurfaceMaterials::parse` on the same bytes as a `surfacemat.2da`.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_walkmesh::{SurfaceMaterial, SurfaceMaterials};

fuzz_target!(|data: &[u8]| {
    if let Ok(w) = rakata_walkmesh::parse(data) {
        let _ = (w.vertices.to_vec(), w.vertices.iter().count());
        for l in w.perimeter_loops() {
            for e in l {
                let _ = (w.edge_points(e.edge), e.edge.corners());
            }
        }
        for a in w.adjacency.iter().flatten().flatten() {
            let _ = w.edge_points(*a);
        }
        let table = SurfaceMaterials {
            materials: (0..32)
                .map(|i| SurfaceMaterial {
                    label: String::new(),
                    walkable: i % 2 == 0,
                })
                .collect(),
        };
        let _ = w.walkable_faces(&table).count();
    }
    if let Ok(t) = SurfaceMaterials::parse(data) {
        for i in 0..=t.materials.len() as u32 {
            let _ = (t.get(i), t.is_walkable(i));
        }
    }
});
