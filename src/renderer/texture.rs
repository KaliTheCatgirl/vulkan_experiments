use bytemuck::Pod;
use glam::vec4;
use vulkano::{buffer::BufferContents, format::Format};

use super::color::Color;

pub trait PixelType: Sized + BufferContents + Pod {
    const VULKAN_FORMAT: Format;
    const BLACK: Self;
}

impl PixelType for f32 {
    const VULKAN_FORMAT: Format = Format::R32_SFLOAT;
    const BLACK: Self = 0.0;
}
impl PixelType for [f32; 1] {
    const VULKAN_FORMAT: Format = Format::R32_SFLOAT;
    const BLACK: Self = [0.0];
}
impl PixelType for [f32; 2] {
    const VULKAN_FORMAT: Format = Format::R32G32_SFLOAT;
    const BLACK: Self = [0.0; 2];
}
impl PixelType for [f32; 3] {
    const VULKAN_FORMAT: Format = Format::R32G32B32_SFLOAT;
    const BLACK: Self = [0.0; 3];
}
impl PixelType for [f32; 4] {
    const VULKAN_FORMAT: Format = Format::R32G32B32A32_SFLOAT;
    const BLACK: Self = [0.0, 0.0, 0.0, 1.0];
}
impl PixelType for Color {
    const VULKAN_FORMAT: Format = Format::R32G32B32A32_SFLOAT;
    const BLACK: Self = vec4(0.0, 0.0, 0.0, 1.0);
}

impl PixelType for u8 {
    const VULKAN_FORMAT: Format = Format::R8_UINT;
    const BLACK: Self = 0;
}
impl PixelType for [u8; 1] {
    const VULKAN_FORMAT: Format = Format::R8_UINT;
    const BLACK: Self = [0];
}
impl PixelType for [u8; 2] {
    const VULKAN_FORMAT: Format = Format::R8G8_UINT;
    const BLACK: Self = [0; 2];
}
impl PixelType for [u8; 3] {
    const VULKAN_FORMAT: Format = Format::R8G8B8_UINT;
    const BLACK: Self = [0; 3];
}
impl PixelType for [u8; 4] {
    const VULKAN_FORMAT: Format = Format::R8G8B8A8_UINT;
    const BLACK: Self = [0, 0, 0, 255];
}
