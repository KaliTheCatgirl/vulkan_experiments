use std::{
    f32::consts::{PI as PI32, TAU as TAU32},
    f64::consts::PI as PI64,
    fs::File,
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use data::Pipelines;
use glam::{vec2, vec3, Mat4, Quat, Vec2, Vec3, Vec4};
use rand::{thread_rng, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rodio::{OutputStream, OutputStreamHandle};
use vulkano::{
    command_buffer::{allocator::CommandBufferAllocator, AutoCommandBufferBuilder},
    device::Device,
    image::ImageUsage,
    memory::allocator::MemoryAllocator,
    pipeline::graphics::viewport::Viewport,
    render_pass::RenderPass,
};
use winit::dpi::PhysicalSize;

use crate::renderer::{
    app::App,
    color::{self, Color},
    ext::CommandBufferExt,
    misc::{self, SinkExtrapolator},
    termbuf::{self, TerminalPanel},
};

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

    use crate::{
        create_graphics_pipeline,
        renderer::{ext::CommandBufferExt, mesh},
    };

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

fn second_drop_write(to: &mut [u8], beat: f64) {
    to.fill(0);
    const DATA: [&'static str; 16] = [
        "ALL", "ALL\nTHE", "ALL\nTHE\nTHINGS", "ALL\nTHE\nTHINGS\nSHE",
        "THINGS", "THINGS\nSHE", "THINGS\nSHE\nSAID", "THINGS\nSHE\nSAID",
        "\nALL", "\nALL", "\nALL\nRUN", "\nALL\nRUNNING",
        "\nRUN", "\nRUNNING", "\nRUNNING\nTHROUGH", "\nRUNNING\nTHROUGH\nMY",
    ];

    let width = 12;

    let index = (beat / 2.0 % 4.0) as usize * 4 + (beat % 1.0 * 4.0) as usize;
    let start_x = 3;
    let start_y = 1;
    let mut x = start_x;
    let mut y = start_y;
    for ch in DATA[index].bytes() {
        if ch == b'\n' {
            x = start_x;
            y += 1;
        } else {
            to[x + y * width] = ch;
            x += 1;
        }
    }
}

enum TimeBase {
    RealTime(Duration),
    Audio(SinkExtrapolator),
}
impl TimeBase {
    pub fn rt_step(&mut self, delta: Duration) {
        if let TimeBase::RealTime(s) = self {
            *s += delta;
        }
    }
    pub fn get_pos(&mut self) -> Duration {
        match self {
            Self::RealTime(duration) => *duration,
            Self::Audio(audio) => audio.get_pos(),
        }
    }
}

pub struct TA1LSD003 {
    _output_stream: (OutputStream, OutputStreamHandle),
    time: TimeBase,
    beat: f64,
    bpm: f64,
    offset: f64,
    start: f64,

    panel: TerminalPanel,
    tunnel: TerminalPanel,
    tunnel_words: TerminalPanel,

    device: Arc<Device>,

    pipelines: Arc<Pipelines>,
}
impl App for TA1LSD003 {
    const INITIAL_SIZE: PhysicalSize<u32> = PhysicalSize::new(9 * 128, 16 * 64);

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

        let panel = TerminalPanel::new(
            64,
            32,
            loader_command_buffer,
            allocator.clone(),
            device.clone(),
            Some(charset.clone()),
            termbuf::PANEL_VERTICES.into_iter(),
            termbuf::PANEL_INDICES.into_iter(),
        )?;

        let tunnel_segments = 8;
        let tunnel_depth = 40;
        let (mut tunnel_vertices, tunnel_indices) =
            misc::generate_quad_plane(tunnel_segments, tunnel_depth);
        for vertex in tunnel_vertices.iter_mut() {
            vertex.position[1] = ((vertex.position[0] + vertex.position[2] * 0.025) * TAU32
                / tunnel_segments as f32)
                .cos()
                * (vertex.position[2] + 20.0)
                / 40.0;
            vertex.position[0] = ((vertex.position[0] + vertex.position[2] * 0.025) * TAU32
                / tunnel_segments as f32)
                .sin()
                * (vertex.position[2] + 20.0)
                / 40.0;
        }

        let tunnel = TerminalPanel::new(
            tunnel_segments,
            tunnel_depth,
            loader_command_buffer,
            allocator.clone(),
            device.clone(),
            Some(charset.clone()),
            tunnel_vertices.into_iter(),
            tunnel_indices.into_iter(),
        )?;
        let tunnel_words = TerminalPanel::new(
            12,
            6,
            loader_command_buffer,
            allocator.clone(),
            device.clone(),
            Some(charset.clone()),
            termbuf::PANEL_VERTICES.into_iter(),
            termbuf::PANEL_INDICES.into_iter(),
        )?;

        let bpm = 100.0;
        let offset = 0.0134;
        let first_drop = 28.0;
        let second_drop = 31.9;

        let start = second_drop * 60.0 / bpm + offset;
        println!("{start}");

        let (_os, osh) = OutputStream::try_default()?;
        // let sink = osh.play_once(File::open("ta1lsd003.mp3")?)?;
        // let extrapolator = SinkExtrapolator::new(sink);
        // extrapolator
        //     .sink
        //     .try_seek(Duration::from_secs_f64(start))
        //     .unwrap();

        Ok(Self {
            beat: 0.0,
            _output_stream: (_os, osh),
            time: TimeBase::RealTime(Duration::from_secs_f64(start)),//extrapolator,
            panel,
            tunnel,
            tunnel_words,
            bpm,
            offset,
            start,
            device: device.clone(),
            pipelines: Arc::new(Pipelines::new(device, render_pass, viewport)?),
        })
    }
    fn update<L, A: CommandBufferAllocator>(
        &mut self,
        upload_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
    ) -> Result<()> {
        let secs = self.time.get_pos().as_secs_f64();
        self.beat = (secs - self.offset) * self.bpm / 60.0;

        let text = "   TA1LSD003   ";
        let text_length = text.len();

        let width = self.panel.width() as usize;
        let height = self.panel.height() as usize;
        let beat = self.beat;

        if beat < 64.0 {
            let mut bg = self.panel.background_buffer.write()?;
            let mut fg = self.panel.foreground_buffer.write()?;
            let mut ch = self.panel.character_buffer.write()?;

            for i in 0..width * height {
                let x = i % width;
                let y = i / width;
                let nx = (x as f64 / width as f64 * 2.0) - 1.0;
                let ny = (y as f64 / height as f64 * 2.0) - 1.0;

                if beat < 32.0 {
                    bg[i] = color::BLACK;
                    fg[i] = color::BLACK;
                } else if beat < 63.0 {
                    let v = (x as f64 / 5.0 + beat * 3.0).sin()
                        * (y as f64 / 5.0 + beat * 2.0).sin()
                        * (beat * PI64).sin()
                        * 0.5
                        + 0.5;

                    ch[i] = text.as_bytes()
                        [((v * text_length as f64).max(0.0) as usize).min(text_length - 1)];

                    bg[i] = color::sinebow(((nx * nx + ny * ny) / 2.0 - beat * 10.0) as f32);
                    bg[i][0] *= 2.0 * (1.0 - beat as f32 * 8.0 % 1.0);
                    bg[i][1] *= 2.0 * (1.0 - (beat as f32 * 8.0 + 2.0 / 3.0) % 1.0);
                    bg[i][2] *= 2.0 * (1.0 - (beat as f32 * 8.0 + 1.0 / 3.0) % 1.0);

                    let xfac = beat / 4.0 % 4.0 + 1.0;
                    let yfac = (beat / 4.0 + 1.0) % 4.0 + 1.0;

                    let beat_m2 = beat % 2.0;

                    if beat_m2 > 1.1 && beat_m2 < 1.25 {
                        ch[i] = 0;
                        bg[i] = color::BLACK;
                    } else {
                        let bw = beat_m2 > 1.0 && beat_m2 < 1.25;
                        let flip = ((x as f64 * xfac + (beat * 32.0).sin() * 64.0 + 64.0) as usize
                            + (y as f64 * yfac + (beat * 32.0).cos() * 64.0 + 64.0) as usize)
                            % 2
                            == 0;
                        if bw {
                            ch[i] = 0;
                            bg[i] = if flip { color::WHITE } else { color::BLACK };
                        }
                        if flip {
                            bg[i] = color::hueflip(bg[i]);
                        }
                        fg[i] = color::randcolor();
                    }
                } else {
                    ch[i] = 0;
                    bg[i] = color::BLACK;
                    fg[i] = color::BLACK;
                }
            }
            self.panel.update(upload_command_buffer);
        } else {
            self.tunnel_words.fill_bg(color::TRANSPARENT)?;
            self.tunnel_words.fill_fg(if beat % 0.075 > 0.0375 { color::WHITE } else { color::BLACK })?;
            second_drop_write(&mut self.tunnel_words.character_buffer.write()?, self.beat);
            self.tunnel_words.update(upload_command_buffer);

            let mut bg = self.tunnel.foreground_buffer.write()?;
            let mut fg = self.tunnel.background_buffer.write()?;
            let mut ch = self.tunnel.character_buffer.write()?;
            for i in 0..self.tunnel.width() as usize * self.tunnel.height() as usize {
                bg[i] = Vec4::new(
                    thread_rng().gen(),
                    thread_rng().gen(),
                    thread_rng().gen(),
                    1.0,
                );
                fg[i] = Vec4::new(
                    thread_rng().gen(),
                    thread_rng().gen(),
                    thread_rng().gen(),
                    1.0,
                );
                ch[i] = thread_rng().gen();
            }
            self.tunnel.update(upload_command_buffer);
        }

        self.time.rt_step(Duration::from_secs_f64(1.0 / 60.0));
        println!("{}", self.beat);

        Ok(())
    }
    fn draw<L, A: CommandBufferAllocator>(
        &mut self,
        render_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
    ) -> Result<()> {
        let aspect = Self::INITIAL_SIZE.width as f32 / Self::INITIAL_SIZE.height as f32;
        render_command_buffer.bind_pipeline_graphics(self.pipelines.terminal_pipeline.clone())?;
        let transform = Mat4::perspective_lh(PI32 / 2.0, aspect, 0.01, 100.0)
            * Mat4::look_at_lh(Vec3::Z * 20.0, Vec3::ZERO, Vec3::Y);

        if (32.0..63.25).contains(&self.beat) {
            self.panel.draw(
                render_command_buffer,
                &self.pipelines.terminal_pipeline,
                self.device.clone(),
                Mat4::IDENTITY,
            )?;
        }
        if (160.0..223.5).contains(&self.beat) && self.beat % 1.0 < 0.8125 && !(191.0..192.0).contains(&self.beat) {
            self.tunnel_words.draw(
                render_command_buffer,
                &self.pipelines.terminal_pipeline,
                self.device.clone(),
                Mat4::from_translation(vec3(thread_rng().gen_range(-0.02..0.02), thread_rng().gen_range(-0.02..0.02), 0.0)),
            )?;
            self.tunnel.draw(
                render_command_buffer,
                &self.pipelines.terminal_pipeline,
                self.device.clone(),
                transform
                    * Mat4::from_translation(vec3(0.0, 0.0, (self.beat % 1.0 * 35.0) as f32))
                    * Mat4::from_rotation_z(self.beat as f32 * 10.0),
            )?;
        }
        Ok(())
    }
    fn resize(&mut self, new_size: PhysicalSize<u32>) -> Result<()> {
        Ok(())
    }
    fn done(&self) -> bool {
        self.beat > 64.0
    }
    fn audio(&self) -> Option<(&'static str, f64)> {
        Some(("ta1lsd003.mp3", self.start))
    }
}
