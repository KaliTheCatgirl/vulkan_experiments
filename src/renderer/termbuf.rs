use std::sync::Arc;

#[derive(Zeroable, Pod, Clone, Copy)]
#[repr(C)]
pub struct TerminalUBO {
    width: u32,
    height: u32,
}
impl TerminalUBO {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use glam::{vec2, Mat4, Quat, Vec2, Vec3};
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::CommandBufferAllocator, AutoCommandBufferBuilder, CopyBufferToImageInfo,
    },
    descriptor_set::WriteDescriptorSet,
    device::Device,
    image::{
        sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
        view::ImageView,
        Image, ImageUsage,
    },
    memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter},
    pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint},
};

use crate::{
    create_descriptor_set,
    vertex::{self, CommonVertex},
    TransformUBO,
};

use super::{color::Color, ext::CommandBufferExt};

pub const PANEL_VERTICES: [vertex::CommonVertex; 4] = [
    vertex::CommonVertex {
        position: [-1.0, -1.0, 0.0],
        color: [1.0; 4],
        uv: [0.0, 0.0],
    },
    vertex::CommonVertex {
        position: [1.0, -1.0, 0.0],
        color: [1.0; 4],
        uv: [1.0, 0.0],
    },
    vertex::CommonVertex {
        position: [-1.0, 1.0, 0.0],
        color: [1.0; 4],
        uv: [0.0, 1.0],
    },
    vertex::CommonVertex {
        position: [1.0, 1.0, 0.0],
        color: [1.0; 4],
        uv: [1.0, 1.0],
    },
];
pub const PANEL_INDICES: [u32; 6] = [0, 1, 2, 3, 2, 1];

#[derive(Clone)]
pub struct TerminalPanel {
    width: u32,
    height: u32,

    transform: Mat4,

    sampler: Arc<Sampler>,

    charset: Arc<Image>,
    character_image: Arc<Image>,
    foreground_image: Arc<Image>,
    background_image: Arc<Image>,

    pub vertex_buffer: Subbuffer<[CommonVertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub uniform_buffer: Subbuffer<TerminalUBO>,
    pub transform_buffer: Subbuffer<TransformUBO>,

    pub character_buffer: Subbuffer<[u8]>,
    pub foreground_buffer: Subbuffer<[Color]>,
    pub background_buffer: Subbuffer<[Color]>,
}
impl TerminalPanel {
    pub fn new(
        width: u32,
        height: u32,
        loader_command_buffer: &mut impl CommandBufferExt,
        allocator: Arc<dyn MemoryAllocator>,
        device: Arc<Device>,
        charset: Option<Arc<Image>>,
        vertices: impl ExactSizeIterator<Item = vertex::CommonVertex>,
        indices: impl ExactSizeIterator<Item = u32>,
    ) -> Result<Self> {
        Self::with_transform(
            width,
            height,
            loader_command_buffer,
            allocator,
            device,
            Mat4::IDENTITY,
            charset,
            vertices,
            indices,
        )
    }
    pub fn with_transform(
        width: u32,
        height: u32,
        loader_command_buffer: &mut impl CommandBufferExt,
        allocator: Arc<dyn MemoryAllocator>,
        device: Arc<Device>,
        transform: Mat4,
        charset: Option<Arc<Image>>,
        vertices: impl ExactSizeIterator<Item = vertex::CommonVertex>,
        indices: impl ExactSizeIterator<Item = u32>,
    ) -> Result<Self> {
        let charset_image = match charset {
            Some(charset) => charset,
            None => {
                loader_command_buffer
                    .load_image("charset.png", allocator.clone(), ImageUsage::SAMPLED)?
                    .1
            }
        };

        let (_, character_image, character_buffer) = loader_command_buffer
            .create_blank_image::<u8>(
                width,
                height,
                allocator.clone(),
                ImageUsage::SAMPLED,
                MemoryTypeFilter::HOST_RANDOM_ACCESS | MemoryTypeFilter::PREFER_HOST,
                BufferUsage::TRANSFER_SRC,
            )?;

        let (_, foreground_image, foreground_buffer) = loader_command_buffer
            .create_blank_image::<Color>(
                width,
                height,
                allocator.clone(),
                ImageUsage::SAMPLED,
                MemoryTypeFilter::HOST_RANDOM_ACCESS | MemoryTypeFilter::PREFER_HOST,
                BufferUsage::TRANSFER_SRC,
            )?;

        let (_, background_image, background_buffer) = loader_command_buffer
            .create_blank_image::<Color>(
                width,
                height,
                allocator.clone(),
                ImageUsage::SAMPLED,
                MemoryTypeFilter::HOST_RANDOM_ACCESS | MemoryTypeFilter::PREFER_HOST,
                BufferUsage::TRANSFER_SRC,
            )?;

        let uniform_buffer = Buffer::from_data(
            allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            TerminalUBO::new(width, height),
        )?;

        let transform_buffer = Buffer::from_data(
            allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            TransformUBO::new(Mat4::IDENTITY),
        )?;

        let vertex_buffer = Buffer::from_iter(
            allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vertices,
        )?;

        let index_buffer = Buffer::from_iter(
            allocator,
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            indices,
        )?;

        Ok(Self {
            width,
            height,
            transform,

            sampler: Sampler::new(
                device.clone(),
                SamplerCreateInfo {
                    mag_filter: Filter::Nearest,
                    min_filter: Filter::Nearest,
                    address_mode: [SamplerAddressMode::Repeat; 3],
                    ..Default::default()
                },
            )?,

            vertex_buffer,
            index_buffer,
            uniform_buffer,
            transform_buffer,

            charset: charset_image,
            character_image,
            foreground_image,
            background_image,

            character_buffer,
            foreground_buffer,
            background_buffer,
        })
    }

