use crate::computable_scene::globals::Globals;
use crate::shader_processing::*;
use super::{ComputeBlock, BlockId};
use super::BlockCreationError;
use super::Dimensions;
use super::{ProcessedMap, ProcessingResult};

const LOCAL_SIZE_X: usize = 16;
const LOCAL_SIZE_Y: usize = 16;

#[derive(Debug)]
pub struct SurfaceBlockDescriptor {
    pub interval_first: Option<BlockId>,
    pub interval_second: Option<BlockId>,
    pub fx: String,
    pub fy: String,
    pub fz: String,
}
impl SurfaceBlockDescriptor {
    pub fn make_block(self, device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap) -> ProcessingResult {
        Ok(ComputeBlock::Surface(SurfaceData::new(device, globals, processed_blocks, self)?))
    }
}

pub struct SurfaceData {
    pub out_buffer: wgpu::Buffer,
    pub compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}

impl SurfaceData {
    pub fn new(device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap, descriptor: SurfaceBlockDescriptor) -> Result<Self, BlockCreationError> {
        let first_input_id = descriptor.interval_first.ok_or(BlockCreationError::InputMissing(" This Surface node \n is missing the first input "))?;
        let found_element = processed_blocks.get(&first_input_id).ok_or(BlockCreationError::InternalError("Surface first input does not exist in the block map"))?;
        let first_input_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;

        let first_interval_data = match first_input_block {
            ComputeBlock::Interval(data) => data,
            _ => return Err(BlockCreationError::InputInvalid("the first input provided to the Surface is not an Interval"))
        };

        let second_input_id = descriptor.interval_second.ok_or(BlockCreationError::InputMissing(" This surface node \n is missing the second input "))?;
        let found_element = processed_blocks.get(&second_input_id).ok_or(BlockCreationError::InternalError("Surface second input does not exist in the block map"))?;
        let second_input_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;

        let second_interval_data = match second_input_block {
            ComputeBlock::Interval(data) => data,
            _ => return Err(BlockCreationError::InputInvalid("the second input provided to the Surface is not an Interval"))
        };

        // We are creating a surface from 2 intervals, output vertex count is the product of the
        // two interval sizes. Buffer size is 4 times as much, because we are storing a Vec4
        let par_1 = first_interval_data.out_dim.as_1d()?;
        let par_2 = second_interval_data.out_dim.as_1d()?;
        let same_intervals = par_1.is_equal(&par_2)?;
        if same_intervals {
            return Err(BlockCreationError::IncorrectAttributes(" The two input intervals \n must have different names "));
        }

        let par_1_name = par_1.name.clone().unwrap();
        let par_2_name = par_2.name.clone().unwrap();

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

layout(set = 0, binding = 0) buffer InputBuffer1 {{
    float par1_buff[];
}};

layout(set = 0, binding = 1) buffer InputBuffer2 {{
    float par2_buff[];
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

{header}

void main() {{
    uint par1_idx = gl_GlobalInvocationID.x;
    uint par2_idx = gl_GlobalInvocationID.y;
    uint index = gl_GlobalInvocationID.x + gl_NumWorkGroups.x * gl_WorkGroupSize.x * gl_GlobalInvocationID.y;
    float {par1} = par1_buff[par1_idx];
    float {par2} = par2_buff[par2_idx];
    out_buff[index].x = {fx};
    out_buff[index].y = {fy};
    out_buff[index].z = {fz};
    out_buff[index].w = 1;
}}
"##, header=&globals.shader_header, par1=par_1_name, par2=par_2_name, dimx=LOCAL_SIZE_X, dimy=LOCAL_SIZE_Y, fx=&descriptor.fx, fy=&descriptor.fy, fz=&descriptor.fz);

        let out_dim = Dimensions::D2(par_1, par_2);
        let out_buffer = out_dim.create_storage_buffer(4 * std::mem::size_of::<f32>(), device);

        let bindings = [
            // add descriptor for input buffers
            CustomBindDescriptor {
                position: 0,
                buffer_slice: first_interval_data.out_buffer.slice(..)
            },
            CustomBindDescriptor {
                position: 1,
                buffer_slice: second_interval_data.out_buffer.slice(..)
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 2,
                buffer_slice: out_buffer.slice(..)
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
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            let (par_1, par_2) = self.out_dim.as_2d().unwrap();
            compute_pass.dispatch((par_1.size/LOCAL_SIZE_X) as u32, (par_2.size/LOCAL_SIZE_Y) as u32, 1);
    }
}
