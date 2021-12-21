pub struct CustomBindDescriptor<'a> {
    pub position: u32,
    pub buffer: &'a wgpu::Buffer,
}

pub struct BindInfo<'a> {
    pub ty: wgpu::BufferBindingType,
    pub buffer: &'a wgpu::Buffer,
}

pub fn compile_graphics_shader(
    _device: &wgpu::Device,
    _shader_src: &str,
    ) -> wgpu::ShaderModule {
    todo!()
}

pub fn naga_compute_pipeline(device: &wgpu::Device, wgsl_source: &str, bindings: &[BindInfo]) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
    // first, compile the wgsl shader
    let wgsl_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: Some("compute shader module"),
        source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
    });

    // then we need to define the pipeline layout and bind group,
    // which in turn requires the bind group layout
    // we compute all of it by processing the BindInfo range we are given as input
    let mut layout_entries = Vec::<wgpu::BindGroupLayoutEntry>::new();
    let mut descriptor_entries = Vec::<wgpu::BindGroupEntry>::new();
    for (position, info) in bindings.iter().enumerate() {
        layout_entries.push(
                wgpu::BindGroupLayoutEntry {
                    binding: position as u32,
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: info.ty,
                        min_binding_size: None,
                        has_dynamic_offset: false,
                    }
                });

        descriptor_entries.push(
            wgpu::BindGroupEntry {
                binding: position as u32,
                resource: info.buffer.as_entire_binding(),
                });
    }

    let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &layout_entries,
        label: None,
    });
    let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&compute_bind_group_layout],
        label: None,
        push_constant_ranges: &[],
    });

    let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &compute_bind_group_layout,
        entries: &descriptor_entries,
        label: None,
    });
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
        label: None,
        layout: Some(&compute_pipeline_layout),
        module: &wgsl_module,
        entry_point: "main",
    });
    (compute_pipeline, compute_bind_group)
}

pub fn create_bind_groups(
    device: &wgpu::Device,
    pipeline: &wgpu::ComputePipeline,
    groups: &[Vec<&wgpu::Buffer>],
    ) -> Vec<wgpu::BindGroup> {
        let mut bind_groups = Vec::<wgpu::BindGroup>::new();
        for (group_id, group) in groups.iter().enumerate() {
            // for each group, we go through all the buffers that belong to that group and create
            // a BindGroupEntry for it.
            let mut descriptor_entries = Vec::<wgpu::BindGroupEntry>::new();
            for (location, buffer) in group.iter().enumerate() {
                descriptor_entries.push(
                    wgpu::BindGroupEntry {
                        binding: location as u32,
                        resource: buffer.as_entire_binding(),
                });
            }
            // then we recover the layout that naga deduced from the wgsl shader
            // and let the device create a bind group combining the two things.
            dbg!(pipeline.get_bind_group_layout(group_id as u32));
            bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &pipeline.get_bind_group_layout(group_id as u32),
                entries: &descriptor_entries,
                label: None,
            }));
        }
        bind_groups
}

