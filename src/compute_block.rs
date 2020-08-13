use crate::compute_chain;

use downcast_rs::{impl_downcast, Downcast};

use std::rc::*;

pub trait ComputeBlock : Downcast {
    fn encode(&self, encoder: &mut wgpu::CommandEncoder);
    fn get_buffer(&self) -> &wgpu::Buffer;
}
impl_downcast!(ComputeBlock);


pub struct IntervalBlock {
    out_buffer: wgpu::Buffer,
    buffer_size: wgpu::BufferAddress,
    name: String,
    num_vertices: usize,
}

#[derive(Debug)]
pub struct IntervalBlockDescriptor {
    pub begin: f32,
    pub end: f32,
    pub quality: u32,
    pub name: String,
}

impl IntervalBlock {
    pub fn new(_compute_chain: &compute_chain::ComputeChain, device: &wgpu::Device, descriptor: &IntervalBlockDescriptor) -> Self {
        let num_vertices = 16 * descriptor.quality as usize;
        let mut interval_points = Vec::with_capacity(num_vertices);
        let delta = (descriptor.end - descriptor.begin) / (num_vertices - 1) as f32;
        for i in 0..num_vertices {
            interval_points.push(descriptor.begin + i as f32 * delta);
        }

        let out_buffer = device.create_buffer_with_data(
            bytemuck::cast_slice(&interval_points),
            wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE,
        );

        Self {
            out_buffer,
            num_vertices,
            buffer_size: (num_vertices * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
            name: descriptor.name.clone(),
        }
    }
}

impl ComputeBlock for IntervalBlock {
    fn encode(&self, _encoder: &mut wgpu::CommandEncoder) {
        // right now, do nothing
    }
    fn get_buffer(&self) -> &wgpu::Buffer {
        &self.out_buffer
    }
}


#[derive(Debug)]
pub struct CurveBlockDescriptor {
    pub interval_input_idx: u16,
    pub x_function: String,
    pub y_function: String,
    pub z_function: String,
}

pub struct CurveBlock {
    out_buffer: wgpu::Buffer,
    buffer_size: wgpu::BufferAddress,
    interval_input: Rc<IntervalBlock>,
    shader_module: wgpu::ShaderModule,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    num_vertices: usize,
}

impl CurveBlock {
    pub fn new(compute_chain: &compute_chain::ComputeChain, device: &wgpu::Device, descriptor: &CurveBlockDescriptor) -> Self {
        let interval_rc = compute_chain.blocks.get(&descriptor.interval_input_idx).expect("unable to find dependency").clone();
        let interval_input = interval_rc.downcast_rc::<IntervalBlock>().map_err(|_|"noped").unwrap();
        let num_vertices = interval_input.num_vertices;
        let input_buffer_size = interval_input.buffer_size as wgpu::BufferAddress;
        let output_buffer_size = input_buffer_size * 4;
        let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: output_buffer_size,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE,
        });

        let shader_source = format!(r##"
#version 450
layout(local_size_x = 16) in;

layout(set = 0, binding = 0) buffer InputBuffer {{
    float {}_buff[];
}};

layout(set = 0, binding = 1) buffer OutputBuffer {{
    vec4 out_buff[];
}};

void main() {{
    uint index = gl_GlobalInvocationID.x;
    float {} = {}_buff[index];
    out_buff[index].x = {};
    out_buff[index].y = {};
    out_buff[index].z = {};
    out_buff[index].w = 1;
}}
"##, "u", "u", "u", &descriptor.x_function, &descriptor.y_function, &descriptor.z_function);
        let mut shader_compiler = shaderc::Compiler::new().unwrap();
        let comp_spirv = shader_compiler.compile_into_spirv(&shader_source, shaderc::ShaderKind::Compute, "shader.comp", "main", None).unwrap();
        let comp_data = wgpu::read_spirv(std::io::Cursor::new(comp_spirv.as_binary_u8())).unwrap();
        let shader_module = device.create_shader_module(&comp_data);
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: false,
                        }
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: false,
                        }
                    }
                ],
                label: Some("ComputeShaderLayout")
            });
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &interval_input.out_buffer,
                        range: 0.. input_buffer_size,
                    },
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &out_buffer,
                        range: 0.. output_buffer_size,
                    }
                }
            ],
            label: Some("compute bind group"),
        });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[&compute_bind_group_layout]
            });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            layout: &compute_pipeline_layout,
            compute_stage: wgpu::ProgrammableStageDescriptor {
                entry_point: "main",
                module: &shader_module,
            }

        });

        Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            num_vertices,
            buffer_size: output_buffer_size,
            interval_input,
            shader_module,
        }
    }
}

impl ComputeBlock for CurveBlock {
    fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.dispatch((self.num_vertices/16) as u32, 1, 1);
    }

    fn get_buffer(&self) -> &wgpu::Buffer {
        &self.out_buffer
    }
}
