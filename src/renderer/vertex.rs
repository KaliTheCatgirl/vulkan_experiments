use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex};

#[derive(Vertex, BufferContents, PartialEq, PartialOrd, Clone, Copy)]
#[repr(C)]
pub struct CommonVertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32B32A32_SFLOAT)]
    pub color: [f32; 4],
    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2],
}