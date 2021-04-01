use crate::computable_scene::globals::Globals;
use crate::shader_processing::*;
use super::ComputeBlock;
use super::BlockId;
use super::Dimensions;
use super::BlockCreationError;
use super::{ProcessedMap, ProcessingResult};

#[derive(Debug)]
pub struct CurveBlockDescriptor {
    pub interval: Option<BlockId>,
    pub fx: String,
    pub fy: String,
    pub fz: String,
}
impl CurveBlockDescriptor {
    pub fn make_block(self, device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap) -> ProcessingResult {
        Ok(ComputeBlock::Curve(CurveData::new(device, globals, processed_blocks, self)?))
    }
}

pub struct CurveData {
    pub out_buffer: wgpu::Buffer,
    pub compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}

impl CurveData {
    pub fn new(device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap, descriptor: CurveBlockDescriptor) -> Result<Self, BlockCreationError> {
        let input_id = descriptor.interval.ok_or(BlockCreationError::InputMissing(" This Curve node \n is missing its input "))?;
        let found_element = processed_blocks.get(&input_id).ok_or(BlockCreationError::InternalError("Curve input does not exist in the block map".into()))?;
        let input_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;

        let interval_data = match input_block {
            ComputeBlock::Interval(data) => data,
            _ => return Err(BlockCreationError::InputInvalid("the input provided to the Curve is not an Interval"))
        };

        // We are creating a curve from an interval, output vertex count is the same as the input
        // one. Buffer size is 4 times as much, because we are storing a Vec4 instead of a f32
        let param = interval_data.out_dim.as_1d()?;
        let param_name = param.name.clone().unwrap();

        // Sanitize all input expressions
        let local_params = vec![param_name.as_str()];
        let sanitized_fx = globals.sanitize_expression(&local_params, &descriptor.fx)?;
        let sanitized_fy = globals.sanitize_expression(&local_params, &descriptor.fy)?;
        let sanitized_fz = globals.sanitize_expression(&local_params, &descriptor.fz)?;

        // Optimization note: a curve, just line an interval, will always fit a single compute
        // invocation, since the limit on the size of the work group (maxComputeWorkGroupInvocations)
        // is at least 256 on every device.
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputBuffer {{
    float in_buff[];
}};

layout(set = 0, binding = 1) buffer OutputBuffer {{
    vec4 out_buff[];
}};

{header}

void main() {{
    uint index = gl_GlobalInvocationID.x;
    float {par} = in_buff[index];
    out_buff[index].x = {fx};
    out_buff[index].y = {fy};
    out_buff[index].z = {fz};
    out_buff[index].w = 1;
}}
"##, header=&globals.shader_header, par=param_name, dimx=param.size, fx=sanitized_fx, fy=sanitized_fy, fz=sanitized_fz);

        let out_dim = Dimensions::D1(param);
        let out_buffer = out_dim.create_storage_buffer(4 * std::mem::size_of::<f32>(), device);

        let bindings = [
            // add descriptor for input buffer
            CustomBindDescriptor {
                position: 0,
                buffer: &interval_data.out_buffer
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 1,
                buffer: &out_buffer
            }
        ];

        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, Some(&globals.bind_layout), Some("Interval"))?;

        Ok(Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
        })
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("curve compute pass"),
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            // BEWARE: as described before, we wrote the size of the buffer inside the local shader
            // dimensions, therefore the whole compute will always take just 1 dispatch
            compute_pass.dispatch(1, 1, 1);
    }
}

