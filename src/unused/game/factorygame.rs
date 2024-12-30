use std::{f32::consts::PI, sync::Arc};

use anyhow::Result;
use data::{Meshes, Pipelines};
use glam::{vec3, Mat4, Vec3};
use vulkano::{
    buffer::BufferUsage,
    command_buffer::{
        allocator::CommandBufferAllocator, AutoCommandBufferBuilder, CopyBufferToImageInfo,
    },
    device::Device,
    image::{
        sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
        view::ImageView,
        Image, ImageUsage,
    },
    memory::allocator::{MemoryAllocator, MemoryTypeFilter},
};
use winit::dpi::PhysicalSize;

use crate::{app::App, ext::CommandBufferExt, mesh::Mesh, stopwatch::Stopwatch, termbuf};

use vulkano::{
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline},
    render_pass::RenderPass,
};

use crate::create_graphics_pipeline;

mod data {
    use crate::mesh;

    use super::*;

    pub struct Pipelines {
        pub mesh_pipeline: Arc<GraphicsPipeline>,
    }
    impl Pipelines {
        pub fn new(
            device: Arc<Device>,
            render_pass: Arc<RenderPass>,
            viewport: Viewport,
        ) -> Result<Self> {
            Ok(Self {
                mesh_pipeline: create_graphics_pipeline(
                    device.clone(),
                    mesh::shaders::vertex::load(device.clone())?,
                    mesh::shaders::fragment::load(device)?,
                    render_pass,
                    viewport,
                )?,
            })
        }
    }

    pub struct Meshes {
        pub duct: Mesh,
        pub iron_ore: Mesh,
    }
    impl Meshes {
        pub fn new(
            loader_commands: &mut impl CommandBufferExt,
            allocator: Arc<dyn MemoryAllocator>,
            device: Arc<Device>,
        ) -> Result<Self> {
            Ok(Self {
                duct: Mesh::load_gltf(
                    "models/duct.glb",
                    allocator.clone(),
                    loader_commands,
                    Filter::Nearest,
                    device.clone(),
                )?,
                iron_ore: Mesh::load_gltf(
                    "models/iron ore.glb",
                    allocator.clone(),
                    loader_commands,
                    Filter::Nearest,
                    device.clone(),
                )?,
            })
        }
    }
}

pub struct VoxelFactory {
    pipelines: Pipelines,
    device: Arc<Device>,
    render_pass: Arc<RenderPass>,
    allocator: Arc<dyn MemoryAllocator>,

    meshes: Meshes,
    stopwatch: Stopwatch,
}
impl App for VoxelFactory {
    fn new<L, A: CommandBufferAllocator + 'static>(
        loader_commands: &mut AutoCommandBufferBuilder<L, A>,
        allocator: Arc<dyn MemoryAllocator>,
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> Result<Self> {
        return Ok(Self {
            meshes: Meshes::new(loader_commands, allocator.clone(), device.clone())?,
            pipelines: Pipelines::new(device.clone(), render_pass.clone(), viewport)?,
            device,
            render_pass,
            allocator,
            stopwatch: Stopwatch::new(),
        });
    }

    fn update<L, A: CommandBufferAllocator>(
        &mut self,
        upload_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
    ) -> Result<()> {
        Ok(())
    }

    fn draw<L, A: CommandBufferAllocator>(
        &mut self,
        render_commands: &mut AutoCommandBufferBuilder<L, A>,
    ) -> Result<()> {
        let t = self.stopwatch.get().as_secs_f32();
        let vp = Mat4::from_scale(vec3(1.0, -1.0, 1.0)) * Mat4::perspective_lh(PI / 2.0, 1600.0 / 900.0, 0.01, 1000.0)
        * Mat4::look_at_lh(vec3(t.sin() * 3.0, 3.0, t.cos() * 3.0), vec3(0.0, 0.0, 0.0), Vec3::Y);
        render_commands.bind_pipeline_graphics(self.pipelines.mesh_pipeline.clone())?;

        self.meshes.iron_ore.draw(
            self.allocator.clone(),
            render_commands,
            self.pipelines.mesh_pipeline.clone(),
            vp,
        )?;

        self.meshes.duct.bind(render_commands, &self.pipelines.mesh_pipeline)?;
        for i in (0..10).rev() {
            self.meshes.duct.draw_prebound(
                self.allocator.clone(),
                render_commands,
                self.pipelines.mesh_pipeline.clone(),
                vp * Mat4::from_translation(vec3(i as f32, 0.0, 0.0)),
            )?;
        }
        Ok(())
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) -> Result<()> {
        self.pipelines = Pipelines::new(
            self.device.clone(),
            self.render_pass.clone(),
            Viewport {
                offset: [0.0, 0.0],
                extent: [new_size.width as f32, new_size.height as f32],
                ..Default::default()
            },
        )?;
        Ok(())
    }
}
