// fn brightness(t: f32) -> f32 {
//     if t > 0.0 && t < 1.0 {
//         return (1.5 - t * 2.0).max(0.75);
//     } else {
//         return 0.25;
//     }
// }
/*
pub struct Shattersong {
    _output_stream: (OutputStream, OutputStreamHandle),
    audio: SinkExtrapolator,
    beat: f64,
    sequence_7: Vec<TerminalPanel>,
}
impl App for Shattersong {
    fn new(
        loader_command_buffer: &mut impl CommandBufferExt,
        allocator: Arc<dyn MemoryAllocator>,
        device: Arc<Device>,
    ) -> Result<Self> {
        let (_, charset) = loader_command_buffer.load_image(
            "charset.png",
            allocator.clone(),
            ImageUsage::SAMPLED,
        )?;

        let sequence_7 = (0..7)
            .map(|i| {
                let angle = i as f32 * PI / 4.0;
                let mut panel = TerminalPanel::new(
                    1,
                    1,
                    loader_command_buffer,
                    allocator.clone(),
                    device.clone(),
                    Some(charset.clone()),
                    termbuf::PANEL_VERTICES.into_iter(),
                    termbuf::PANEL_INDICES.into_iter(),
                )?;
                panel.flat_transform(
                    vec3(angle.sin() * 0.75, -angle.cos() * 0.75, 0.0),
                    Quat::from_rotation_z(angle as f32),
                    vec2(0.125, 0.4),
                );
                panel.fill_bg([1.0; 4])?;
                panel.fill_fg([1.0; 4])?;
                Ok(panel)
            })
            .collect::<anyhow::Result<Vec<TerminalPanel>>>()?;

        let (_os, osh) = OutputStream::try_default()?;
        let sink = osh.play_once(File::open("shong.mp3")?)?;
        let extrapolator = SinkExtrapolator::new(sink);

        Ok(Self {
            beat: 0.0,
            _output_stream: (_os, osh),
            audio: extrapolator,
            sequence_7,
        })
    }
    fn update<L, A: CommandBufferAllocator>(
        &mut self,
        upload_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
    ) -> Result<()> {
        let secs = self.audio.get_pos().as_secs_f64();
        let bpm = 116.0;
        let offset = 0.514;
        self.beat = (secs - offset) * bpm / 60.0;
        // let active_panel = self.beat as usize % 7;

        let colors = [
            [0.275, 0.435, 0.761, 1.0],
            [0.275, 0.435, 0.761, 1.0],
            [0.761, 0.275, 0.537, 1.0],
            [0.761, 0.275, 0.537, 1.0],
            [0.788, 0.588, 0.286, 1.0],
            [0.788, 0.588, 0.286, 1.0],
            [0.263, 0.651, 0.427, 1.0],
        ];

        // let text = "shattersong";

        for (index, panel) in self.sequence_7.iter().enumerate() {
            let fac;
            if self.beat < 0.0 {
                fac = 0.1;
            } else {
                fac = brightness((self.beat - index as f64).rem_euclid(7.0) as f32);
            }
            let mut color = colors[index].map(|f| f * fac);
            color[3] = 1.0;

            panel.fill_bg(color)?;
            panel.fill_fg(color)?;

            panel.update(upload_command_buffer);
        }
        Ok(())
    }
    fn draw<L, A: CommandBufferAllocator>(
        &mut self,
        render_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
        device: Arc<Device>,
    ) -> Result<()> {
        render_command_buffer
            .bind_pipeline_graphics(graphics_pipelines.terminal_pipeline.clone())?;
        // let t = ((1.0 - (-beat).rem_euclid(1.0).powi(2)).sqrt() + beat.floor() + beat) / 2.0;
        // let eye = vec3(t.sin() as f32, 0.0, t.cos() as f32) * 3.0;
        // let target = vec3(0.0, 0.0, 0.0);

        for panel in &self.sequence_7 {
            panel
                .draw(
                    render_command_buffer,
                    &graphics_pipelines.terminal_pipeline,
                    device.clone(),
                    Mat4::from_scale(vec3(900.0 / 1600.0, 1.0, 1.0))
                        * Mat4::from_rotation_z(self.beat.max(0.0) as f32 * PI / 28.0), // Mat4::from_scale(vec3(1.0, -1.0, 1.0))
                                                                                        //     * Mat4::perspective_rh(PI / 2.0, 1600.0 / 900.0, 0.01, 100.0)
                                                                                        //     * Mat4::look_at_rh(eye, target, Vec3::Y),
                )
                .unwrap();
        }

        Ok(())
    }
}

struct AudioTimer {
    current_duration: Duration,
}
enum AudioTimeBase {
    AudioBased(SinkExtrapolator),
    FixedTimestep(Duration),
}

pub struct M3NMIN3 {
    background: TerminalPanel,
    title: Vec<TerminalPanel>,
    madewith: TerminalPanel,
    songby: TerminalPanel,
}
impl App for M3NMIN3 {
    fn new(
        loader_command_buffer: &mut impl CommandBufferExt,
        allocator: Arc<dyn MemoryAllocator>,
        device: Arc<Device>,
    ) -> Result<Self> {
        let (_, charset) = loader_command_buffer.load_image(
            "charset.png",
            allocator.clone(),
            ImageUsage::SAMPLED,
        )?;

        let title = (0..7)
            .map(|i| {
                let mut panel = TerminalPanel::new(
                    3,
                    1,
                    loader_command_buffer,
                    allocator.clone(),
                    device.clone(),
                    Some(charset.clone()),
                    termbuf::PANEL_VERTICES.into_iter(),
                    termbuf::PANEL_INDICES.into_iter(),
                )?;

                panel.character_buffer.write()?.copy_from_slice(b"cve");
                panel.fill_bg([0.0; 4])?;
                panel.flat_transform(
                    vec3(
                        i as f32 * 0.16 / 1.28 / 9.0,
                        -i as f32 * 0.16 / 0.48 / 16.0,
                        (i + 1) as f32 * -0.01 + 0.1,
                    ),
                    Quat::IDENTITY,
                    vec2(0.16 / 1.28, 0.16 / 0.48),
                );

                Ok(panel)
            })
            .rev()
            .collect::<Result<Vec<TerminalPanel>>>()?;

        let mut background = TerminalPanel::new(
            128,
            64,
            loader_command_buffer,
            allocator.clone(),
            device.clone(),
            Some(charset.clone()),
            termbuf::PANEL_VERTICES.into_iter(),
            termbuf::PANEL_INDICES.into_iter(),
        )?;
        background.flat_transform(vec3(0.0, 0.0, 0.1), Quat::IDENTITY, 2.0 / vec2(128.0, 64.0));

        let madewith_text = "background made with vulkan";
        let mut madewith = TerminalPanel::new(
            madewith_text.len() as u32,
            1,
            loader_command_buffer,
            allocator.clone(),
            device.clone(),
            Some(charset.clone()),
            termbuf::PANEL_VERTICES.into_iter(),
            termbuf::PANEL_INDICES.into_iter(),
        )?;
        madewith.print(0, 0, madewith_text, Some([1.0; 4]), Some([0.0; 4]))?;
        madewith.flat_transform(
            vec3(0.0, -0.25, 0.0),
            Quat::IDENTITY,
            vec2(0.16 / 1.28, 0.16 / 0.48) * 0.25,
        );

        let songby_text = "song by kalithecatgirl";
        let mut songby = TerminalPanel::new(
            songby_text.len() as u32,
            1,
            loader_command_buffer,
            allocator,
            device,
            Some(charset),
            termbuf::PANEL_VERTICES.into_iter(),
            termbuf::PANEL_INDICES.into_iter(),
        )?;
        songby.print(0, 0, songby_text, Some([1.0; 4]), Some([0.0; 4]))?;
        songby.flat_transform(
            vec3(0.0, 0.15, 0.0),
            Quat::IDENTITY,
            vec2(0.16 / 1.28, 0.16 / 0.48) * 0.25,
        );

        Ok(Self {
            background,
            title,
            madewith,
            songby,
        })
    }
    fn update<L, A: CommandBufferAllocator>(
        &mut self,
        upload_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
    ) -> Result<()> {

        thread_rng().fill(&mut *self.background.character_buffer.write()?);
        for (fg, bg) in self
            .background
            .foreground_buffer
            .write()?
            .iter_mut()
            .zip(self.background.background_buffer.write()?.iter_mut())
        {
            *fg = thread_rng().gen::<[f32; 4]>().map(|f| f - 0.5);
            fg[3] = 1.0;
            *bg = thread_rng().gen::<[f32; 4]>().map(|f| f - 0.5);
            bg[3] = 1.0;
        }
        self.background.update(upload_command_buffer);

        Ok(())
    }
    fn draw<L, A: CommandBufferAllocator>(
        &mut self,
        render_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
        graphics_pipelines: &GraphicsPipelines,
        device: Arc<Device>,
    ) -> Result<()> {
        render_command_buffer
            .bind_pipeline_graphics(graphics_pipelines.terminal_pipeline.clone())?;

        self.background.draw(
            render_command_buffer,
            &graphics_pipelines.terminal_pipeline,
            device.clone(),
            Mat4::IDENTITY,
        )?;

        Ok(())
    }
}

pub struct CVESong {
    background: TerminalPanel,
    title: Vec<TerminalPanel>,
    madewith: TerminalPanel,
    songby: TerminalPanel,
}
impl App for CVESong {
    fn new(
        loader_command_buffer: &mut impl CommandBufferExt,
        allocator: Arc<dyn MemoryAllocator>,
        device: Arc<Device>,
    ) -> Result<Self> {
        let (_, charset) = loader_command_buffer.load_image(
            "charset.png",
            allocator.clone(),
            ImageUsage::SAMPLED,
        )?;

        let title = (0..7)
            .map(|i| {
                let mut panel = TerminalPanel::new(
                    3,
                    1,
                    loader_command_buffer,
                    allocator.clone(),
                    device.clone(),
                    Some(charset.clone()),
                    termbuf::PANEL_VERTICES.into_iter(),
                    termbuf::PANEL_INDICES.into_iter(),
                )?;

                panel.character_buffer.write()?.copy_from_slice(b"cve");
                panel.fill_bg([0.0; 4])?;
                panel.flat_transform(
                    vec3(
                        i as f32 * 0.16 / 1.28 / 9.0,
                        -i as f32 * 0.16 / 0.48 / 16.0,
                        (i + 1) as f32 * -0.01 + 0.1,
                    ),
                    Quat::IDENTITY,
                    vec2(0.16 / 1.28, 0.16 / 0.48),
                );

                Ok(panel)
            })
            .rev()
            .collect::<Result<Vec<TerminalPanel>>>()?;

        let mut background = TerminalPanel::new(
            128,
            64,
            loader_command_buffer,
            allocator.clone(),
            device.clone(),
            Some(charset.clone()),
            termbuf::PANEL_VERTICES.into_iter(),
            termbuf::PANEL_INDICES.into_iter(),
        )?;
        background.flat_transform(vec3(0.0, 0.0, 0.1), Quat::IDENTITY, 2.0 / vec2(128.0, 64.0));

        let madewith_text = "background made with vulkan";
        let mut madewith = TerminalPanel::new(
            madewith_text.len() as u32,
            1,
            loader_command_buffer,
            allocator.clone(),
            device.clone(),
            Some(charset.clone()),
            termbuf::PANEL_VERTICES.into_iter(),
            termbuf::PANEL_INDICES.into_iter(),
        )?;
        madewith.print(0, 0, madewith_text, Some([1.0; 4]), Some([0.0; 4]))?;
        madewith.flat_transform(
            vec3(0.0, -0.25, 0.0),
            Quat::IDENTITY,
            vec2(0.16 / 1.28, 0.16 / 0.48) * 0.25,
        );

        let songby_text = "song by kalithecatgirl";
        let mut songby = TerminalPanel::new(
            songby_text.len() as u32,
            1,
            loader_command_buffer,
            allocator,
            device,
            Some(charset),
            termbuf::PANEL_VERTICES.into_iter(),
            termbuf::PANEL_INDICES.into_iter(),
        )?;
        songby.print(0, 0, songby_text, Some([1.0; 4]), Some([0.0; 4]))?;
        songby.flat_transform(
            vec3(0.0, 0.15, 0.0),
            Quat::IDENTITY,
            vec2(0.16 / 1.28, 0.16 / 0.48) * 0.25,
        );

        Ok(Self {
            background,
            title,
            madewith,
            songby,
        })
    }
    fn update<L, A: CommandBufferAllocator>(
        &mut self,
        upload_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
    ) -> Result<()> {
        thread_rng().fill(&mut *self.background.character_buffer.write()?);
        for (fg, bg) in self
            .background
            .foreground_buffer
            .write()?
            .iter_mut()
            .zip(self.background.background_buffer.write()?.iter_mut())
        {
            *fg = thread_rng().gen::<[f32; 4]>().map(|f| f - 0.5);
            fg[3] = 1.0;
            *bg = thread_rng().gen::<[f32; 4]>().map(|f| f - 0.5);
            bg[3] = 1.0;
        }
        for (i, panel) in self.title.iter().enumerate() {
            panel.fill_fg(color::sinebow(i as f32 / 8.0).map(|f| f * 1.25 - 0.25))?;
            panel.update(upload_command_buffer);
        }
        self.background.update(upload_command_buffer);
        Ok(())
    }
    fn draw<L, A: CommandBufferAllocator>(
        &mut self,
        render_command_buffer: &mut AutoCommandBufferBuilder<L, A>,
        graphics_pipelines: &GraphicsPipelines,
        device: Arc<Device>,
    ) -> Result<()> {
        render_command_buffer
            .bind_pipeline_graphics(graphics_pipelines.terminal_pipeline.clone())?;

        self.background.draw(
            render_command_buffer,
            &graphics_pipelines.terminal_pipeline,
            device.clone(),
            Mat4::IDENTITY,
        )?;

        self.madewith.draw(
            render_command_buffer,
            &graphics_pipelines.terminal_pipeline,
            device.clone(),
            Mat4::IDENTITY,
        )?;
        self.songby.draw(
            render_command_buffer,
            &graphics_pipelines.terminal_pipeline,
            device.clone(),
            Mat4::IDENTITY,
        )?;

        for panel in &self.title {
            panel.draw(
                render_command_buffer,
                &graphics_pipelines.terminal_pipeline,
                device.clone(),
                Mat4::IDENTITY,
            )?;
        }

        Ok(())
    }
}
*/
