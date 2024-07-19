use super::Instance;
use glam::Vec3;

pub fn load(vox_path: &str) -> (Vec<Instance>, Vec3) {
    log::info!("Loading {}", vox_path);

    let vox = dot_vox::load(vox_path).unwrap();
    let mut instances = vec![];
    let mut dimensions = Vec3::ZERO;

    for model in vox.models {
        instances.reserve(model.voxels.len());
        for voxel in model.voxels {
            let palette_index = voxel.i as usize;
            let color: [u8; 4] = vox
                .palette
                .get(palette_index)
                .or(dot_vox::DEFAULT_PALETTE.get(palette_index))
                .unwrap()
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

    (instances, dimensions)
}
