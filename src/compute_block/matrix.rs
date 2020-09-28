use crate::compute_chain::ComputeChain;
use crate::shader_processing::*;
use smol_str::SmolStr;
use super::ComputeBlock;
use super::Dimensions;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MatrixBlockDescriptor {
    pub interval_id: Option<String>,
    pub m: [[SmolStr; 4]; 3], // matrix elements, row-major order
}

impl MatrixBlockDescriptor {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> ComputeBlock {
        ComputeBlock::Matrix(MatrixData::new(chain, device, &self))
    }
}

impl Default for MatrixBlockDescriptor {
    fn default() -> Self {
        Self {
            interval_id: None,
            m: [
                ["1.0".into(),"0.0".into(),"0.0".into(),"0.0".into()],
                ["0.0".into(),"1.0".into(),"0.0".into(),"0.0".into()],
                ["0.0".into(),"0.0".into(),"1.0".into(),"0.0".into()]
            ]
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
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &MatrixBlockDescriptor) -> Self {
        if descriptor.interval_id.is_some() {
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
            compute_pass.dispatch(1, 1, 1);
    }

    fn new_with_interval(compute_chain: &ComputeChain, device: &wgpu::Device, desc: &MatrixBlockDescriptor) -> Self {
        let interval_id = desc.interval_id.as_ref().unwrap();
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
    vec4 col_0 = vec4({_m00}, {_m10}, {_m20}, 0.0);
    vec4 col_1 = vec4({_m01}, {_m11}, {_m21}, 0.0);
    vec4 col_2 = vec4({_m02}, {_m12}, {_m22}, 0.0);
    vec4 col_3 = vec4({_m03}, {_m13}, {_m23}, 1.0);

    out_buff[index][0] = col_0;
    out_buff[index][1] = col_1;
    out_buff[index][2] = col_2;
    out_buff[index][3] = col_3;
}}
"##, header=&compute_chain.shader_header, par=&interval_data.name, dimx=n_evals,
    _m00=desc.m[0][0], _m10=desc.m[1][0], _m20=desc.m[2][0],
    _m01=desc.m[0][1], _m11=desc.m[1][1], _m21=desc.m[2][1],
    _m02=desc.m[0][2], _m12=desc.m[1][2], _m22=desc.m[2][2],
    _m03=desc.m[0][3], _m13=desc.m[1][3], _m23=desc.m[2][3],
);
        //println!("debug info for matrix shader: \n{}", shader_source);
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

    fn new_without_interval(compute_chain: &ComputeChain, device: &wgpu::Device, desc: &MatrixBlockDescriptor) -> Self {
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
"##, header=&compute_chain.shader_header,
    _m00=desc.m[0][0], _m10=desc.m[1][0], _m20=desc.m[2][0],
    _m01=desc.m[0][1], _m11=desc.m[1][1], _m21=desc.m[2][1],
    _m02=desc.m[0][2], _m12=desc.m[1][2], _m22=desc.m[2][2],
    _m03=desc.m[0][3], _m13=desc.m[1][3], _m23=desc.m[2][3],
);
        //println!("debug info for matrix shader: \n{}", shader_source);
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
