pub mod vox;

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    // uses one byte for padding
    position: [f32; 4],
    normal: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub const VERTICES: &[Vertex] = &[
    // top (0., 0., 1.)
    Vertex {
        position: [0., 0., 1., 1.],
        normal: [0., 0., 1., 0.],
    },
    Vertex {
        position: [1., 0., 1., 1.],
        normal: [0., 0., 1., 0.],
    },
    Vertex {
        position: [1., 1., 1., 1.],
        normal: [0., 0., 1., 0.],
    },
    Vertex {
        position: [0., 1., 1., 1.],
        normal: [0., 0., 1., 0.],
    },
    // bottom (0., 0., -1.)
    Vertex {
        position: [0., 1., 0., 1.],
        normal: [0., 0., -1., 0.],
    },
    Vertex {
        position: [1., 1., 0., 1.],
        normal: [0., 0., -1., 0.],
    },
    Vertex {
        position: [1., 0., 0., 1.],
        normal: [0., 0., -1., 0.],
    },
    Vertex {
        position: [0., 0., 0., 1.],
        normal: [0., 0., -1., 0.],
    },
    // right (1., 0., 0.)
    Vertex {
        position: [1., 0., 0., 1.],
        normal: [1., 0., 0., 0.],
    },
    Vertex {
        position: [1., 1., 0., 1.],
        normal: [1., 0., 0., 0.],
    },
    Vertex {
        position: [1., 1., 1., 1.],
        normal: [1., 0., 0., 0.],
    },
    Vertex {
        position: [1., 0., 1., 1.],
        normal: [1., 0., 0., 0.],
    },
    // left (-1., 0., 0.)
    Vertex {
        position: [0., 0., 1., 1.],
        normal: [-1., 0., 0., 0.],
    },
    Vertex {
        position: [0., 1., 1., 1.],
        normal: [-1., 0., 0., 0.],
    },
    Vertex {
        position: [0., 1., 0., 1.],
        normal: [-1., 0., 0., 0.],
    },
    Vertex {
        position: [0., 0., 0., 1.],
        normal: [-1., 0., 0., 0.],
    },
    // front (0., 1., 0.)
    Vertex {
        position: [1., 1., 0., 1.],
        normal: [0., 1., 0., 0.],
    },
    Vertex {
        position: [0., 1., 0., 1.],
        normal: [0., 1., 0., 0.],
    },
    Vertex {
        position: [0., 1., 1., 1.],
        normal: [0., 1., 0., 0.],
    },
    Vertex {
        position: [1., 1., 1., 1.],
        normal: [0., 1., 0., 0.],
    },
    // back (0., -1., 0.)
    Vertex {
        position: [1., 0., 1., 1.],
        normal: [0., -1., 0., 0.],
    },
    Vertex {
        position: [0., 0., 1., 1.],
        normal: [0., -1., 0., 0.],
    },
    Vertex {
        position: [0., 0., 0., 1.],
        normal: [0., -1., 0., 0.],
    },
    Vertex {
        position: [1., 0., 0., 1.],
        normal: [0., -1., 0., 0.],
    },
];

pub const INDICES: &[u16] = &[
    0, 1, 2, 2, 3, 0, // top
    4, 5, 6, 6, 7, 4, // bottom
    8, 9, 10, 10, 11, 8, // right
    12, 13, 14, 14, 15, 12, // left
    16, 17, 18, 18, 19, 16, // front
    20, 21, 22, 22, 23, 20, // back
];

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Instance {
    position: [u8; 4],
    color: [u8; 4],
}

impl Instance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Uint8x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[u8; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Unorm8x4,
                },
            ],
        }
    }
}
