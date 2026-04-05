use super::{Mesh, Vertex};
use glam::Vec3;
use std::collections::HashSet;

#[derive(Debug)]
pub enum LoadError {
    Io(String),
    Parse(String),
    Empty,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(msg) => write!(f, "could not open file: {msg}"),
            LoadError::Parse(msg) => write!(f, "malformed .vox file: {msg}"),
            LoadError::Empty => write!(f, "file contains no voxel data"),
        }
    }
}

// Each face: (neighbor offset to check, normal, 4 quad vertices relative to voxel origin)
const FACES: [([i32; 3], [f32; 4], [[f32; 4]; 4]); 6] = [
    // top (+Z)
    ([0, 0, 1], [0., 0., 1., 0.], [
        [0., 0., 1., 1.], [1., 0., 1., 1.], [1., 1., 1., 1.], [0., 1., 1., 1.],
    ]),
    // bottom (-Z)
    ([0, 0, -1], [0., 0., -1., 0.], [
        [0., 1., 0., 1.], [1., 1., 0., 1.], [1., 0., 0., 1.], [0., 0., 0., 1.],
    ]),
    // right (+X)
    ([1, 0, 0], [1., 0., 0., 0.], [
        [1., 0., 0., 1.], [1., 1., 0., 1.], [1., 1., 1., 1.], [1., 0., 1., 1.],
    ]),
    // left (-X)
    ([-1, 0, 0], [-1., 0., 0., 0.], [
        [0., 0., 1., 1.], [0., 1., 1., 1.], [0., 1., 0., 1.], [0., 0., 0., 1.],
    ]),
    // front (+Y)
    ([0, 1, 0], [0., 1., 0., 0.], [
        [1., 1., 0., 1.], [0., 1., 0., 1.], [0., 1., 1., 1.], [1., 1., 1., 1.],
    ]),
    // back (-Y)
    ([0, -1, 0], [0., -1., 0., 0.], [
        [1., 0., 1., 1.], [0., 0., 1., 1.], [0., 0., 0., 1.], [1., 0., 0., 1.],
    ]),
];

pub fn load(vox_path: &str) -> Result<(Mesh, Vec3), LoadError> {
    log::info!("Loading {}", vox_path);
    let bytes = std::fs::read(vox_path).map_err(|e| LoadError::Io(e.to_string()))?;
    let vox = dot_vox::load_bytes(&bytes).map_err(|e| LoadError::Parse(e.to_string()))?;
    build_mesh(&vox)
}

