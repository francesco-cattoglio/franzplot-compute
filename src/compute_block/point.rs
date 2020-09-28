use crate::compute_chain::ComputeChain;
use crate::shader_processing::*;
use super::ComputeBlock;
use super::Dimensions;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PointBlockDescriptor {
    pub fx: String,
    pub fy: String,
    pub fz: String,
}
impl PointBlockDescriptor {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> ComputeBlock {
        ComputeBlock::Point(PointData::new(chain, device, &self))
    }
}

pub struct PointData {
    pub out_buffer: wgpu::Buffer,
    pub compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}
impl PointData {
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &PointBlockDescriptor) -> Self {
        let out_dim = Dimensions::D0;
        let out_buffer = out_dim.create_storage_buffer(4 * std::mem::size_of::<f32>(), device);
        let shader_source = format!(r##"
#version 450
layout(local_size_x = 1, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer OutputBuffer {{
    vec4 out_buff;
}};

{header}

void main() {{
    out_buff.x = {fx};
    out_buff.y = {fy};
    out_buff.z = {fz};
    out_buff.w = 1;
}}
"##, header=&compute_chain.shader_header, fx=&descriptor.fx, fy=&descriptor.fy, fz=&descriptor.fz);
        //println!("debug info for curve shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for output buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: out_buffer.slice(..)
        });
        let (compute_pipeline, compute_bind_group) = compute_shader_from_glsl(shader_source.as_str(), &bindings, &compute_chain.globals_bind_layout, device, Some("Interval"));
        Self {
            compute_bind_group,
            compute_pipeline,
            out_buffer,
            out_dim,
        }
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            compute_pass.dispatch(1, 1, 1);
    }
}
