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

    let bytes = std::fs::read(vox_path)
        .map_err(|e| LoadError::Io(e.to_string()))?;
    let vox = dot_vox::load_bytes(&bytes)
        .map_err(|e| LoadError::Parse(e.to_string()))?;

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
