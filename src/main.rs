mod anim;
mod renderer;

use std::{
    io::{self, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::Arc,
};

use anim::{free99::BULLETINMYBRAIN, ta1lsd003::TA1LSD003, ta1lsd005::TA1LSD005};
use anyhow::Result;
use glam::Mat4;
use image::{Rgba32FImage, RgbaImage};
use renderer::{app::App, ext::CommandBufferExt, vertex};
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage},
    command_buffer::{
        allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo},
        AutoCommandBufferBuilder, CommandBufferUsage, CopyImageInfo, CopyImageToBufferInfo,
        PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract, RenderPassBeginInfo,
        SubpassBeginInfo, SubpassContents, SubpassEndInfo,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
    },
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageType, ImageUsage},
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo},
    memory::allocator::{
        AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter, StandardMemoryAllocator,
    },
    pipeline::{
        graphics::{
            color_blend::{
                AttachmentBlend, BlendFactor, BlendOp, ColorBlendAttachmentState, ColorBlendState,
            },
            depth_stencil::{DepthState, DepthStencilState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            vertex_input::{Vertex, VertexDefinition},
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    shader::ShaderModule,
    swapchain::{PresentMode, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo},
    sync::{self, future::FenceSignalFuture, GpuFuture},
    Validated, VulkanError, VulkanLibrary,
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn ffmpeg_video_stream(
    width: usize,
    height: usize,
    file: impl AsRef<str>,
) -> Result<(Child, ChildStdin)> {
    let mut child = Command::new("ffmpeg")
        .args([
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-s:v",
            &format!("{width}x{height}"),
            "-r",
            "60",
            "-i",
            "pipe:",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-y",
            file.as_ref(),
        ])
        .stdin(Stdio::piped())
        // .stdout(Stdio::null())
        // .stdin(Stdio::null())
        .spawn()?;
    let pixel_input = child.stdin.take().unwrap();
    Ok((child, pixel_input))
}
fn merge_av(
    audio: impl AsRef<str>,
    audio_offset: f64,
    video: impl AsRef<str>,
    output: impl AsRef<str>,
) -> Result<(Child, ChildStdin)> {
    let mut child = Command::new("ffmpeg")
        .args([
            "-ss",
            "0.0",
            "-i",
            video.as_ref(),
            "-ss",
            &format!("{audio_offset}"),
            "-i",
            audio.as_ref(),
            "-c:v",
            "copy",
            "-c:a",
            "mp3",
            "-shortest",
            "-y",
            output.as_ref(),
        ])
        .stdin(Stdio::piped())
        // .stdout(Stdio::null())
        // .stdin(Stdio::null())
        .spawn()?;
    let pixel_input = child.stdin.take().unwrap();
    Ok((child, pixel_input))
}

#[derive(BufferContents)]
#[repr(C)]
struct TransformUBO {
    transform: [[f32; 4]; 4],
}
impl TransformUBO {
    pub fn new(transform: Mat4) -> Self {
        Self {
            transform: transform.to_cols_array_2d(),
        }
    }
}

fn get_physical_device(
    instance: &Arc<Instance>,
    surface: &Arc<Surface>,
    device_extensions: &DeviceExtensions,
) -> (Arc<PhysicalDevice>, u32) {
    instance
        .enumerate_physical_devices()
        .expect("could not enumerate devices")
        .filter(|p| p.supported_extensions().contains(&device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.contains(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, &surface).unwrap_or(false)
                })
                .map(|q| (p, q as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            _ => 4,
        })
        .expect("no device available")
}

fn create_render_pass(device: Arc<Device>, format: Format) -> Result<Arc<RenderPass>> {
    Ok(vulkano::single_pass_renderpass!(
        device.clone(),
        attachments: {
            color: {
                format: format,
                samples: 1,
                load_op: Clear,
                store_op: Store,
            },
            depth_stencil: {
                format: Format::D16_UNORM,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {depth_stencil},
        },
    )?)
}

fn create_framebuffers(
    images: &[Arc<Image>],
    render_pass: &Arc<RenderPass>,
    allocator: Arc<dyn MemoryAllocator>,
) -> Result<Vec<Arc<Framebuffer>>> {
    let depth_buffer = ImageView::new_default(Image::new(
        allocator,
        ImageCreateInfo {
            image_type: ImageType::Dim2d,
            format: Format::D16_UNORM,
            extent: images[0].extent(),
            usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
            ..Default::default()
        },
        AllocationCreateInfo::default(),
    )?)?;

    images
        .iter()
        .map(|image| {
            Ok(Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![ImageView::new_default(image.clone())?, depth_buffer.clone()],
                    ..Default::default()
                },
            )?)
        })
        .collect::<Result<Vec<Arc<Framebuffer>>>>()
}

fn create_graphics_pipeline(
    device: Arc<Device>,
    vsh: Arc<ShaderModule>,
    fsh: Arc<ShaderModule>,
    render_pass: Arc<RenderPass>,
    viewport: Viewport,
) -> Result<Arc<GraphicsPipeline>> {
    let vsh_entry = vsh.entry_point("main").unwrap();
    let fsh_entry = fsh.entry_point("main").unwrap();
    let vertex_input_state =
        vertex::CommonVertex::per_vertex().definition(&vsh_entry.info().input_interface)?;
    let stages = [
        PipelineShaderStageCreateInfo::new(vsh_entry),
        PipelineShaderStageCreateInfo::new(fsh_entry),
    ];
    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(device.clone())?,
    )?;
    let subpass = Subpass::from(render_pass.clone(), 0).unwrap();
    Ok(GraphicsPipeline::new(
        device.clone(),
        None,
        GraphicsPipelineCreateInfo {
            stages: stages.into_iter().collect(),
            vertex_input_state: Some(vertex_input_state),
            input_assembly_state: Some(InputAssemblyState::default()),
            #[allow(deprecated)]
            viewport_state: Some(ViewportState::viewport_fixed_scissor_irrelevant([viewport])),
            rasterization_state: Some(RasterizationState::default()),
            multisample_state: Some(MultisampleState::default()),
            color_blend_state: Some(ColorBlendState::with_attachment_states(
                subpass.num_color_attachments(),
                ColorBlendAttachmentState {
                    blend: Some(AttachmentBlend {
                        src_color_blend_factor: BlendFactor::SrcAlpha,
                        dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
                        color_blend_op: BlendOp::Add,

                        src_alpha_blend_factor: BlendFactor::One,
                        dst_alpha_blend_factor: BlendFactor::Zero,
                        alpha_blend_op: BlendOp::Add,
                    }),
                    ..Default::default()
                },
            )),
            depth_stencil_state: Some(DepthStencilState {
                depth: Some(DepthState::simple()),
                ..Default::default()
            }),
            subpass: Some(subpass.into()),
            ..GraphicsPipelineCreateInfo::layout(layout)
        },
    )?)
}

fn create_descriptor_set(
    index: usize,
    writes: impl IntoIterator<Item = WriteDescriptorSet>,
    pipeline: &Arc<GraphicsPipeline>,
    device: Arc<Device>,
) -> Result<Arc<PersistentDescriptorSet>> {
    let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device, Default::default());
    let pipeline_layout = pipeline.layout();
    let descriptor_set_layouts = pipeline_layout.set_layouts();

    let descriptor_set_layout = descriptor_set_layouts.get(index).unwrap();

    Ok(PersistentDescriptorSet::new(
        &descriptor_set_allocator,
        descriptor_set_layout.clone(),
        writes,
        [],
    )?)
}

