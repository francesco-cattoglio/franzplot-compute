use crate::compute_chain::ComputeChain;
use crate::shader_processing::*;
use super::ComputeBlock;
use ultraviolet::Vec3u;

const LOCAL_SIZE_X: u32 = 16;
const LOCAL_SIZE_Y: u32 = 16;

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
    pub out_buffer: wgpu::Buffer,
    pub buffer_size: wgpu::BufferAddress,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_sizes: Vec3u,
    pub name: String,
}

impl IntervalData {
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &IntervalBlockDescriptor) -> Self {
        let out_sizes = Vec3u::new(16 * descriptor.quality, 1, 1);
        let buffer_size = (out_sizes.x * std::mem::size_of::<f32>() as u32) as wgpu::BufferAddress;
        let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            label: None,
            size: buffer_size,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::STORAGE,
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
        println!("debug info for interval shader: \n{}", &shader_source);

        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for output buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: out_buffer.slice(..)
        });
        let (compute_pipeline, compute_bind_group) = compute_shader_from_glsl(shader_source.as_str(), &bindings, &compute_chain.globals_bind_layout, device, Some("Interval"));
        Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_sizes,
            buffer_size,
            name: descriptor.name.clone(),
        }
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            compute_pass.dispatch((self.out_sizes.x/LOCAL_SIZE_X) as u32, 1, 1);
    }
}

