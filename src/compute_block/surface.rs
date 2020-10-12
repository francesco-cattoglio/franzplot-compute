use crate::compute_chain::ComputeChain;
use crate::shader_processing::*;
use super::ComputeBlock;
use super::Dimensions;
use super::IntervalData;
use serde::{Deserialize, Serialize};

const LOCAL_SIZE_X: usize = 16;
const LOCAL_SIZE_Y: usize = 16;

#[derive(Debug, Deserialize, Serialize)]
pub struct SurfaceBlockDescriptor {
    pub interval_first: String,
    pub interval_second: String,
    pub fx: String,
    pub fy: String,
    pub fz: String,
}
impl SurfaceBlockDescriptor {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> ComputeBlock {
        ComputeBlock::Surface(SurfaceData::new(chain, device, &self))
    }
}

pub struct SurfaceData {
    pub out_buffer: wgpu::Buffer,
    pub compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}

impl SurfaceData {
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &SurfaceBlockDescriptor) -> Self {
        let first_interval_block = compute_chain.get_block(&descriptor.interval_first).expect("unable to find first dependency for curve block");
        let first_interval_data: &IntervalData;
        if let ComputeBlock::Interval(data) = first_interval_block {
            first_interval_data = data;
        } else {
            panic!("internal error");
        }
        let second_interval_block = compute_chain.get_block(&descriptor.interval_second).expect("unable to find second dependency for curve block");
        let second_interval_data: &IntervalData;
        if let ComputeBlock::Interval(data) = second_interval_block {
            second_interval_data = data;
        } else {
            panic!("internal error");
        }
        // We are creating a surface from 2 intervals, output vertex count is the product of the
        // two interval sizes. Buffer size is 4 times as much, because we are storing a Vec4
        let dim_1 = first_interval_data.out_dim.as_1d().unwrap();
        let dim_2 = second_interval_data.out_dim.as_1d().unwrap();
        let out_dim = Dimensions::D2(dim_1, dim_2);
        let out_buffer = out_dim.create_storage_buffer(4 * std::mem::size_of::<f32>(), device);
            //let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        //    label: None,
        //    mapped_at_creation: false,
        //    size: output_buffer_size as wgpu::BufferAddress,
        //    usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::MAP_READ,
        //});

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

layout(set = 0, binding = 0) buffer InputBuffer1 {{
    float {par1}_buff[];
}};

layout(set = 0, binding = 1) buffer InputBuffer2 {{
    float {par2}_buff[];
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

{header}

void main() {{
    uint par1_idx = gl_GlobalInvocationID.x;
    uint par2_idx = gl_GlobalInvocationID.y;
    uint index = gl_GlobalInvocationID.x + gl_NumWorkGroups.x * gl_WorkGroupSize.x * gl_GlobalInvocationID.y;
    float {par1} = {par1}_buff[par1_idx];
    float {par2} = {par2}_buff[par2_idx];
    out_buff[index].x = {fx};
    out_buff[index].y = {fy};
    out_buff[index].z = {fz};
    out_buff[index].w = 1;
}}
"##, header=&compute_chain.shader_header, par1=&first_interval_data.name, par2=&second_interval_data.name, dimx=LOCAL_SIZE_X, dimy=LOCAL_SIZE_Y, fx=&descriptor.fx, fy=&descriptor.fy, fz=&descriptor.fz);
        println!("debug info for curve shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffers
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: first_interval_data.out_buffer.slice(..)
        });
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer_slice: second_interval_data.out_buffer.slice(..)
        });
        // add descriptor for output buffer
        bindings.push(CustomBindDescriptor {
            position: 2,
            buffer_slice: out_buffer.slice(..)
        });
        let (compute_pipeline, compute_bind_group) = compute_shader_from_glsl(shader_source.as_str(), &bindings, &compute_chain.globals_bind_layout, device, Some("Interval"));

        Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
        }
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            let (par_1, par_2) = self.out_dim.as_2d().unwrap();
            compute_pass.dispatch((par_1.size/LOCAL_SIZE_X) as u32, (par_2.size/LOCAL_SIZE_Y) as u32, 1);
    }
}
