use std::sync::Arc;

use anyhow::Result;
use vulkano::{
    command_buffer::{allocator::CommandBufferAllocator, AutoCommandBufferBuilder},
    device::Device,
    memory::allocator::MemoryAllocator,
    pipeline::graphics::viewport::Viewport,
    render_pass::RenderPass,
};
use winit::dpi::PhysicalSize;

pub trait App: Sized {
    const INITIAL_SIZE: PhysicalSize<u32> = PhysicalSize::new(1600, 900);

    fn new<L, A: CommandBufferAllocator + 'static>(
        loader_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
        allocator: Arc<dyn MemoryAllocator>,
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> Result<Self>;

    fn update<L, A: CommandBufferAllocator + 'static>(
        &mut self,
        upload_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
    ) -> Result<()>;

    fn draw<L, A: CommandBufferAllocator + 'static>(
        &mut self,
        render_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
    ) -> Result<()>;

    fn resize(&mut self, new_size: PhysicalSize<u32>) -> Result<()>;
    fn done(&self) -> bool {
        false
    }
    fn audio(&self) -> Option<(&'static str, f64)> {
        None
    }
}