    pub fn update<L, A: CommandBufferAllocator>(
        &self,
        upload_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
    ) {
        upload_command_buffer
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                self.character_buffer.clone(),
                self.character_image.clone(),
            ))
            .unwrap()
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                self.foreground_buffer.clone(),
                self.foreground_image.clone(),
            ))
            .unwrap()
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                self.background_buffer.clone(),
                self.background_image.clone(),
            ))
            .unwrap();
    }
    pub fn flat_transform(&mut self, center: Vec3, rotation: Quat, character_size: Vec2) {
        self.transform = Mat4::from_scale_rotation_translation(
            (character_size * vec2(self.width as f32, self.height as f32) * 0.5).extend(1.0),
            rotation,
            center,
        );
    }

    pub fn texture_descriptor_writes(&self) -> [WriteDescriptorSet; 5] {
        [
            WriteDescriptorSet::sampler(0, self.sampler.clone()),
            WriteDescriptorSet::image_view(
                1,
                ImageView::new_default(self.charset.clone()).unwrap(),
            ),
            WriteDescriptorSet::image_view(
                2,
                ImageView::new_default(self.character_image.clone()).unwrap(),
            ),
            WriteDescriptorSet::image_view(
                3,
                ImageView::new_default(self.foreground_image.clone()).unwrap(),
            ),
            WriteDescriptorSet::image_view(
                4,
                ImageView::new_default(self.background_image.clone()).unwrap(),
            ),
        ]
    }

    pub fn draw<'a, L, A: CommandBufferAllocator>(
        &self,
        render_command_buffer_builder: &'a mut AutoCommandBufferBuilder<L, A>,
        pipeline: &Arc<GraphicsPipeline>,
        device: Arc<Device>,
        vp: Mat4,
    ) -> Result<&'a mut AutoCommandBufferBuilder<L, A>> {
        *self.uniform_buffer.write()? = TerminalUBO::new(self.width, self.height);
        *self.transform_buffer.write()? = TransformUBO::new(vp * self.transform);

        Ok(render_command_buffer_builder
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipeline.layout().clone(),
                0,
                create_descriptor_set(
                    0,
                    self.texture_descriptor_writes(),
                    pipeline,
                    device.clone(),
                )?,
            )?
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipeline.layout().clone(),
                1,
                create_descriptor_set(
                    1,
                    [WriteDescriptorSet::buffer(0, self.uniform_buffer.clone())],
                    pipeline,
                    device.clone(),
                )?,
            )?
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipeline.layout().clone(),
                2,
                create_descriptor_set(
                    2,
                    [WriteDescriptorSet::buffer(0, self.transform_buffer.clone())],
                    pipeline,
                    device,
                )?,
            )?
            .bind_vertex_buffers(0, self.vertex_buffer.clone())?
            .bind_index_buffer(self.index_buffer.clone())?
            .draw_indexed(self.index_buffer.len() as u32, 1, 0, 0, 0)?)
    }

    pub fn fill_chars(&self, character: u8) -> Result<()> {
        Ok(self.character_buffer.write()?.fill(character))
    }
    pub fn fill_fg(&self, color: Color) -> Result<()> {
        Ok(self.foreground_buffer.write()?.fill(color))
    }
    pub fn fill_bg(&self, color: Color) -> Result<()> {
        Ok(self.background_buffer.write()?.fill(color))
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn print(
        &self,
        x: u32,
        y: u32,
        text: &str,
        foreground: Option<Color>,
        background: Option<Color>,
    ) -> Result<()> {
        if x >= self.width || y >= self.height {
            return Ok(());
        }
        let index = x + y * self.width;
        let write_length = (text.len() as u32).min(self.width - x);
        let write_range = index as usize..(index + write_length) as usize;
        if let Some(fg) = foreground {
            self.foreground_buffer.write()?[write_range.clone()].fill(fg);
        }
        if let Some(bg) = background {
            self.background_buffer.write()?[write_range.clone()].fill(bg);
        }
        self.character_buffer.write()?[write_range].copy_from_slice(text.as_bytes());
        Ok(())
    }
}

