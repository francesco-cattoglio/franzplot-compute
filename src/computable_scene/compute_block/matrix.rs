use crate::computable_scene::globals::Globals;
use crate::shader_processing::*;
use smol_str::SmolStr;
use super::ComputeBlock;
use super::BlockId;
use super::BlockCreationError;
use super::{ProcessedMap, ProcessingResult};
use super::Dimensions;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MatrixBlockDescriptor {
    pub interval: Option<BlockId>,
    pub row_1: [SmolStr; 4], // matrix elements, row-major order
    pub row_2: [SmolStr; 4], // matrix elements, row-major order
    pub row_3: [SmolStr; 4], // matrix elements, row-major order
}

impl MatrixBlockDescriptor {
    pub fn to_block(&self, device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap) -> ProcessingResult {
        Ok(ComputeBlock::Matrix(MatrixData::new(device, globals, processed_blocks, &self)?))
    }

    pub fn get_input_ids(&self) -> Vec<BlockId> {
        match self.interval {
            Some(id) => vec![id],
            None => vec![]
        }
    }
}

impl Default for MatrixBlockDescriptor {
    fn default() -> Self {
        Self {
            interval: None,
            row_1: ["1.0".into(),"0.0".into(),"0.0".into(),"0.0".into()],
            row_2: ["0.0".into(),"1.0".into(),"0.0".into(),"0.0".into()],
            row_3: ["0.0".into(),"0.0".into(),"1.0".into(),"0.0".into()]
        }
    }
}

pub struct MatrixData {
    pub out_buffer: wgpu::Buffer,
    pub buffer_size: wgpu::BufferAddress,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}

impl MatrixData {
    pub fn new(device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap, descriptor: &MatrixBlockDescriptor) -> Result<Self, BlockCreationError> {
        if descriptor.interval.is_some() {
            Self::new_with_interval(device, globals, processed_blocks, descriptor)
        } else {
            Self::new_without_interval(device, globals, descriptor)
        }
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            compute_pass.dispatch(1, 1, 1);
    }

    fn new_with_interval(device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap, desc: &MatrixBlockDescriptor) -> Result<Self, BlockCreationError> {
        let input_id = desc.interval.ok_or(BlockCreationError::InternalError("Matrix new_with_interval() called with no-interval descriptor"))?;
        let found_element = processed_blocks.get(&input_id).ok_or(BlockCreationError::InternalError("Matrix interval input does not exist in the block map"))?;
        let input_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;

        let interval_data = match input_block {
            ComputeBlock::Interval(data) => data,
            _ => return Err(BlockCreationError::InputInvalid("the input provided to the Matrix is not an Interval"))
        };

        let out_dim = interval_data.out_dim.clone();
        let par = out_dim.as_1d().unwrap();
        let n_evals = par.size;
        let output_buffer_size = (16 * std::mem::size_of::<f32>() * n_evals) as wgpu::BufferAddress;
        let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            mapped_at_creation: false,
            size: output_buffer_size,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::STORAGE,
        });
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputBuffer {{
    float {par}_buff[];
}};

layout(set = 0, binding = 1) buffer OutputBuffer {{
    mat4 out_buff[];
}};

{header}

void main() {{
    uint index = gl_GlobalInvocationID.x;
    float {par} = {par}_buff[index];
    vec4 col_0 = vec4({_m00}, {_m10}, {_m20}, 0.0);
    vec4 col_1 = vec4({_m01}, {_m11}, {_m21}, 0.0);
    vec4 col_2 = vec4({_m02}, {_m12}, {_m22}, 0.0);
    vec4 col_3 = vec4({_m03}, {_m13}, {_m23}, 1.0);

    out_buff[index][0] = col_0;
    out_buff[index][1] = col_1;
    out_buff[index][2] = col_2;
    out_buff[index][3] = col_3;
}}
"##, header=&globals.shader_header, par=&interval_data.name, dimx=n_evals,
    _m00=desc.row_1[0], _m10=desc.row_2[0], _m20=desc.row_3[0],
    _m01=desc.row_1[1], _m11=desc.row_2[1], _m21=desc.row_3[1],
    _m02=desc.row_1[2], _m12=desc.row_2[2], _m22=desc.row_3[2],
    _m03=desc.row_1[3], _m13=desc.row_2[3], _m23=desc.row_3[3],
);

        let bindings = [
            // add descriptor for input buffer
            CustomBindDescriptor {
                position: 0,
                buffer_slice: interval_data.out_buffer.slice(..)
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 1,
                buffer_slice: out_buffer.slice(..)
            }
        ];

        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, Some(&globals.bind_layout), Some("Interval"))?;

        Ok(Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
            buffer_size: output_buffer_size,
        })
    }

    fn new_without_interval(device: &wgpu::Device, globals: &Globals, desc: &MatrixBlockDescriptor) -> Result<Self, BlockCreationError> {
        let out_dim = Dimensions::D0;
        let output_buffer_size = 16 * std::mem::size_of::<f32>() as wgpu::BufferAddress;
        let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            mapped_at_creation: false,
            size: output_buffer_size,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::STORAGE,
        });
        let shader_source = format!(r##"
#version 450
layout(local_size_x = 1, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer OutputBuffer {{
    mat4 out_buff[];
}};

{header}

void main() {{
    uint index = gl_GlobalInvocationID.x;
    vec4 col_0 = vec4({_m00}, {_m10}, {_m20}, 0.0);
    vec4 col_1 = vec4({_m01}, {_m11}, {_m21}, 0.0);
    vec4 col_2 = vec4({_m02}, {_m12}, {_m22}, 0.0);
    vec4 col_3 = vec4({_m03}, {_m13}, {_m23}, 1.0);

    out_buff[index][0] = col_0;
    out_buff[index][1] = col_1;
    out_buff[index][2] = col_2;
    out_buff[index][3] = col_3;
}}
"##, header=&globals.shader_header,
    _m00=desc.row_1[0], _m10=desc.row_2[0], _m20=desc.row_3[0],
    _m01=desc.row_1[1], _m11=desc.row_2[1], _m21=desc.row_3[1],
    _m02=desc.row_1[2], _m12=desc.row_2[2], _m22=desc.row_3[2],
    _m03=desc.row_1[3], _m13=desc.row_2[3], _m23=desc.row_3[3],
);

        let bindings = [
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 0,
                buffer_slice: out_buffer.slice(..)
            }
        ];

        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, Some(&globals.bind_layout), Some("Interval"))?;

        Ok(Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
            buffer_size: output_buffer_size,
        })
    }

}
