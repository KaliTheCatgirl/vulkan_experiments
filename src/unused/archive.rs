
fn create_command_buffers(
    device: Arc<Device>,
    allocator: &StandardCommandBufferAllocator,
    queue: &Queue,
    pipeline: &Arc<GraphicsPipeline>,
    framebuffers: &[Arc<Framebuffer>],
    vertex_buffer: &Subbuffer<[vertex::CommonVertex]>,
    index_buffer: &Subbuffer<[u32]>,
    descriptor_writes: impl IntoIterator<Item = WriteDescriptorSet>,
) -> Result<Vec<Arc<PrimaryAutoCommandBuffer>>> {
    let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device, Default::default());
    let pipeline_layout = pipeline.layout();
    let descriptor_set_layouts = pipeline_layout.set_layouts();

    let descriptor_set_layout_index = 0;
    let descriptor_set_layout = descriptor_set_layouts
        .get(descriptor_set_layout_index)
        .unwrap();
    let descriptor_set = PersistentDescriptorSet::new(
        &descriptor_set_allocator,
        descriptor_set_layout.clone(),
        descriptor_writes,
        [],
    )?;

    framebuffers
        .iter()
        .map(|framebuffer| {
            let mut builder = AutoCommandBufferBuilder::primary(
                allocator,
                queue.queue_family_index(),
                CommandBufferUsage::MultipleSubmit,
            )?;
            builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values: vec![Some([0.5, 0.25, 0.4, 1.0].into())],
                        ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                    },
                    SubpassBeginInfo {
                        contents: SubpassContents::Inline,
                        ..Default::default()
                    },
                )?
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    0,
                    descriptor_set.clone(),
                )?
                .bind_pipeline_graphics(pipeline.clone())?
                .bind_vertex_buffers(0, vertex_buffer.clone())?
                .bind_index_buffer(index_buffer.clone())?
                .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)?
                .end_render_pass(SubpassEndInfo::default())?;
            Ok(builder.build()?)
        })
        .collect()
}
