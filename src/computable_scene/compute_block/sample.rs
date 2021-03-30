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
        let found_element = processed_blocks.get(&geometry_id).ok_or(BlockCreationError::InternalError("Transform Geometry input does not exist in the block map".into()))?;
        let geometry_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;

        // Make sure that inputs are ok
        let maybe_name = Globals::sanitize_variable_name(&descriptor.parameter);
        let sanitized_name = maybe_name.ok_or(BlockCreationError::IncorrectAttributes(" the parameter's name \n is not valid "))?;
        let sanitized_value = globals.sanitize_expression(&descriptor.value)?;

        let (geometry_dim, geometry_buffer) = match geometry_block {
            ComputeBlock::Point(data) => (data.out_dim.clone(), &data.out_buffer),
            ComputeBlock::Curve(data) => (data.out_dim.clone(), &data.out_buffer),
            ComputeBlock::Surface(data) => (data.out_dim.clone(), &data.out_buffer),
            ComputeBlock::Transform(data) => (data.out_dim.clone(), &data.out_buffer),
            ComputeBlock::Prefab(data) => (data.out_dim.clone(), &data.out_buffer),
            ComputeBlock::Sample(data) => (data.out_dim.clone(), &data.out_buffer),
            _ => return Err(BlockCreationError::InputInvalid("the first input provided to the Sample is not a Geometry"))
        };

        match geometry_dim {
            Dimensions::D0 => Err(BlockCreationError::InputInvalid(" Cannot sample a parameter \n from a point ")),
            Dimensions::D1(param) => {
                Self::sample_1d_0d(device, globals, geometry_buffer, param, sanitized_name, &sanitized_value)
            },
            Dimensions::D2(param_1, param_2) => {
                Self::sample_2d_1d(device, globals, geometry_buffer, param_1, param_2, sanitized_name, &sanitized_value)
            },
            Dimensions::D3(_, _) => Err(BlockCreationError::InputInvalid(" Cannot sample a parameter \n from a prefab mesh ")),
        }
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("sampling compute pass"),
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
    // clamp index acces, to make sure nothing bad happens, no matter what value was given
    inf_idx = clamp(inf_idx, 0, {array_size} - 1);
    sup_idx = clamp(sup_idx, 0, {array_size} - 1);
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
        // TODO: maybe a re-write could be useful. The logic might be very hard to follow here.
        let used_param = match (geo_param_1.name.as_ref(), geo_param_2.name.as_ref()) {
            (Some(name_1), Some(name_2)) => {
                if name_1 == sampled_name {
                    1
                } else if name_2 == sampled_name {
                    2
                } else {
                    return Err(BlockCreationError::IncorrectAttributes(" the parameter used \n is not known "));
                }
            },
            (None, Some(name_2)) => {
                if name_2 == sampled_name {
                    2
                } else {
                    return Err(BlockCreationError::IncorrectAttributes(" the parameter used \n is not known "));
                }
            },
            (Some(name_1), None) => {
                if name_1 == sampled_name {
                    1
                } else {
                    return Err(BlockCreationError::IncorrectAttributes(" the parameter used \n is not known "));
                }
            },
            (None, None) => {
                return Err(BlockCreationError::IncorrectAttributes(" the parameter used \n is not known "));
            }
        };

        // we now know the parameter name, we know that we need to allocate space for one point.
        let out_param = if used_param == 1 {
            geo_param_2.clone()
        } else {
            geo_param_1.clone()
        };
        let out_dim = Dimensions::D1(out_param.clone());
        let out_buffer = out_dim.create_storage_buffer(4 * std::mem::size_of::<f32>(), &device);

        // this is the shader that samples the FIRST parameter, which means that the second
        // parameter is the one that survives
        let shader_source_1 = format!(r##"
#version 450
layout(local_size_x = {size_y}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputPoint {{
    vec4 in_buff[];
}};

layout(set = 0, binding = 1) buffer OutputBuffer {{
    vec4 out_buff[];
}};

{header}

void main() {{
    // parameter space is linear, so we can figure out which index we should access
    float size = {size_x};
    float interval_begin = {begin};
    float interval_end = {end};
    float value = {value};
    // transform the value so that the interval extends from 0 to size-1
    value = (value - interval_begin) * (size - 1) / (interval_end - interval_begin);
    int inf_idx = int(floor(value));
    int sup_idx = int(ceil(value));
    float alpha = fract(value);
    // clamp index acces, to make sure nothing bad happens, no matter what value was given
    inf_idx = clamp(inf_idx, 0, {size_x} - 1);
    sup_idx = clamp(sup_idx, 0, {size_x} - 1);
    uint inf_index = inf_idx + {size_x} * gl_GlobalInvocationID.x;
    uint sup_index = sup_idx + {size_x} * gl_GlobalInvocationID.x;
    out_buff[gl_GlobalInvocationID.x] = (1 - alpha) * in_buff[inf_index] + alpha * in_buff[sup_index];
    out_buff[gl_GlobalInvocationID.x].w = 1.0;
}}
"##, header=&globals.shader_header, size_x=geo_param_1.size, size_y=geo_param_2.size,
begin=&geo_param_1.begin, end=&geo_param_1.end, value=sampled_value);

        // this is the shader that samples the SECOND parameter, which means that the first
        // parameter is the one that survives
        let shader_source_2 = format!(r##"
#version 450
layout(local_size_x = {size_x}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputPoint {{
    vec4 in_buff[];
}};

layout(set = 0, binding = 1) buffer OutputBuffer {{
    vec4 out_buff[];
}};

{header}

void main() {{
    // parameter space is linear, so we can figure out which index we should access
    float size = {size_y};
    float interval_begin = {begin};
    float interval_end = {end};
    float value = {value};
    // transform the value so that the interval extends from 0 to size-1
    value = (value - interval_begin) * (size - 1) / (interval_end - interval_begin);
    int inf_idx = int(floor(value));
    int sup_idx = int(ceil(value));
    float alpha = fract(value);
    // clamp index acces, to make sure nothing bad happens, no matter what value was given
    inf_idx = clamp(inf_idx, 0, {size_y} - 1);
    sup_idx = clamp(sup_idx, 0, {size_y} - 1);
    uint inf_index = gl_GlobalInvocationID.x + {size_x} * inf_idx;
    uint sup_index = gl_GlobalInvocationID.x + {size_x} * sup_idx;
    out_buff[gl_GlobalInvocationID.x] = (1 - alpha) * in_buff[inf_index] + alpha * in_buff[sup_index];
    out_buff[gl_GlobalInvocationID.x].w = 1.0;
}}
"##, header=&globals.shader_header, size_x=geo_param_1.size, size_y=geo_param_2.size,
begin=&geo_param_2.begin, end=&geo_param_2.end, value=sampled_value);

        let shader_source = if used_param == 1 {
            shader_source_1
        } else {
            shader_source_2
        };

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
}

