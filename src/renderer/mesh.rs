use std::{
    fs::{self},
    path::Path,
    sync::Arc,
};

use anyhow::{anyhow, Result};
use glam::Mat4;
use goth_gltf::{default_extensions::Extensions, AccessorType, Gltf};
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{allocator::CommandBufferAllocator, AutoCommandBufferBuilder},
    descriptor_set::WriteDescriptorSet,
    device::{Device, DeviceOwned},
    image::{
        sampler::{Filter, Sampler, SamplerCreateInfo},
        view::ImageView,
        ImageUsage,
    },
    memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter},
    pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint},
};

use crate::{create_descriptor_set, vertex};

use super::ext::CommandBufferExt;

pub struct Mesh {
    pub vertex_buffer: Subbuffer<[vertex::CommonVertex]>,
    pub index_buffer: Subbuffer<[u32]>,

    pub texture: Arc<ImageView>,
    pub texture_sampler: Arc<Sampler>,
}
impl Mesh {
    pub fn new(
        allocator: Arc<dyn MemoryAllocator>,
        vertices: impl ExactSizeIterator<Item = vertex::CommonVertex>,
        indices: impl ExactSizeIterator<Item = u32>,
        image: Arc<ImageView>,
        sampler: Arc<Sampler>,
    ) -> Result<Self> {
        Self::with_buffer_usage(
            allocator,
            vertices,
            indices,
            image,
            sampler,
            BufferUsage::TRANSFER_DST,
            MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
        )
    }
    pub fn load_gltf<P: AsRef<Path>>(
        path: P,
        allocator: Arc<dyn MemoryAllocator>,
        loader_commands: &mut impl CommandBufferExt,
        filter: Filter,
        device: Arc<Device>,
    ) -> Result<Self> {
        let data = fs::read(path)?;
        let (gltf, binary): (Gltf<Extensions>, Option<&[u8]>) =
            Gltf::<Extensions>::from_bytes(&data)?;
        let mesh = &gltf.meshes[0];
        let attributes = &mesh.primitives[0].attributes;

        let binary = binary.ok_or(anyhow!("no binary data in gltf"))?;

        let position_accessor = &gltf.accessors[attributes
            .position
            .ok_or(anyhow!("no position buffer specified"))?];
        dbg!(&position_accessor.accessor_type);
        if let AccessorType::Vec3 = position_accessor.accessor_type {
        } else {
            Err(anyhow!("expected position buffer type to be vec3"))?;
        }
        let position_view = &gltf.buffer_views[position_accessor
            .buffer_view
            .ok_or(anyhow!("no buffer view specified in position accessor"))?];
        let position_data = &binary
            [position_view.byte_offset..position_view.byte_offset + position_view.byte_length];

        let uv_accessor = &gltf.accessors[attributes
            .texcoord_0
            .ok_or(anyhow!("no uv buffer specified"))?];
        if let AccessorType::Vec2 = uv_accessor.accessor_type {
        } else {
            Err(anyhow!("expected uv buffer type to be vec2"))?;
        }
        let uv_view = &gltf.buffer_views[uv_accessor
            .buffer_view
            .ok_or(anyhow!("no buffer view specified in uv accessor"))?];
        let uv_data = &binary[uv_view.byte_offset..uv_view.byte_offset + uv_view.byte_length];

        let vertices = bytemuck::try_cast_slice::<u8, [f32; 3]>(position_data)
            .map_err(|e| anyhow!(e))?
            .iter()
            .zip(bytemuck::try_cast_slice::<u8, [f32; 2]>(uv_data).map_err(|e| anyhow!(e))?)
            .map(|(position, uv)| vertex::CommonVertex {
                position: *position,
                color: [1.0; 4],
                uv: *uv,
            });

        let index_view = &gltf.buffer_views[gltf.accessors[mesh.primitives[0]
            .indices
            .ok_or(anyhow!("no index buffer specified"))?]
        .buffer_view
        .ok_or(anyhow!("no buffer view specified in index accessor"))?];
        let index_data =
            &binary[index_view.byte_offset..index_view.byte_offset + index_view.byte_length];
        let indices = bytemuck::try_cast_slice::<u8, u16>(index_data).map_err(|e| anyhow!(e))?;

        let image_view = &gltf.buffer_views[gltf.images[0]
            .buffer_view
            .ok_or(anyhow!("no buffer view specified in image"))?];

        let image_data =
            &binary[image_view.byte_offset..image_view.byte_offset + image_view.byte_length];

        let image = ImageView::new_default(
            loader_commands
                .load_image_from_memory(image_data, allocator.clone(), ImageUsage::SAMPLED)?
                .1,
        )?;
        let sampler = Sampler::new(
            device,
            SamplerCreateInfo {
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            },
        )?;

        Mesh::new(
            allocator,
            vertices,
            indices.into_iter().map(|i| *i as u32),
            image,
            sampler,
        )
    }
    pub fn with_buffer_usage(
        allocator: Arc<dyn MemoryAllocator>,
        vertices: impl ExactSizeIterator<Item = vertex::CommonVertex>,
        indices: impl ExactSizeIterator<Item = u32>,
        image: Arc<ImageView>,
        sampler: Arc<Sampler>,
        buffer_usage: BufferUsage,
        memory_filter: MemoryTypeFilter,
    ) -> Result<Self> {
        let vertex_buffer = Buffer::from_iter(
            allocator.clone(),
            BufferCreateInfo {
                usage: buffer_usage | BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: memory_filter,
                ..Default::default()
            },
            vertices,
        )?;
        let index_buffer = Buffer::from_iter(
            allocator.clone(),
            BufferCreateInfo {
                usage: buffer_usage | BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: memory_filter,
                ..Default::default()
            },
            indices,
        )?;
        Ok(Self {
            vertex_buffer,
            index_buffer,
            texture: image,
            texture_sampler: sampler,
        })
    }
    pub fn rebind_transform<L, A: CommandBufferAllocator>(&self, render_commands: &mut AutoCommandBufferBuilder<L, A>, pipeline: Arc<GraphicsPipeline>, allocator: Arc<dyn MemoryAllocator>, transform: Mat4) -> Result<()> {
        let device = render_commands.device().clone();
        render_commands
        .bind_descriptor_sets(
            PipelineBindPoint::Graphics,
            pipeline.layout().clone(),
            1,
            create_descriptor_set(
                1,
                [WriteDescriptorSet::buffer(0, Buffer::from_data(
                    allocator,
                    BufferCreateInfo {
                        usage: BufferUsage::UNIFORM_BUFFER,
                        ..Default::default()
                    },
                    AllocationCreateInfo {
                        memory_type_filter: MemoryTypeFilter::PREFER_DEVICE | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                        ..Default::default()
                    },
                    transform.to_cols_array_2d(),
                )?)],
                &pipeline,
                device,
            )?,
        )?;
        Ok(())
    }
    pub fn bind<L, A: CommandBufferAllocator>(
        &self,
        render_commands: &mut AutoCommandBufferBuilder<L, A>,
        pipeline: &Arc<GraphicsPipeline>,
    ) -> Result<()> {
        let device = render_commands.device().clone();
        render_commands
            .bind_index_buffer(self.index_buffer.clone())?
            .bind_vertex_buffers(0, self.vertex_buffer.clone())?
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipeline.layout().clone(),
                0,
                create_descriptor_set(
                    0,
                    [
                        WriteDescriptorSet::sampler(0, self.texture_sampler.clone()),
                        WriteDescriptorSet::image_view(1, self.texture.clone()),
                    ],
                    pipeline,
                    device.clone(),
                )?,
            )?;
        Ok(())
    }
    pub fn add_draw_command<L, A: CommandBufferAllocator>(
        &self,
        render_commands: &mut AutoCommandBufferBuilder<L, A>,
    ) -> Result<()> {
        render_commands.draw_indexed(self.index_buffer.len() as u32, 1, 0, 0, 0)?;
        Ok(())
    }
    pub fn draw<L, A: CommandBufferAllocator>(
        &self,
        allocator: Arc<dyn MemoryAllocator>,
        render_commands: &mut AutoCommandBufferBuilder<L, A>,
        pipeline: Arc<GraphicsPipeline>,
        transform: Mat4,
    ) -> Result<()> {
        self.bind(render_commands, &pipeline);
        self.rebind_transform(render_commands, pipeline, allocator, transform);
        self.add_draw_command(render_commands);
        Ok(())
    }
    pub fn draw_prebound<L, A: CommandBufferAllocator>(
        &self,
        allocator: Arc<dyn MemoryAllocator>,
        render_commands: &mut AutoCommandBufferBuilder<L, A>,
        pipeline: Arc<GraphicsPipeline>,
        transform: Mat4,
    ) -> Result<()> {
        self.rebind_transform(render_commands, pipeline, allocator, transform);
        self.add_draw_command(render_commands);
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

            layout(set = 1, binding = 0) uniform transform {
                mat4 transformation;
            };

            void main() {
                gl_Position = transformation * vec4(position, 1.0);
                fragment_color = color;
                fragment_uv = uv;
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
            layout(set = 0, binding = 1) uniform texture2D tex;

            layout(location = 0) in vec4 fragment_color;
            layout(location = 1) in vec2 fragment_uv;

            layout(location = 0) out vec4 color;

            void main() {
                color = texture(sampler2D(tex, s), fragment_uv) * fragment_color;
            }
        ",
        }
    }
}
