use crate::computable_scene::globals::Globals;
use crate::shader_processing::*;
use super::{ComputeBlock, BlockCreationError, Dimensions};
use super::{ProcessingResult};

#[derive(Debug)]
pub struct VectorBlockDescriptor {
    pub vx: String,
    pub vy: String,
    pub vz: String,
}
impl VectorBlockDescriptor {
    pub fn make_block(self, device: &wgpu::Device, globals: &Globals) -> ProcessingResult {
        Ok(ComputeBlock::Vector(VectorData::new(device, globals, self)?))
    }
}

pub struct VectorData {
    pub out_buffer: wgpu::Buffer,
    pub compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}
impl VectorData {
    pub fn new(device: &wgpu::Device, globals: &Globals, descriptor: VectorBlockDescriptor) -> Result<Self, BlockCreationError> {
        let out_dim = Dimensions::D0;
        let out_buffer = out_dim.create_storage_buffer(4 * std::mem::size_of::<f32>(), device);

        // Sanitize all input expressions
        let maybe_vx = Globals::sanitize_expression(&descriptor.vx);
        let sanitized_vx = maybe_vx.ok_or(BlockCreationError::IncorrectAttributes(" the x field \n contains invalid symbols "))?;
        let maybe_vy = Globals::sanitize_expression(&descriptor.vy);
        let sanitized_vy = maybe_vy.ok_or(BlockCreationError::IncorrectAttributes(" the y field \n contains invalid symbols "))?;
        let maybe_vz = Globals::sanitize_expression(&descriptor.vz);
        let sanitized_vz = maybe_vz.ok_or(BlockCreationError::IncorrectAttributes(" the z field \n contains invalid symbols "))?;

        let shader_source = format!(r##"
#version 450
layout(local_size_x = 1, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer OutputBuffer {{
    vec4 out_buff;
}};

{header}

void main() {{
    out_buff.x = {vx};
    out_buff.y = {vy};
    out_buff.z = {vz};
    out_buff.w = 0.0;
}}
"##, header=&globals.shader_header, vx=sanitized_vx, vy=sanitized_vy, vz=sanitized_vz);

        let bindings = [
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 0,
                buffer: &out_buffer,
            }
        ];

        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, Some(&globals.bind_layout), Some("Vector"))?;
        Ok(Self {
            compute_bind_group,
            compute_pipeline,
            out_buffer,
            out_dim,
        })
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("vector compute pass"),
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            compute_pass.dispatch(1, 1, 1);
    }
}
