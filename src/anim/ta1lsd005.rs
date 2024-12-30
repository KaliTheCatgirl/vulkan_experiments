use std::{f32::consts::PI, fs::File, sync::Arc, time::Duration};

use anyhow::Result;
use data::Pipelines;
use glam::{vec2, vec3, Mat4, Quat, Vec2, Vec3};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rodio::{OutputStream, OutputStreamHandle};
use vulkano::{command_buffer::{allocator::CommandBufferAllocator, AutoCommandBufferBuilder}, device::Device, image::ImageUsage, memory::allocator::MemoryAllocator, pipeline::graphics::viewport::Viewport, render_pass::RenderPass};
use winit::dpi::PhysicalSize;

use crate::renderer::{app::App, color, ext::CommandBufferExt, misc::SinkExtrapolator, termbuf::{self, TerminalPanel}};

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
        pub terminal_pipeline: Arc<GraphicsPipeline>,
    }
    impl Pipelines {
        pub fn new(
            device: Arc<Device>,
            render_pass: Arc<RenderPass>,
            viewport: Viewport,
        ) -> Result<Self> {
            Ok(Self {
                terminal_pipeline: create_graphics_pipeline(
                    device.clone(),
                    termbuf::shaders::vertex::load(device.clone())?,
                    termbuf::shaders::fragment::load(device)?,
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

pub struct TA1LSD005 {
    _output_stream: (OutputStream, OutputStreamHandle),
    audio: SinkExtrapolator,
    beat: f64,

    title: Vec<TerminalPanel>,
    ring: Vec<TerminalPanel>,

    device: Arc<Device>,

    pipelines: Arc<Pipelines>,
}
impl App for TA1LSD005 {
    fn new<L, A: CommandBufferAllocator + 'static>(
        loader_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
        allocator: Arc<dyn MemoryAllocator>,
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> Result<Self> {
        let (_, charset) = loader_command_buffer.load_image(
            "charset.png",
            allocator.clone(),
            ImageUsage::SAMPLED,
        )?;

        let title = "ta1lsd005"
            .bytes()
            .map(|i| {
                let panel = TerminalPanel::new(
                    1,
                    1,
                    loader_command_buffer,
                    allocator.clone(),
                    device.clone(),
                    Some(charset.clone()),
                    termbuf::PANEL_VERTICES.into_iter(),
                    termbuf::PANEL_INDICES.into_iter(),
                )?;
                panel.character_buffer.write()?[0] = i;
                panel.foreground_buffer.write()?[0] = color::WHITE;
                panel.background_buffer.write()?[0] = color::BLACK;
                Ok(panel)
            })
            .collect::<Result<Vec<TerminalPanel>>>()?;

        let ring = (0..16)
            .map(|_| {
                let mut panel = TerminalPanel::new(
                    50,
                    1,
                    loader_command_buffer,
                    allocator.clone(),
                    device.clone(),
                    Some(charset.clone()),
                    termbuf::PANEL_VERTICES.into_iter(),
                    termbuf::PANEL_INDICES.into_iter(),
                )?;
                panel.flat_transform(Vec3::ZERO, Quat::IDENTITY, vec2(0.09, 0.16));
                panel.fill_bg(color::BLACK)?;
                panel.fill_fg(color::WHITE)?;
                panel.fill_chars(0xb1)?;
                Ok(panel)
            })
            .collect::<Result<Vec<TerminalPanel>>>()?;

        let (_os, osh) = OutputStream::try_default()?;
        let sink = osh.play_once(File::open("ta1lsd005.mp3")?)?;
        let extrapolator = SinkExtrapolator::new(sink);
        let bpm = 115.0;
        let offset = 0.0134;
        extrapolator
            .sink
            .try_seek(Duration::from_secs_f32(30.0 * 60.0 / bpm + offset))
            .unwrap();

        Ok(Self {
            beat: 0.0,
            _output_stream: (_os, osh),
            audio: extrapolator,
            title,
            ring,
            device: device.clone(),
            pipelines: Arc::new(Pipelines::new(device, render_pass, viewport)?)
        })
    }
    fn update<L, A: CommandBufferAllocator>(
        &mut self,
        upload_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
    ) -> Result<()> {
        let secs = self.audio.get_pos().as_secs_f64();
        let bpm = 115.0;
        let offset = 0.0134;
        self.beat = (secs - offset) * bpm / 60.0;
        // let active_panel = self.beat as usize % 7;

        let mut rng = ChaCha8Rng::seed_from_u64(0);
        let title_length = self.title.len();
        let title_beat = self.beat as f32 - 31.0;
        for (index, panel) in self.title.iter_mut().enumerate() {
            panel.flat_transform(
                vec3(
                    (rng.gen::<f32>() - 0.5) * (title_beat as f32 * 2.0).powi(2) * 0.2
                        + (index as f32 - title_length as f32 / 2.0) * 0.18,
                    (rng.gen::<f32>() - 0.5) * (title_beat as f32 * 2.0).powi(2) * 0.2,
                    0.0,
                ),
                Quat::IDENTITY,
                vec2(0.09, 0.16),
            );
            panel.fill_fg(color::sinebow(
                index as f32 / title_length as f32 + self.beat as f32,
            ))?;
            panel.update(upload_command_buffer);
        }

        let ring_beat = self.beat as f32 - 32.0;
        let arm_count = self.ring.len();
        for (index, panel) in self.ring.iter().enumerate() {
            let speed = (index % 4 + 1) as f32;
            let mut color =
                color::sinebow(index as f32 / arm_count as f32 + ring_beat).map(|f| f * 0.5 + 0.5);
            color[3] = 1.0 - (ring_beat * speed / 4.0).rem_euclid(1.0);
            panel.fill_fg(color)?;
            for (ch_index, character) in panel.character_buffer.write()?.iter_mut().enumerate() {
                let ch = b"-\\|/"[((ring_beat * 8.0) as usize + index + ch_index) % 4];
                *character = ch;
            }
            panel.update(upload_command_buffer);
        }
        Ok(())
    }
    fn draw<L, A: CommandBufferAllocator>(
        &mut self,
        render_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
    ) -> Result<()> {
        let title_beat = self.beat as f32 - 31.0;

        render_command_buffer
            .bind_pipeline_graphics(self.pipelines.terminal_pipeline.clone())?;
        // let t = ((1.0 - (-beat).rem_euclid(1.0).powi(2)).sqrt() + beat.floor() + beat) / 2.0;
        // let eye = vec3(t.sin() as f32, 0.0, t.cos() as f32) * 3.0;
        // let target = vec3(0.0, 0.0, 0.0);

        let aspect = Mat4::from_scale(vec3(900.0 / 1600.0, 1.0, 1.0));

        let title_transform = aspect
            * Mat4::from_scale(
                Vec2::splat(2.0f32.powf((title_beat).powi(8) * 8.0) - title_beat * 0.5).extend(1.0),
            );
        if title_beat > 0.0 && title_beat < 1.0 {
            for panel in &self.title {
                panel
                    .draw(
                        render_command_buffer,
                        &self.pipelines.terminal_pipeline,
                        self.device.clone(),
                        title_transform,
                    )
                    .unwrap();
            }
        }

        let ring_beat = self.beat as f32 - 32.0;
        let arm_count = self.ring.len();
        if ring_beat > 0.0 {
            for (index, arm) in self.ring.iter().enumerate().rev() {
                let speed =
                    (index % arm_count + 4) as f32 * if index % 2 == 0 { -1.0 } else { 1.0 };
                arm.draw(
                    render_command_buffer,
                    &self.pipelines.terminal_pipeline,
                    self.device.clone(),
                    aspect * Mat4::from_rotation_z((ring_beat as f32 * speed / 16.0) * PI),
                )
                .unwrap();
            }
        }

        Ok(())
    }
    fn resize(&mut self, new_size: PhysicalSize<u32>) -> Result<()> {
        Ok(())
    }
}
