use crate::computable_scene::compute_block::BlockCreationError;

pub type CompilationResult = Result<(wgpu::ComputePipeline, wgpu::BindGroup), BlockCreationError>;

pub struct CustomBindDescriptor<'a> {
    pub position: u32,
    pub buffer: &'a wgpu::Buffer,
}

pub fn compile_compute_shader(
    device: &wgpu::Device,
    shader_src: &str,
    bindings: &[CustomBindDescriptor],
    globals_bind_layout: Option<&wgpu::BindGroupLayout>,
    label: Option<&str>,
    ) -> CompilationResult {
        let mut shader_compiler = shaderc::Compiler::new().ok_or(BlockCreationError::InternalError("unable to initialize shader compiler"))?;
        let comp_spirv = shader_compiler.compile_into_spirv(shader_src, shaderc::ShaderKind::Compute, "shader.comp", "main", None)
        .map_err(|error: shaderc::Error| {
            dbg!(&error);
            BlockCreationError::IncorrectAttributes(" check the expressions \n for errors ")
        })?;
        let comp_data = wgpu::util::make_spirv(comp_spirv.as_binary_u8());
        let shader_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor{
            label: None,
            source: comp_data,
            flags: wgpu::ShaderFlags::empty(), // TODO: maybe use VALIDATION flags
        });
        let mut layout_entries = Vec::<wgpu::BindGroupLayoutEntry>::new();
        let mut descriptor_entries = Vec::<wgpu::BindGroupEntry>::new();
        for binding in bindings {
            let position = binding.position;
            layout_entries.push(
                    wgpu::BindGroupLayoutEntry {
                        binding: position,
                        count: None,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            min_binding_size: None,
                            has_dynamic_offset: false,
                        }
                    });

            descriptor_entries.push(
                wgpu::BindGroupEntry {
                    binding: position,
                    resource: binding.buffer.as_entire_binding(),
                    });
        }
        dbg!(&layout_entries);
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &layout_entries,
                label
            });
        // the bind group layouts will be different, depending on the optional
        // globals_bind_layout, but everything else stays the same
        let bg_layouts_vec: Vec<&wgpu::BindGroupLayout> = if let Some(bind_layout) = globals_bind_layout {
            vec![&compute_bind_group_layout, &bind_layout]
        } else {
            vec![&compute_bind_group_layout]
        };
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &bg_layouts_vec,
                label,
                push_constant_ranges: &[],
            });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            layout: Some(&compute_pipeline_layout),
            label,
            module: &shader_module,
            entry_point: "main",
        });
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
            entries: &descriptor_entries,
            label,
        });
        Ok((compute_pipeline, compute_bind_group))
}

