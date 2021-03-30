use crate::computable_scene::globals::Globals;
use crate::shader_processing::*;
use super::{ComputeBlock, BlockCreationError, Dimensions};
use super::{ProcessingResult};

#[derive(Debug)]
pub struct PointBlockDescriptor {
    pub fx: String,
    pub fy: String,
    pub fz: String,
}
impl PointBlockDescriptor {
    pub fn make_block(self, device: &wgpu::Device, globals: &Globals) -> ProcessingResult {
        Ok(ComputeBlock::Point(PointData::new(device, globals, self)?))
    }
}

pub struct PointData {
    pub out_buffer: wgpu::Buffer,
    pub compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}
impl PointData {
    pub fn new(device: &wgpu::Device, globals: &Globals, descriptor: PointBlockDescriptor) -> Result<Self, BlockCreationError> {
        let out_dim = Dimensions::D0;
        let out_buffer = out_dim.create_storage_buffer(4 * std::mem::size_of::<f32>(), device);

        // Sanitize all input expressions
        let sanitized_fx = globals.sanitize_expression(&descriptor.fx)?;
        let sanitized_fy = globals.sanitize_expression(&descriptor.fy)?;
        let sanitized_fz = globals.sanitize_expression(&descriptor.fz)?;

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
"##, header=&globals.shader_header, fx=sanitized_fx, fy=sanitized_fy, fz=sanitized_fz);

        let bindings = [
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 0,
                buffer: &out_buffer
            }
        ];

        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, Some(&globals.bind_layout), Some("Point"))?;
        Ok(Self {
            compute_bind_group,
            compute_pipeline,
            out_buffer,
            out_dim,
        })
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("point compute pass"),
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            compute_pass.dispatch(1, 1, 1);
    }
}
