use crate::compute_chain::ComputeChain;
use crate::shader_processing::*;
use super::ComputeBlock;
use super::Dimensions;

#[derive(Debug)]
pub struct MatrixBlockDescriptor {
    pub interval_id: Option<String>,
}

impl MatrixBlockDescriptor {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> ComputeBlock {
        println!("executing matrix to_block");
        ComputeBlock::Matrix(MatrixData::new(chain, device, &self))
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
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &MatrixBlockDescriptor) -> Self {
        if let Some(_) = &descriptor.interval_id {
            Self::new_with_interval(compute_chain, device, descriptor)
        } else {
            Self::new_without_interval(compute_chain, device, descriptor)
        }
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            // regardless of this matrix being a simple or an interval one, this will be at most
            // 256 elements, and this means that we can fit them into a single local group,
            // an we will only need 1 dispatch operation
            compute_pass.dispatch(1, 1, 1);
    }

    // TODO: maybe do not use the whole descriptor, but pass in the interval and the matrix strings
    fn new_with_interval(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &MatrixBlockDescriptor) -> Self {
        let interval_id = descriptor.interval_id.as_ref().unwrap();
        let interval_block = compute_chain.chain.get(interval_id).expect("could not find the interval");
        let interval_data;
        if let ComputeBlock::Interval(data) = interval_block {
            interval_data = data;
        } else {
            panic!("");
        }
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
    vec4 col_0 = vec4(1.0, 0.0, 0.0, 0.0);
    vec4 col_1 = vec4(0.0, 1.0, 0.0, 0.0);
    vec4 col_2 = vec4(0.0, 0.0, 1.0, 0.0);
    vec4 col_3 = vec4(0.0, 0.0, {par}, 1.0);

    out_buff[index][0] = col_0;
    out_buff[index][1] = col_1;
    out_buff[index][2] = col_2;
    out_buff[index][3] = col_3;
}}
"##, header=&compute_chain.shader_header, par=&interval_data.name, dimx=n_evals);
        println!("debug info for matrix shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: interval_data.out_buffer.slice(..)
        });
        // add descriptor for output buffer
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer_slice: out_buffer.slice(..)
        });
        let (compute_pipeline, compute_bind_group) = compute_shader_from_glsl(shader_source.as_str(), &bindings, &compute_chain.globals_bind_layout, device, Some("Interval"));

        Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
            buffer_size: output_buffer_size,
        }
    }

    fn new_without_interval(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &MatrixBlockDescriptor) -> Self {
        let out_dim = Dimensions::D0;
        // in order to keep the memory bandwidth as small as possible, we only pass in the first
        // three rows of the transform matrix
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
    vec4 col_0 = vec4(0.0, 1.0, 0.0, 0.0);
    vec4 col_1 = vec4(1.0, 0.0, 0.0, 0.0);
    vec4 col_2 = vec4(0.0, 0.0, 1.0, 0.0);
    vec4 col_3 = vec4(0.0, 0.0, 0.0, 1.0);
    out_buff[index][0] = col_0;
    out_buff[index][1] = col_1;
    out_buff[index][2] = col_2;
    out_buff[index][3] = col_3;
}}
"##, header=&compute_chain.shader_header);
        println!("debug info for matrix shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for output buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: out_buffer.slice(..)
        });
        let (compute_pipeline, compute_bind_group) = compute_shader_from_glsl(shader_source.as_str(), &bindings, &compute_chain.globals_bind_layout, device, Some("Interval"));

        Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
            buffer_size: output_buffer_size,
        }
    }

}
