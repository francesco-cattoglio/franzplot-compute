use crate::compute_chain::ComputeChain;
use crate::shader_processing::*;
use super::ComputeBlock;
use ultraviolet::Vec3u;
use super::IntervalData;

const LOCAL_SIZE_X: u32 = 16;
const LOCAL_SIZE_Y: u32 = 16;

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
    pub out_buffer: wgpu::Buffer,
    pub compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_sizes: Vec3u,
    #[allow(unused)]
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
            mapped_at_creation: false,
            size: output_buffer_size,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::STORAGE,
        });
        dbg!(&input_buffer_size);
        dbg!(&output_buffer_size);

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
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: interval_data.out_buffer.slice(..)
        });
        // add descriptor for output buffer
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer_slice: out_buffer.slice(..)
        });
        let (compute_pipeline, compute_bind_group) = compute_shader_from_glsl(shader_source.as_str(), &bindings, &compute_chain.globals_bind_layout, device, Some("Interval"));

        Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_sizes,
            buffer_size: output_buffer_size,
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

