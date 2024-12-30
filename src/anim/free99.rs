use std::{fs::File, sync::Arc};

use rodio::OutputStream;
use vulkano::{command_buffer::{allocator::CommandBufferAllocator, AutoCommandBufferBuilder}, device::Device, memory::allocator::MemoryAllocator, pipeline::graphics::viewport::Viewport, render_pass::RenderPass};
use winit::dpi::PhysicalSize;

use crate::renderer::{app::App, misc::SinkExtrapolator};

mod data {
    use std::sync::Arc;

    use anyhow::Result;
    use vulkano::{
        device::Device,
        image::sampler::Filter,
        memory::allocator::MemoryAllocator,
        pipeline::{graphics::viewport::Viewport, GraphicsPipeline},
        render_pass::RenderPass,
    };

    use crate::{create_graphics_pipeline, renderer::{ext::CommandBufferExt, mesh}};

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

    pub struct Meshes {}
    impl Meshes {
        pub fn new(
            loader_commands: &mut impl CommandBufferExt,
            allocator: Arc<dyn MemoryAllocator>,
            device: Arc<Device>,
        ) -> Result<Self> {
            Ok(Self {
                // duct: Mesh::load_gltf(
                //     "models/duct.glb",
                //     allocator.clone(),
                //     loader_commands,
                //     Filter::Nearest,
                //     device.clone(),
                // )?,
            })
        }
    }
}

pub struct BULLETINMYBRAIN {
    audio: SinkExtrapolator,
}
impl App for BULLETINMYBRAIN {
    fn new<L, A: CommandBufferAllocator + 'static>(
        loader_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
        allocator: Arc<dyn MemoryAllocator>,
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> anyhow::Result<Self> {
        let (_os, osh) = OutputStream::try_default()?;
        Ok(Self {
            audio: SinkExtrapolator::new(osh.play_once(File::open("bullet in my brain.ogg")?)?),
        })
    }

    fn update<L, A: CommandBufferAllocator + 'static>(
        &mut self,
        upload_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn draw<L, A: CommandBufferAllocator + 'static>(
        &mut self,
        render_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) -> anyhow::Result<()> {
        Ok(())
    }
}
