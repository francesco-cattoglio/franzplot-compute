use crate::compute_chain::ComputeChain;

use ultraviolet::Vec3u;

const LOCAL_SIZE_X: u32 = 16;
const _LOCAL_SIZE_Y: u32 = 16;

pub enum ComputeBlock {
    Interval(IntervalData),
    Curve(CurveData),
}

impl ComputeBlock {
    pub fn get_buffer(&self) -> &wgpu::Buffer {
        match self {
            Self::Interval(data) => &data.out_buffer,
            Self::Curve(data) => &data.out_buffer,
        }
    }

    pub fn encode(&self, globals_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
        match self {
            Self::Interval(data) => data.encode(globals_bind_group, encoder),
            Self::Curve(data) => data.encode(globals_bind_group, encoder),
        }
    }
}

#[derive(Debug)]
pub struct BlockDescriptor {
    pub id: String,
    pub data: DescriptorData,
}

#[derive(Debug)]
pub enum DescriptorData {
    Curve (CurveBlockDescriptor),
    Interval (IntervalBlockDescriptor),
}

#[derive(Debug)]
pub struct IntervalBlockDescriptor {
    pub begin: String,
    pub end: String,
    pub quality: u32,
    pub name: String,
}
impl IntervalBlockDescriptor {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> ComputeBlock {
        ComputeBlock::Interval(IntervalData::new(chain, device, &self))
    }
}

pub struct IntervalData {
    out_buffer: wgpu::Buffer,
    buffer_size: wgpu::BufferAddress,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    out_sizes: Vec3u,
    name: String,
    #[allow(unused)]
    shader_module: wgpu::ShaderModule,
}

impl IntervalData {
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &IntervalBlockDescriptor) -> Self {
        let out_sizes = Vec3u::new(16 * descriptor.quality, 1, 1);
        let buffer_size = (out_sizes.x * std::mem::size_of::<f32>() as u32) as wgpu::BufferAddress;
        let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_size,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE,
        });

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

layout(set = 0, binding = 0) buffer OutputBuffer {{
    float out_buff[];
}};

{globals_header}

void main() {{
    uint index = gl_GlobalInvocationID.x;
    float delta = ({interval_end} - {interval_begin}) / ({num_points} - 1.0);
    out_buff[index] = {interval_begin} + delta * index;
}}
"##, globals_header=&compute_chain.shader_header, interval_begin=&descriptor.begin, interval_end=&descriptor.end, num_points=out_sizes.x, dimx=LOCAL_SIZE_X, dimy=1
);
        println!("debug info for interval shader: \n{}", shader_source);

        let mut shader_compiler = shaderc::Compiler::new().unwrap();
        let comp_spirv = shader_compiler.compile_into_spirv(&shader_source, shaderc::ShaderKind::Compute, "shader.comp", "main", None).unwrap();
        let comp_data = wgpu::util::make_spirv(comp_spirv.as_binary_u8());
        let shader_module = device.create_shader_module(comp_data);
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        count: None,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            min_binding_size: None,
                            readonly: false,
                        }
                    },
                ],
                label: Some("IntervalShaderBindLayout")
            });
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer (
                        out_buffer.slice(..),
                    ),
                },
            ],
            label: Some("IntervalShaderBindGroup"),
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[&compute_bind_group_layout, &compute_chain.globals_bind_layout],
                label: None,
                push_constant_ranges: &[],
            });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            layout: Some(&compute_pipeline_layout),
            label: None,
            compute_stage: wgpu::ProgrammableStageDescriptor {
                entry_point: "main",
                module: &shader_module,
            }
        });

        Self {
            shader_module,
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_sizes,
            buffer_size,
            name: descriptor.name.clone(),
        }
    }

    fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            compute_pass.dispatch((self.out_sizes.x/LOCAL_SIZE_X) as u32, 1, 1);
    }
}


#[derive(Debug)]
pub struct CurveBlockDescriptor {
    pub interval_input_id: String,
    pub x_function: String,
    pub y_function: String,
    pub z_function: String,
}
impl CurveBlockDescriptor {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> ComputeBlock {
        ComputeBlock::Curve(CurveData::new(chain, device, &self))
    }
}

pub struct CurveData {
    out_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    out_sizes: Vec3u,
    #[allow(unused)]
    shader_module: wgpu::ShaderModule,
    buffer_size: wgpu::BufferAddress,
}

impl CurveData {
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &CurveBlockDescriptor) -> Self {
        let interval_block = compute_chain.chain.get(&descriptor.interval_input_id).expect("unable to find dependency for curve block").clone();
        let interval_data: &IntervalData;
        if let ComputeBlock::Interval(data) = interval_block {
            interval_data = data;
        } else {
            panic!("internal error");
        }
        // We are creating a curve from an interval, output vertex count is the same as the input
        // one. Buffer size is 4 times as much, because we are storing a Vec4 instead of a f32
        let out_sizes = interval_data.out_sizes;
        let input_buffer_size = interval_data.buffer_size;
        let output_buffer_size = input_buffer_size * 4;
        let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            mapped_at_creation: true,
            size: output_buffer_size,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE,
        });

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

layout(set = 0, binding = 0) buffer InputBuffer {{
    float {par}_buff[];
}};

layout(set = 0, binding = 1) buffer OutputBuffer {{
    vec4 out_buff[];
}};

{header}

void main() {{
    uint index = gl_GlobalInvocationID.x;
    float {par} = {par}_buff[index];
    out_buff[index].x = {fx};
    out_buff[index].y = {fy};
    out_buff[index].z = {fz};
    out_buff[index].w = 1;
}}
"##, header=&compute_chain.shader_header, par=&interval_data.name, dimx=LOCAL_SIZE_X, dimy=1, fx=&descriptor.x_function, fy=&descriptor.y_function, fz=&descriptor.z_function);
        println!("debug info for curve shader: \n{}", shader_source);
        let mut shader_compiler = shaderc::Compiler::new().unwrap();
        let comp_spirv = shader_compiler.compile_into_spirv(&shader_source, shaderc::ShaderKind::Compute, "shader.comp", "main", None).unwrap();
        let comp_data = wgpu::util::make_spirv(comp_spirv.as_binary_u8());
        let shader_module = device.create_shader_module(comp_data);
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        count: None,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            min_binding_size: None,
                            dynamic: false,
                            readonly: false,
                        }
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        count: None,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            min_binding_size: None,
                            dynamic: false,
                            readonly: false,
                        }
                    }
                ],
                label: Some("ComputeShaderLayout")
            });
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer (
                        interval_data.out_buffer.slice(..),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer (
                        out_buffer.slice(..),
                    ),
                },
            ],
            label: Some("compute bind group"),
        });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[&compute_bind_group_layout, &compute_chain.globals_bind_layout],
                label: None,
                push_constant_ranges: &[],
            });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            layout: Some(&compute_pipeline_layout),
            label: None,
            compute_stage: wgpu::ProgrammableStageDescriptor {
                entry_point: "main",
                module: &shader_module,
            }
        });

        Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_sizes,
            buffer_size: output_buffer_size,
            shader_module,
        }
    }
}

impl CurveData {
    fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            compute_pass.dispatch((self.out_sizes.x/LOCAL_SIZE_X) as u32, 1, 1);
    }
}
