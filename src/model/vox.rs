use super::Instance;
use glam::Vec3;

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

pub fn load(vox_path: &str) -> Result<(Vec<Instance>, Vec3), LoadError> {
    log::info!("Loading {}", vox_path);

    let bytes = std::fs::read(vox_path)
        .map_err(|e| LoadError::Io(e.to_string()))?;
    let vox = dot_vox::load_bytes(&bytes)
        .map_err(|e| LoadError::Parse(e.to_string()))?;

    if vox.models.is_empty() {
        return Err(LoadError::Empty);
    }

    let mut instances = vec![];
    let mut dimensions = Vec3::ZERO;

    for model in &vox.models {
        instances.reserve(model.voxels.len());
        for voxel in &model.voxels {
            let palette_index = voxel.i as usize;
            let color: [u8; 4] = vox
                .palette
                .get(palette_index)
                .or_else(|| dot_vox::DEFAULT_PALETTE.get(palette_index))
                .ok_or_else(|| LoadError::Parse(format!("palette index {palette_index} out of range")))?
                .into();
            instances.push(Instance {
                position: [voxel.x, voxel.y, voxel.z, 0],
                color,
            });
        }
        dimensions = dimensions.max(Vec3::new(
            model.size.x as f32,
            model.size.y as f32,
            model.size.z as f32,
        ));
    }

    if instances.is_empty() {
        return Err(LoadError::Empty);
    }

    Ok((instances, dimensions))
}
