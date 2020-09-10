
pub struct CustomBindDescriptor<'a> {
    pub position: u32,
    pub buffer_slice: wgpu::BufferSlice<'a>,
}

pub fn compute_shader_from_glsl(
    shader_src: &str,
    bindings: &[CustomBindDescriptor],
    globals_bind_layout: &wgpu::BindGroupLayout,
    device: &wgpu::Device,
    label: Option<&str>,
    ) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
        let mut shader_compiler = shaderc::Compiler::new().unwrap();
        let comp_spirv = shader_compiler.compile_into_spirv(shader_src, shaderc::ShaderKind::Compute, "shader.comp", "main", None).unwrap();
        let comp_data = wgpu::util::make_spirv(comp_spirv.as_binary_u8());
        let shader_module = device.create_shader_module(comp_data);
        let mut layout_entries = Vec::<wgpu::BindGroupLayoutEntry>::new();
        let mut descriptor_entries = Vec::<wgpu::BindGroupEntry>::new();
        for binding in bindings {
            let position = binding.position;
            let buffer_slice = binding.buffer_slice;
            layout_entries.push(
                    wgpu::BindGroupLayoutEntry {
                        binding: position,
                        count: None,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            min_binding_size: None,
                            dynamic: false,
                            readonly: false,
                        }
                    });

            descriptor_entries.push(
                wgpu::BindGroupEntry {
                    binding: position,
                    resource: wgpu::BindingResource::Buffer (buffer_slice),
                    });
        }
        dbg!(&layout_entries);
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &layout_entries,
                label
            });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[&compute_bind_group_layout, &globals_bind_layout],
                label,
                push_constant_ranges: &[],
            });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            layout: Some(&compute_pipeline_layout),
            label,
            compute_stage: wgpu::ProgrammableStageDescriptor {
                entry_point: "main",
                module: &shader_module,
            }
        });
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
            entries: &descriptor_entries,
            label,
        });
        (compute_pipeline, compute_bind_group)
}
