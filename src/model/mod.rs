pub mod vox;

use bytemuck::{Pod, Zeroable};

// Vertex for the voxel mesh — color baked in, no instancing.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub color: [u8; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x4,
        1 => Float32x4,
        2 => Unorm8x4,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// Face-culled voxel mesh returned by the loader.
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

// Simple vertex for the light-cube visualization (position only).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LightVertex {
    pub position: [f32; 4],
}

impl LightVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x4];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub const LIGHT_CUBE_VERTICES: &[LightVertex] = &[
    LightVertex { position: [0., 0., 1., 1.] },
    LightVertex { position: [1., 0., 1., 1.] },
    LightVertex { position: [1., 1., 1., 1.] },
    LightVertex { position: [0., 1., 1., 1.] },
    LightVertex { position: [0., 1., 0., 1.] },
    LightVertex { position: [1., 1., 0., 1.] },
    LightVertex { position: [1., 0., 0., 1.] },
    LightVertex { position: [0., 0., 0., 1.] },
    LightVertex { position: [1., 0., 0., 1.] },
    LightVertex { position: [1., 1., 0., 1.] },
    LightVertex { position: [1., 1., 1., 1.] },
    LightVertex { position: [1., 0., 1., 1.] },
    LightVertex { position: [0., 0., 1., 1.] },
    LightVertex { position: [0., 1., 1., 1.] },
    LightVertex { position: [0., 1., 0., 1.] },
    LightVertex { position: [0., 0., 0., 1.] },
    LightVertex { position: [1., 1., 0., 1.] },
    LightVertex { position: [0., 1., 0., 1.] },
    LightVertex { position: [0., 1., 1., 1.] },
    LightVertex { position: [1., 1., 1., 1.] },
    LightVertex { position: [1., 0., 1., 1.] },
    LightVertex { position: [0., 0., 1., 1.] },
    LightVertex { position: [0., 0., 0., 1.] },
    LightVertex { position: [1., 0., 0., 1.] },
];

pub const LIGHT_CUBE_INDICES: &[u16] = &[
    0, 1, 2, 2, 3, 0,
    4, 5, 6, 6, 7, 4,
    8, 9, 10, 10, 11, 8,
    12, 13, 14, 14, 15, 12,
    16, 17, 18, 18, 19, 16,
    20, 21, 22, 22, 23, 20,
];
