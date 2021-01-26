use crate::computable_scene::globals::Globals;
use crate::shader_processing::*;
use super::{ComputeBlock, BlockId};
use super::Parameter;
use super::SurfaceData;
use super::BlockCreationError;
use super::Dimensions;
use super::{ProcessedMap, ProcessingResult};

const LOCAL_SIZE_X: usize = 16;
const LOCAL_SIZE_Y: usize = 16;

#[derive(Debug)]
pub struct PlaneBlockDescriptor {
    pub center: Option<BlockId>,
    pub normal: Option<BlockId>,
}
impl PlaneBlockDescriptor {
    pub fn make_block(self, device: &wgpu::Device, processed_blocks: &ProcessedMap) -> ProcessingResult {
        Ok(ComputeBlock::Surface(PlaneData::new(device, processed_blocks, self)?))
    }
}

pub struct PlaneData {
    pub out_buffer: wgpu::Buffer,
    pub compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}

impl PlaneData {
    pub fn new(device: &wgpu::Device, processed_blocks: &ProcessedMap, descriptor: PlaneBlockDescriptor) -> Result<SurfaceData, BlockCreationError> {
        let center_id = descriptor.center.ok_or(BlockCreationError::InputMissing(" This Plane node \n is missing the point input "))?;
        let found_element = processed_blocks.get(&center_id).ok_or(BlockCreationError::InternalError("Plane point input does not exist in the block map"))?;
        let center_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;

        let center_data = match center_block {
            ComputeBlock::Point(data) => data,
            _ => return Err(BlockCreationError::InputInvalid(" Plane first input \n is not a point "))
        };


        let normal_id = descriptor.normal.ok_or(BlockCreationError::InputMissing(" This Plane node \n is missing the normal input "))?;
        let found_element = processed_blocks.get(&normal_id).ok_or(BlockCreationError::InternalError("Plane normal input does not exist in the block map"))?;
        let normal_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;

        let normal_data = match normal_block {
            ComputeBlock::Vector(data) => data,
            _ => return Err(BlockCreationError::InputInvalid(" Plane normal input \n is not a vector ")),
        };

        // change "begin" and "end" to increase the size of the plane,
        let param = Parameter {
            name: None,
            begin: "-1.0".into(),
            end: "1.0".into(),
            size: 16,
        };

        let shader_source = format!(r##"

#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

layout(set = 0, binding = 0) buffer InputBuffer1 {{
    vec4 center;
}};

layout(set = 0, binding = 1) buffer InputBuffer2 {{
    vec4 normal;
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

void main() {{
    uint par1_idx = gl_GlobalInvocationID.x;
    uint par2_idx = gl_GlobalInvocationID.y;
    uint index = gl_GlobalInvocationID.x + gl_NumWorkGroups.x * gl_WorkGroupSize.x * gl_GlobalInvocationID.y;
    float delta = ({interval_end} - {interval_begin}) / ({n_points} - 1.0);
    vec3 versor = normalize(normal.xyz);
    vec3 cross_me = (abs(versor.z) > 0.01) ? vec3(0.0, 1.0, 0.0) : vec3(0.0, 0.0, 1.0);
    vec3 v1 = normalize(cross(cross_me, versor));
    vec3 v2 = normalize(cross(versor, v1));
    float delta_x = {interval_begin} + delta * par1_idx;
    float delta_y = {interval_begin} + delta * par2_idx;
    out_buff[index] = center + delta_x * vec4(v1, 0.0) + delta_y * vec4(v2, 0.0);
}}
"##, dimx=LOCAL_SIZE_X, dimy=LOCAL_SIZE_Y, interval_begin=param.begin, interval_end=param.end, n_points=param.size);

        let out_dim = Dimensions::D2(param.clone(), param.clone());
        let out_buffer = out_dim.create_storage_buffer(4 * std::mem::size_of::<f32>(), device);

        let bindings = [
            // add descriptor for input buffers
            CustomBindDescriptor {
                position: 0,
                buffer_slice: center_data.out_buffer.slice(..)
            },
            CustomBindDescriptor {
                position: 1,
                buffer_slice: normal_data.out_buffer.slice(..)
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 2,
                buffer_slice: out_buffer.slice(..)
            }
        ];

        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, None, Some("Plane"))?;

        Ok(SurfaceData{
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
        })
    }
}