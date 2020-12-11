use crate::computable_scene::globals::Globals;
use crate::shader_processing::*;
use super::{ ComputeBlock, BlockCreationError, Dimensions, Parameter };
use super::ProcessingResult;

#[derive(Debug)]
pub struct IntervalBlockDescriptor {
    pub begin: String,
    pub end: String,
    pub quality: usize,
    pub name: String,
}
impl IntervalBlockDescriptor {
    pub fn to_block(self, device: &wgpu::Device, globals: &Globals) -> ProcessingResult {
        Ok(ComputeBlock::Interval(IntervalData::new(device, globals, self)?))
    }
}

pub struct IntervalData {
    pub out_buffer: wgpu::Buffer,
    pub buffer_size: wgpu::BufferAddress,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
    pub name: String,
}

impl IntervalData {
    pub fn new(device: &wgpu::Device, globals: &Globals, mut descriptor: IntervalBlockDescriptor) -> Result<Self, BlockCreationError> {
        if descriptor.quality < 1 || descriptor.quality > 16 {
            return Err(BlockCreationError::IncorrectAttributes("Interval quality attribute must be an integer in the [1, 16] range"))
        }
        if descriptor.name.is_empty() {
            return Err(BlockCreationError::IncorrectAttributes(" please provide a name \n for the interval's variable "));
        }
        if descriptor.begin.is_empty() {
            return Err(BlockCreationError::IncorrectAttributes(" please provide an expression \n for the interval's begin "));
        }
        if descriptor.end.is_empty() {
            return Err(BlockCreationError::IncorrectAttributes(" please provide an expression \n for the interval's end "));
        }
        let n_evals = 16 * descriptor.quality;
        // Make sure that the name does not contain any internal whitespace
        if descriptor.name.split_whitespace().count() != 1 {
            return Err(BlockCreationError::IncorrectAttributes(" an interval's name cannot \n contain spaces "))
        }
        // and then strip the leading and trailing ones
        descriptor.name.retain(|c| !c.is_whitespace());
        // Remove any leading and trailing whitespaces from the begin and end attributes.
        // This is done here because Parameters can be compared, and if we strip all
        // whitespaces here we are sure that the comparison will be succesful if the user
        // inputs the same thing in two different nodes but adds an extra whitespace.
        descriptor.begin.retain(|c| !c.is_whitespace());
        descriptor.end.retain(|c| !c.is_whitespace());
        let param = Parameter {
            name: descriptor.name.clone(),
            begin: descriptor.begin.clone(),
            end: descriptor.end.clone(),
            size: n_evals,
        };
        let out_dim = Dimensions::D1(param);
        let buffer_size = (n_evals * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            label: None,
            size: buffer_size,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::STORAGE,
        });

        // Optimization note: an interval, will always fit a single compute local group,
        // since the limit on the size of the work group (maxComputeWorkGroupInvocations)
        // is at least 256 on every device.
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer OutputBuffer {{
    float out_buff[];
}};

{globals_header}

void main() {{
    uint index = gl_GlobalInvocationID.x;
    float delta = ({interval_end} - {interval_begin}) / ({num_points} - 1.0);
    out_buff[index] = {interval_begin} + delta * index;
}}
"##, globals_header=&globals.shader_header, interval_begin=descriptor.begin, interval_end=descriptor.end, num_points=n_evals, dimx=n_evals
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
            buffer_size,
            name: descriptor.name,
        })
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            // BEWARE: as described before, we wrote the size of the buffer inside the local shader
            // dimensions, therefore the whole compute will always take just 1 dispatch
            compute_pass.dispatch(1, 1, 1);
    }
}

