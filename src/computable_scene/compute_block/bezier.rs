use crate::computable_scene::globals::Globals;
use crate::shader_processing::*;
use super::ComputeBlock;
use super::BlockId;
use super::Dimensions;
use super::BlockCreationError;
use super::{ProcessedMap, ProcessingResult};

#[derive(Debug)]
pub struct BezierBlockDescriptor {
    pub points: Vec<BlockId>,
    pub quality: usize,
}
impl BezierBlockDescriptor {
    pub fn make_block(self, device: &wgpu::Device, processed_blocks: &ProcessedMap) -> ProcessingResult {
        Ok(ComputeBlock::Bezier(BezierData::new(device, processed_blocks, self)?))
    }
}

pub struct BezierData {
    pub out_buffer: wgpu::Buffer,
    pub compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}

impl BezierData {
    pub fn new(device: &wgpu::Device, processed_blocks: &ProcessedMap, descriptor: BezierBlockDescriptor) -> Result<Self, BlockCreationError> {
        match descriptor.points.len() {
            0..=1 => Err(BlockCreationError::InputMissing(" A Bezier curve requires \n at least 2 points ")),
            2 => Self::new_degree_1(device, processed_blocks, descriptor),
            3 => Self::new_degree_2(device, processed_blocks, descriptor),
            4 => Self::new_degree_3(device, processed_blocks, descriptor),
            _ => Err(BlockCreationError::InternalError("Currently we only support Bézier curves up to degree 3")),
        }
    }

    fn get_point_buffer(processed_blocks: &ProcessedMap, id: BlockId) -> Result<&wgpu::Buffer, BlockCreationError> {
        let found_element = processed_blocks.get(&id).ok_or(BlockCreationError::InternalError("Point input does not exist in the block map"))?;
        let block = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;
        match block {
            ComputeBlock::Point(data) => Ok(&data.out_buffer),
            ComputeBlock::Transform(data) => {
                if let Dimensions::D0 = data.out_dim {
                    Ok(&data.out_buffer)
                } else {
                    Err(BlockCreationError::InputInvalid(" the input provided to Bezier \n is not a Point "))
                }
            },
            _ => Err(BlockCreationError::InputInvalid(" the input provided to Bezier \n is not a Point ")),
        }
    }

    fn create_parameter(quality: usize) -> super::Parameter {
        super::Parameter {
            name: None,
            begin: "0.0".into(),
            end: "1.0".into(),
            size: 16 * quality,
        }
    }
    pub fn new_degree_1(device: &wgpu::Device, processed_blocks: &ProcessedMap, descriptor: BezierBlockDescriptor) -> Result<Self, BlockCreationError> {
        let p0_buffer = Self::get_point_buffer(processed_blocks, descriptor.points[0])?;
        let p1_buffer = Self::get_point_buffer(processed_blocks, descriptor.points[1])?;

        let param = Self::create_parameter(descriptor.quality);

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {n_points}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputBuffer0 {{
    vec4 p0;
}};

layout(set = 0, binding = 1) buffer InputBuffer1 {{
    vec4 p1;
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

void main() {{
    uint index = gl_GlobalInvocationID.x;
    float t = index / ({n_points} - 1.0);
    vec4 f_t = (1-t) * p0 + t * p1;
    out_buff[index] = f_t;
}}
"##, n_points=param.size);

        let out_dim = Dimensions::D1(param);
        let out_buffer = out_dim.create_storage_buffer(4 * std::mem::size_of::<f32>(), device);

        let bindings = [
            // add descriptor for input buffers
            CustomBindDescriptor {
                position: 0,
                buffer: &p0_buffer
            },
            CustomBindDescriptor {
                position: 1,
                buffer: &p1_buffer
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 2,
                buffer: &out_buffer
            }
        ];

        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, None, Some("Bezier"))?;

        Ok(Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
        })
    }

