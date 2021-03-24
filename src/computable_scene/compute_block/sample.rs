use crate::computable_scene::globals::Globals;
use crate::shader_processing::*;
use super::{ComputeBlock, BlockCreationError, Dimensions, BlockId};
use super::{ProcessedMap, ProcessingResult, Parameter};

#[derive(Debug)]
pub struct SampleBlockDescriptor {
    pub geometry: Option<BlockId>,
    pub parameter: String,
    pub value: String,
}
impl SampleBlockDescriptor {
    pub fn make_block(self, device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap) -> ProcessingResult {
        Ok(ComputeBlock::Sample(SampleData::new(device, globals, processed_blocks, self)?))
    }
}

pub struct SampleData {
    pub out_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}

impl SampleData {
    pub fn new(device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap, descriptor: SampleBlockDescriptor) -> Result<Self, BlockCreationError> {
        let geometry_id = descriptor.geometry.ok_or(BlockCreationError::InputMissing(" This Transform node \n is missing the Geometry input "))?;
        let found_element = processed_blocks.get(&geometry_id).ok_or(BlockCreationError::InternalError("Transform Geometry input does not exist in the block map"))?;
        let geometry_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;

        // Make sure that inputs are ok
        let maybe_name = Globals::sanitize_variable_name(&descriptor.parameter);
        let sanitized_name = maybe_name.ok_or(BlockCreationError::IncorrectAttributes(" the parameter's name \n is not valid "))?;
        let maybe_value = Globals::sanitize_expression(&descriptor.value);
        let sanitized_value = maybe_value.ok_or(BlockCreationError::IncorrectAttributes(" the value used \n is not valid "))?;

        let (geometry_dim, geometry_buffer) = match geometry_block {
            ComputeBlock::Point(data) => (data.out_dim.clone(), &data.out_buffer),
            ComputeBlock::Curve(data) => (data.out_dim.clone(), &data.out_buffer),
            ComputeBlock::Surface(data) => (data.out_dim.clone(), &data.out_buffer),
            ComputeBlock::Transform(data) => (data.out_dim.clone(), &data.out_buffer),
            ComputeBlock::Prefab(data) => (data.out_dim.clone(), &data.out_buffer),
            _ => return Err(BlockCreationError::InputInvalid("the first input provided to the Transform is not a Geometry"))
        };

        match geometry_dim {
            Dimensions::D0 => Err(BlockCreationError::InputInvalid(" Cannot sample a parameter \n from a point ")),
            Dimensions::D1(param) => {
                Self::sample_1d_0d(device, globals, geometry_buffer, param, sanitized_name, sanitized_value)
            },
            Dimensions::D2(param_1, param_2) => {
                Self::sample_2d_1d(device, globals, geometry_buffer, param_1, param_2, sanitized_name, sanitized_value)
            },
            Dimensions::D3(_, _) => Err(BlockCreationError::InputInvalid(" Cannot sample a parameter \n from a prefab mesh ")),
        }
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("interval compute pass"),
        });
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
        compute_pass.set_bind_group(1, variables_bind_group, &[]);
        // BEWARE: as described before, we wrote the size of the buffer inside the local shader
        // dimensions, therefore the whole compute will always take just 1 dispatch
        compute_pass.dispatch(1, 1, 1);
    }

    fn sample_1d_0d(device: &wgpu::Device, globals: &Globals, geo_buff: &wgpu::Buffer, geo_param: Parameter, sampled_name: &str, sampled_value: &str) -> Result<Self, BlockCreationError> {
        if let Some(name) = geo_param.name.as_ref() {
            // if the name does not match the one from the parameter, error out
            if name != sampled_name {
                return Err(BlockCreationError::IncorrectAttributes(" the parameter used \n is not known "));
            }
        } else {
            // if the geometry parameter does not exist, error our as well.
            // TODO: we might want to change this, so that one can sample a Bezier curve
            return Err(BlockCreationError::IncorrectAttributes(" the parameter used \n is not known "));
        }

        // we now know the parameter name, we know that we need to allocate space for one point.
        let out_dim = Dimensions::D0;
        let out_buffer = out_dim.create_storage_buffer(4 * std::mem::size_of::<f32>(), &device);

        let shader_source = format!(r##"
#version 450
layout(local_size_x = 1, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputPoint {{
    vec4 in_buff[];
}};

layout(set = 0, binding = 1) buffer OutputBuffer {{
    vec4 out_buff;
}};

{header}

void main() {{
    // parameter space is linear, so we can figure out which index we should access
    float size = {array_size};
    float interval_begin = {begin};
    float interval_end = {end};
    float value = {value};
    // transform the value so that the interval extends from 0 to size-1
    value = (value - interval_begin) * (size - 1) / (interval_end - interval_begin);
    int inf_idx = int(floor(value));
    int sup_idx = int(ceil(value));
    float alpha = fract(value);
    out_buff = (1 - alpha) * in_buff[inf_idx] + alpha * in_buff[sup_idx];
    out_buff.w = 1.0;
}}
"##, header=&globals.shader_header, array_size=geo_param.size, begin=&geo_param.begin, end=&geo_param.end, value=sampled_value);

        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer: geo_buff,
        });
        // add descriptor for matrix
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer: &out_buffer,
        });
        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, Some(&globals.bind_layout), Some("Sample 1D-0D"))?;
        Ok(Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
        })
    }

    fn sample_2d_1d(device: &wgpu::Device, globals: &Globals, geo_buff: &wgpu::Buffer, geo_param_1: Parameter, geo_param_2: Parameter, sampled_name: &str, sampled_value: &str) -> Result<Self, BlockCreationError> {
        todo!()
    }
}