fn begin_render_command_buffer(
    allocator: &StandardCommandBufferAllocator,
    queue: &Queue,
    framebuffer: Arc<Framebuffer>,
    clear_color: [f32; 4],
) -> Result<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer, StandardCommandBufferAllocator>> {
    let mut builder = AutoCommandBufferBuilder::primary(
        allocator,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )?;

    builder.begin_render_pass(
        RenderPassBeginInfo {
            clear_values: vec![Some(clear_color.into()), Some(1.0f32.into())],
            ..RenderPassBeginInfo::framebuffer(framebuffer)
        },
        SubpassBeginInfo {
            contents: SubpassContents::Inline,
            ..Default::default()
        },
    )?;
    Ok(builder)
}

type TargetApp = TA1LSD003;
fn main() -> Result<()> {
    let event_loop = EventLoop::new();

    // let size = PhysicalSize::new(9 * 128, 16 * 48);
    let size = TargetApp::INITIAL_SIZE;

    let window = Arc::new(
        WindowBuilder::new()
            // winit likes to change window sizes if we dont do this
            .with_max_inner_size(size)
            .with_min_inner_size(size)
            .with_inner_size(size)
            .build(&event_loop)?,
    );

    // initialise vulkan
    let library = VulkanLibrary::new()?;
    let required_extensions = Surface::required_extensions(&event_loop);
    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            enabled_extensions: required_extensions,
            ..Default::default()
        },
    )?;

    // make vulkan surface
    let surface = Surface::from_window(instance.clone(), window.clone())?;

    // get device and queue
    let required_device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    };
    let (physical_device, queue_family_index) =
        get_physical_device(&instance, &surface, &required_device_extensions);
    let (device, mut queues) = Device::new(
        physical_device.clone(),
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            enabled_extensions: required_device_extensions,
            ..Default::default()
        },
    )?;
    let queue = queues.next().unwrap();

    let surface_capabilities = physical_device
        .clone()
        .surface_capabilities(&surface, Default::default())?;

    let dimensions = window.inner_size();
    let composite_alpha = surface_capabilities
        .supported_composite_alpha
        .into_iter()
        .next()
        .unwrap();
    let image_format = physical_device.surface_formats(&surface, Default::default())?[0].0;
    let (mut swapchain, images) = Swapchain::new(
        device.clone(),
        surface.clone(),
        SwapchainCreateInfo {
            min_image_count: surface_capabilities.min_image_count + 1,
            image_format,
            present_mode: PresentMode::Immediate,
            image_extent: dimensions.into(),
            image_usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_SRC,
            composite_alpha,
            ..Default::default()
        },
    )?;

    // buffer/image allocator
    let allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

    // create render pass
    let recording_format = Format::R32G32B32A32_SFLOAT;
    let extent = images[0].extent();
    let render_pass = create_render_pass(device.clone(), recording_format)?;

    // create image for recorded output
    let recording_image = Image::new(
        allocator.clone(),
        ImageCreateInfo {
            format: recording_format,
            image_type: ImageType::Dim2d,
            usage: ImageUsage::TRANSFER_DST
                | ImageUsage::TRANSFER_SRC
                | ImageUsage::COLOR_ATTACHMENT,
            extent,
            ..Default::default()
        },
        AllocationCreateInfo::default(),
    )?;
    // create framebuffer for recording image
    let recording_framebuffer =
        create_framebuffers(&[recording_image.clone()], &render_pass, allocator.clone())?.remove(0);
    // create staging buffer for saving image
    let pixel_count = extent[0] as usize * extent[1] as usize * 4 as usize;
    let recording_staging_buffer = Buffer::from_iter(
        allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::HOST_RANDOM_ACCESS,
            ..Default::default()
        },
        (0..pixel_count).map(|_| 0.0f32),
    )?;
    // we cant render with unorm, but we also cant save with sfloat, so a conversion is required
    let mut unorm_buffer = vec![0u8; pixel_count];

    // create framebuffers for swapchain
    // let mut framebuffers = create_framebuffers(&images, &render_pass, allocator.clone())?;

    // create graphics pipelines
    let mut viewport = Viewport {
        offset: [0.0, 0.0],
        extent: window.inner_size().into(),
        depth_range: 0.0..=1.0,
    };

    // create command buffer allocator and terminal panel
    let command_buffer_allocator = StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo::default(),
    );
    let mut loader_command_buffer = AutoCommandBufferBuilder::primary(
        &command_buffer_allocator,
        queue_family_index,
        CommandBufferUsage::OneTimeSubmit,
    )?;

    let mut force_recreate_swapchain = false;
    let mut window_resized = false;

    let in_flight_frame_count = images.len();
    let mut image_fences: Vec<Option<Arc<FenceSignalFuture<_>>>> =
        vec![None; in_flight_frame_count];
    let mut previous_fence_index = 0;

    // let mut last_render_time = Instant::now();

    let mut app = TargetApp::new(
        &mut loader_command_buffer,
        allocator.clone(),
        device.clone(),
        render_pass.clone(),
        viewport.clone(),
    )?;

    drop(loader_command_buffer.build()?.execute(queue.clone())?);

    let mut frame_index = 0;
    let (mut ffmpeg, pixel_input) =
        ffmpeg_video_stream(extent[0] as usize, extent[1] as usize, "output.mp4").unwrap();
    let mut pixel_input = Some(pixel_input);

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => {
            window_resized = true;
        }
        Event::MainEventsCleared => {
            if force_recreate_swapchain || window_resized {
                force_recreate_swapchain = false;

                let new_size = window.inner_size();

                let (new_swapchain, new_images) = swapchain
                    .recreate(SwapchainCreateInfo {
                        image_extent: new_size.into(),
                        ..swapchain.create_info()
                    })
                    .unwrap();

                swapchain = new_swapchain;
                // framebuffers =
                //     create_framebuffers(&new_images, &render_pass, allocator.clone()).unwrap();

                if window_resized {
                    window_resized = false;

                    viewport.extent = new_size.into();

                    app.resize(new_size).unwrap();
                }
            }

            // update and send data to buffers
            let mut upload_command_buffer = AutoCommandBufferBuilder::primary(
                &command_buffer_allocator,
                queue_family_index,
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();

            app.update(&mut upload_command_buffer).unwrap();

            let upload_commands = upload_command_buffer.build().unwrap();
            sync::now(device.clone())
                .then_execute(queue.clone(), upload_commands)
                .unwrap()
                .then_signal_fence_and_flush()
                .unwrap()
                .wait(None)
                .unwrap();

            // acquire next swapchain image
            let (image_index, is_suboptimal, acquire_future) =
                match vulkano::swapchain::acquire_next_image(swapchain.clone(), None)
                    .map_err(Validated::unwrap)
                {
                    Result::Ok(good) => good,
                    Result::Err(VulkanError::OutOfDate) => {
                        force_recreate_swapchain = true;
                        return;
                    }
                    Result::Err(e) => {
                        panic!("vulkan error while acquiring next swapchain image: {e}")
                    }
                };

            // render everything
            let mut render_command_buffer = begin_render_command_buffer(
                &command_buffer_allocator,
                &queue,
                recording_framebuffer.clone(),
                // framebuffers[image_index as usize].clone(),
                [0.0, 0.0, 0.0, 1.0],
            )
            .unwrap();

            app.draw(&mut render_command_buffer).unwrap();

            render_command_buffer
                .end_render_pass(SubpassEndInfo::default())
                .unwrap();

            let render_commands = render_command_buffer.build().unwrap();

            // submit image
            if is_suboptimal {
                force_recreate_swapchain = true;
            }

            if let Some(image_fence) = &image_fences[image_index as usize] {
                image_fence.wait(None).unwrap();
            }

            let previous_fence = match image_fences[previous_fence_index as usize].clone() {
                None => {
                    let mut nothing = sync::now(device.clone());
                    nothing.cleanup_finished();
                    nothing.boxed()
                }
                Some(fence) => fence.boxed(),
            };

            let execution_result = sync::now(device.clone())
                .join(acquire_future)
                .join(previous_fence)
                .then_execute(queue.clone(), render_commands)
                .unwrap()
                .then_swapchain_present(
                    queue.clone(),
                    SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_index),
                )
                .then_signal_fence_and_flush();

            image_fences[image_index as usize] = match execution_result.map_err(Validated::unwrap) {
                Result::Ok(future) => Some(Arc::new(future)),
                Result::Err(VulkanError::OutOfDate) => {
                    force_recreate_swapchain = true;
                    None
                }
                Result::Err(err) => panic!("error while rendering: {err}"),
            };

            let mut copy_buffer = AutoCommandBufferBuilder::primary(
                &command_buffer_allocator,
                queue_family_index,
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();
            copy_buffer
                .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
                    recording_image.clone(),
                    recording_staging_buffer.clone(),
                ))
                .unwrap();
            let copy_commands = copy_buffer.build().unwrap();
            sync::now(device.clone())
                .then_execute(queue.clone(), copy_commands)
                .unwrap()
                .then_signal_fence_and_flush()
                .unwrap()
                .wait(None)
                .unwrap();

            for (dst, src) in unorm_buffer
                .iter_mut()
                .zip(recording_staging_buffer.read().unwrap().iter())
            {
                *dst = (src * 255.0) as u8;
            }

            pixel_input.as_mut().unwrap().write(&unorm_buffer).unwrap();
            println!("copied {frame_index}");

            // let mut output = RgbaImage::new(extent[0], extent[1]);
            // output.copy_from_slice(&unorm_buffer);
            // output.save(format!("output_{frame_index:06}.png")).unwrap();

            previous_fence_index = image_index;
            frame_index += 1;
            // let current_time = Instant::now();
            // println!(
            //     "{:?} FPS",
            //     1.0 / (current_time - last_render_time).as_secs_f64()
            // );
            // last_render_time = current_time;

            if app.done() {
                drop(pixel_input.take());
                ffmpeg.wait().unwrap();
                let (audio_file, audio_offset) = app.audio().unwrap();
                merge_av(audio_file, audio_offset, "output.mp4", "done.mp4").unwrap();
                control_flow.set_exit();
            }
        }
        _ => (),
    });
}