pub mod shaders {
    pub mod vertex {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: r"
                #version 460
    
                layout(location = 0) in vec3 position;
                layout(location = 1) in vec4 color;
                layout(location = 2) in vec2 uv;
    
                layout(location = 0) out vec4 fragment_color;
                layout(location = 1) out vec2 fragment_uv;
                layout(location = 2) out vec2 cell_pos;
    
                layout(set = 1, binding = 0) uniform terminal_dims {
                    uint width;
                    uint height;
                };
    
                layout(set = 2, binding = 0) uniform transform {
                    mat4 transformation;
                };
    
                void main() {
                    gl_Position = transformation * vec4(position, 1.0);
                    fragment_color = color;
                    fragment_uv = uv;
                    cell_pos = uv * vec2(float(width), float(height));
                }
            ",
        }
    }
    pub mod fragment {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: r"
                #version 460
    
                layout(set = 0, binding = 0) uniform sampler s;
                layout(set = 0, binding = 1) uniform texture2D charset;
                layout(set = 0, binding = 2) uniform utexture2D charbuf;
                layout(set = 0, binding = 3) uniform texture2D foreground;
                layout(set = 0, binding = 4) uniform texture2D background;
    
                layout(location = 0) in vec4 fragment_color;
                layout(location = 1) in vec2 fragment_uv;
                layout(location = 2) in vec2 cell_pos;
    
                layout(location = 0) out vec4 color;
    
                void main() {
                    float character = 1.0 - texture(usampler2D(charbuf, s), fragment_uv).x * (255.0 / 256.0);
                    float fracx = fract(cell_pos.x) / 258.0;
                    color = mix(
                        texture(sampler2D(background, s), fragment_uv),
                        texture(sampler2D(foreground, s), fragment_uv),
                        texture(sampler2D(charset, s), vec2(character + fracx, fract(cell_pos.y))).x
                    ) * fragment_color;
                    if (color.w == 0.0) { discard; }
                }
            ",
        }
    }
}