pub(crate) fn build_mesh(vox: &dot_vox::DotVoxData) -> Result<(Mesh, Vec3), LoadError> {
    if vox.models.is_empty() {
        return Err(LoadError::Empty);
    }

    // Collect all occupied positions for neighbour lookup.
    let occupied: HashSet<(i32, i32, i32)> = vox
        .models
        .iter()
        .flat_map(|m| m.voxels.iter().map(|v| (v.x as i32, v.y as i32, v.z as i32)))
        .collect();

    let total_voxels: usize = vox.models.iter().map(|m| m.voxels.len()).sum();
    let total_faces_before = total_voxels * 6;

    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut dimensions = Vec3::ZERO;

    for model in &vox.models {
        for voxel in &model.voxels {
            let palette_index = voxel.i as usize;
            let color: [u8; 4] = vox
                .palette
                .get(palette_index)
                .or_else(|| dot_vox::DEFAULT_PALETTE.get(palette_index))
                .ok_or_else(|| {
                    LoadError::Parse(format!("palette index {palette_index} out of range"))
                })?
                .into();

            let (vx, vy, vz) = (voxel.x as i32, voxel.y as i32, voxel.z as i32);

            for (neighbor_offset, normal, quad_verts) in &FACES {
                let neighbor = (
                    vx + neighbor_offset[0],
                    vy + neighbor_offset[1],
                    vz + neighbor_offset[2],
                );
                if occupied.contains(&neighbor) {
                    continue; // face is hidden — skip
                }

                let base = vertices.len() as u32;
                for rel in quad_verts {
                    vertices.push(Vertex {
                        position: [
                            rel[0] + voxel.x as f32,
                            rel[1] + voxel.y as f32,
                            rel[2] + voxel.z as f32,
                            rel[3],
                        ],
                        normal: *normal,
                        color,
                    });
                }
                // CCW quad: two triangles
                indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
            }
        }

        dimensions = dimensions.max(Vec3::new(
            model.size.x as f32,
            model.size.y as f32,
            model.size.z as f32,
        ));
    }

    if vertices.is_empty() {
        return Err(LoadError::Empty);
    }

    let visible_faces = indices.len() / 6;
    let total_triangles_before = total_faces_before * 2;
    let visible_triangles = indices.len() / 3;
    log::info!(
        "Face culling: {total_faces_before} → {visible_faces} faces, {} → {visible_triangles} triangles ({:.0}% reduction)",
        total_triangles_before,
        (1.0 - visible_faces as f64 / total_faces_before as f64) * 100.0,
    );

    Ok((Mesh { vertices, indices }, dimensions))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dot_vox::{DotVoxData, Model, Size, Voxel};

    fn vox_with_voxels(voxels: Vec<Voxel>) -> DotVoxData {
        DotVoxData {
            version: 150,
            models: vec![Model {
                size: Size { x: 10, y: 10, z: 10 },
                voxels,
            }],
            palette: dot_vox::DEFAULT_PALETTE.to_vec(),
            materials: Default::default(),
            scenes: vec![],
            layers: vec![],
            index_map: dot_vox::DEFAULT_INDEX_MAP.to_vec(),
        }
    }

    #[test]
    fn load_valid_file() {
        assert!(load("assets/snow.vox").is_ok());
    }

    #[test]
    fn load_invalid_path() {
        let err = load("nonexistent.vox").unwrap_err();
        assert!(matches!(err, LoadError::Io(_)));
    }

    #[test]
    fn load_malformed_bytes() {
        let tmp = std::env::temp_dir().join("malformed_test.vox");
        std::fs::write(&tmp, b"not a vox file").unwrap();
        let err = load(tmp.to_str().unwrap()).unwrap_err();
        assert!(matches!(err, LoadError::Parse(_)));
    }

    #[test]
    fn build_mesh_no_models() {
        let vox = DotVoxData {
            version: 150,
            models: vec![],
            palette: vec![],
            materials: Default::default(),
            scenes: vec![],
            layers: vec![],
            index_map: dot_vox::DEFAULT_INDEX_MAP.to_vec(),
        };
        assert!(matches!(build_mesh(&vox), Err(LoadError::Empty)));
    }

    #[test]
    fn build_mesh_empty_model() {
        let vox = vox_with_voxels(vec![]);
        assert!(matches!(build_mesh(&vox), Err(LoadError::Empty)));
    }

    #[test]
    fn single_isolated_voxel_has_6_faces() {
        let vox = vox_with_voxels(vec![Voxel { x: 0, y: 0, z: 0, i: 1 }]);
        let (mesh, _) = build_mesh(&vox).unwrap();
        // 6 faces × 2 triangles × 3 indices = 36
        assert_eq!(mesh.indices.len(), 36);
        // 6 faces × 4 vertices per quad = 24
        assert_eq!(mesh.vertices.len(), 24);
    }

    #[test]
    fn two_adjacent_voxels_share_two_hidden_faces() {
        // Voxels at (0,0,0) and (1,0,0) share one face each → 10 visible faces total
        let vox = vox_with_voxels(vec![
            Voxel { x: 0, y: 0, z: 0, i: 1 },
            Voxel { x: 1, y: 0, z: 0, i: 1 },
        ]);
        let (mesh, _) = build_mesh(&vox).unwrap();
        // 10 faces × 6 indices = 60
        assert_eq!(mesh.indices.len(), 60);
    }

    #[test]
    fn fully_enclosed_voxel_emits_no_faces() {
        // Center voxel surrounded on all 6 sides — all faces culled
        let vox = vox_with_voxels(vec![
            Voxel { x: 1, y: 1, z: 1, i: 1 }, // center
            Voxel { x: 2, y: 1, z: 1, i: 1 }, // +X
            Voxel { x: 0, y: 1, z: 1, i: 1 }, // -X
            Voxel { x: 1, y: 2, z: 1, i: 1 }, // +Y
            Voxel { x: 1, y: 0, z: 1, i: 1 }, // -Y
            Voxel { x: 1, y: 1, z: 2, i: 1 }, // +Z
            Voxel { x: 1, y: 1, z: 0, i: 1 }, // -Z
        ]);
        let (mesh, _) = build_mesh(&vox).unwrap();
        // Outer 6 voxels each have 5 visible faces (one shared with center) = 30 faces
        assert_eq!(mesh.indices.len() / 6, 30);
    }
}
