use std::{
    fs,
    ops::{Add, Mul, Sub},
    path::Path,
    sync::Arc,
};

use anyhow::Result;
use bytemuck::Pod;
use image::Rgba32FImage;
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::CommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferExecFuture,
        CommandBufferUsage, CopyBufferToImageInfo,
    },
    device::{Device, Queue},
    format::Format,
    image::{Image, ImageCreateInfo, ImageLayout, ImageType, ImageUsage},
    memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter},
    sync::{
        self,
        future::{FenceSignalFuture, NowFuture},
        GpuFuture,
    },
};

use super::texture::PixelType;

pub trait CommandBufferExt {
    fn load_image<P: AsRef<Path>>(
        &mut self,
        path: P,
        allocator: Arc<dyn MemoryAllocator>,
        usage: ImageUsage,
    ) -> Result<(&mut Self, Arc<Image>)> {
        self.load_image_from_memory(&fs::read(path)?, allocator, usage)
    }

    fn load_image_from_memory(
        &mut self,
        data: &[u8],
        allocator: Arc<dyn MemoryAllocator>,
        usage: ImageUsage,
    ) -> Result<(&mut Self, Arc<Image>)> {
        self.load_image_from_rgba32f(
            image::load_from_memory(data)?.to_rgba32f(),
            allocator,
            usage,
        )
    }

    fn load_image_from_rgba32f(
        &mut self,
        data: Rgba32FImage,
        allocator: Arc<dyn MemoryAllocator>,
        usage: ImageUsage,
    ) -> Result<(&mut Self, Arc<Image>)>;

    fn create_blank_image<C: PixelType>(
        &mut self,
        width: u32,
        height: u32,
        allocator: Arc<dyn MemoryAllocator>,
        image_usage: ImageUsage,
        buffer_memory_filter: MemoryTypeFilter,
        buffer_usage: BufferUsage,
    ) -> Result<(&mut Self, Arc<Image>, Subbuffer<[C]>)>;
}

impl<L, A: CommandBufferAllocator + 'static> CommandBufferExt for AutoCommandBufferBuilder<L, A> {
    fn load_image_from_rgba32f(
        &mut self,
        image_data: Rgba32FImage,
        allocator: Arc<dyn MemoryAllocator>,
        usage: ImageUsage,
    ) -> Result<(&mut Self, Arc<Image>)> {
        let image = Image::new(
            allocator.clone(),
            ImageCreateInfo {
                format: Format::R32G32B32A32_SFLOAT,
                image_type: ImageType::Dim2d,
                usage: ImageUsage::TRANSFER_DST | usage,
                extent: [image_data.width(), image_data.height(), 1],
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )?;
        let staging_buffer = Buffer::from_iter(
            allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            image_data.into_iter().copied(),
        )?;
        self.copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
            staging_buffer,
            image.clone(),
        ))
        .unwrap();
        Ok((self, image))
    }

    fn create_blank_image<C: PixelType>(
        &mut self,
        width: u32,
        height: u32,
        allocator: Arc<dyn MemoryAllocator>,
        image_usage: ImageUsage,
        buffer_memory_type: MemoryTypeFilter,
        buffer_usage: BufferUsage,
    ) -> Result<(&mut Self, Arc<Image>, Subbuffer<[C]>)> {
        let image = Image::new(
            allocator.clone(),
            ImageCreateInfo {
                format: C::VULKAN_FORMAT,
                image_type: ImageType::Dim2d,
                usage: ImageUsage::TRANSFER_DST | image_usage,
                extent: [width, height, 1],
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )?;
        let staging_buffer = Buffer::from_iter(
            allocator.clone(),
            BufferCreateInfo {
                usage: buffer_usage,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: buffer_memory_type,
                ..Default::default()
            },
            (0..width * height).map(|_| C::BLACK),
        )?;
        self.copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
            staging_buffer.clone(),
            image.clone(),
        ))
        .unwrap();
        Ok((self, image, staging_buffer))
    }
}

// generic af trait
pub trait LerpExt<By = Self, Rhs = Self, Output = Self> {
    fn lerp(self, other: Rhs, mix: By) -> Output;
}
impl<
        Factored,
        Diff: Mul<By, Output = Factored>,
        T: Clone + Add<Factored, Output = Output>,
        Rhs: Sub<T, Output = Diff>,
        By,
        Output,
    > LerpExt<By, Rhs, Output> for T
{
    fn lerp(self, other: Rhs, mix: By) -> Output {
        self.clone() + (other - self) * mix
    }
}