    pub fn new_degree_2(device: &wgpu::Device, processed_blocks: &ProcessedMap, descriptor: BezierBlockDescriptor) -> Result<Self, BlockCreationError> {
        let p0_buffer = Self::get_point_buffer(processed_blocks, descriptor.points[0])?;
        let p1_buffer = Self::get_point_buffer(processed_blocks, descriptor.points[1])?;
        let p2_buffer = Self::get_point_buffer(processed_blocks, descriptor.points[2])?;

        let param = Self::create_parameter(descriptor.quality);

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {n_points}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputBuffer0 {{
    vec4 p0;
}};
layout(set = 0, binding = 1) buffer InputBuffer1 {{
    vec4 p1;
}};
layout(set = 0, binding = 2) buffer InputBuffer2 {{
    vec4 p2;
}};

layout(set = 0, binding = 3) buffer OutputBuffer {{
    vec4 out_buff[];
}};

void main() {{
    uint index = gl_GlobalInvocationID.x;
    float t = index / ({n_points} - 1.0);
    vec4 f_t = (1-t)*(1-t)*p0 + 2*(1-t)*t*p1 + t*t*p2;
    out_buff[index] = f_t;
}}
"##, n_points=param.size);

        let out_dim = Dimensions::D1(param);
        let out_buffer = out_dim.create_storage_buffer(4 * std::mem::size_of::<f32>(), device);

        let bindings = [
            // add descriptor for input buffers
            CustomBindDescriptor {
                position: 0,
                buffer: &p0_buffer
            },
            CustomBindDescriptor {
                position: 1,
                buffer: &p1_buffer
            },
            CustomBindDescriptor {
                position: 2,
                buffer: &p2_buffer
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 3,
                buffer: &out_buffer
            }
        ];

        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, None, Some("Bezier"))?;

        Ok(Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
        })
    }
    pub fn new_degree_3(device: &wgpu::Device, processed_blocks: &ProcessedMap, descriptor: BezierBlockDescriptor) -> Result<Self, BlockCreationError> {
        let p0_buffer = Self::get_point_buffer(processed_blocks, descriptor.points[0])?;
        let p1_buffer = Self::get_point_buffer(processed_blocks, descriptor.points[1])?;
        let p2_buffer = Self::get_point_buffer(processed_blocks, descriptor.points[2])?;
        let p3_buffer = Self::get_point_buffer(processed_blocks, descriptor.points[3])?;

        let param = Self::create_parameter(descriptor.quality);

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {n_points}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputBuffer0 {{
    vec4 p0;
}};
layout(set = 0, binding = 1) buffer InputBuffer1 {{
    vec4 p1;
}};
layout(set = 0, binding = 2) buffer InputBuffer2 {{
    vec4 p2;
}};
layout(set = 0, binding = 3) buffer InputBuffer3 {{
    vec4 p3;
}};

layout(set = 0, binding = 4) buffer OutputBuffer {{
    vec4 out_buff[];
}};

void main() {{
    uint index = gl_GlobalInvocationID.x;
    float t = index / ({n_points} - 1.0);
    vec4 f_t = (1-t)*(1-t)*(1-t)*p0 + 3*(1-t)*(1-t)*t*p1 + 3*(1-t)*t*t*p2 + t*t*t*p3;
    out_buff[index] = f_t;
}}
"##, n_points=param.size);

        let out_dim = Dimensions::D1(param);
        let out_buffer = out_dim.create_storage_buffer(4 * std::mem::size_of::<f32>(), device);

        let bindings = [
            // add descriptor for input buffers
            CustomBindDescriptor {
                position: 0,
                buffer: &p0_buffer
            },
            CustomBindDescriptor {
                position: 1,
                buffer: &p1_buffer
            },
            CustomBindDescriptor {
                position: 2,
                buffer: &p2_buffer
            },
            CustomBindDescriptor {
                position: 3,
                buffer: &p3_buffer
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 4,
                buffer: &out_buffer
            }
        ];

        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, None, Some("Bezier"))?;

        Ok(Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
        })
    }

    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("bezier compute pass"),
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            // BEWARE: just like we did for the curve, we wrote the size of the buffer inside the
            // local shader dimensions, therefore the whole compute will always take just 1 dispatch
            compute_pass.dispatch(1, 1, 1);
    }
}
