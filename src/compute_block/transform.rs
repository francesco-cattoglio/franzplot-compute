use crate::compute_chain::ComputeChain;
use crate::shader_processing::*;
use super::ComputeBlock;
use super::Dimensions;

const LOCAL_SIZE_X: u32 = 16;
const LOCAL_SIZE_Y: u32 = 16;

#[derive(Debug)]
pub struct TransformBlockDescriptor {
    pub geometry_id: String,
    pub matrix_id: String,
}

impl TransformBlockDescriptor {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> ComputeBlock {
        ComputeBlock::Transform(TransformData::new(chain, device, &self))
    }
}

pub struct TransformData {
    pub out_buffer: wgpu::Buffer,
    pub buffer_size: wgpu::BufferAddress,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}

impl TransformData {
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &TransformBlockDescriptor) -> Self {
        let geometry_block = compute_chain.chain.get(&descriptor.geometry_id).expect("could not find input geometry");
        let matrix_block = compute_chain.chain.get(&descriptor.matrix_id).expect("could not find input matrix");
        let geometry_dim = match geometry_block {
            ComputeBlock::Curve(data) => data.out_dim.clone(),
            ComputeBlock::Surface(data) => data.out_dim.clone(),
            _ => panic!("Internal error"),
        };
        let matrix_sizes = match matrix_block {
            ComputeBlock::Matrix(data) => data.out_dim.clone(),
            _ => panic!("internal error"),
        };
        // now we need to do something different depending on the size of the matrices: if the
        // matrix is a simple one, then we just need a shader that reads every vector in the
        // input buffer and multiplies it by the matrix before outputting something.
        // Otherwise, we will need to compute new sizes depending on the initial geometry size!
        let mut out_sizes = geometry_dim;
        // TODO: make this into a match statement
        // TODO: we need to handle the situation in which I am applying a transform in the same
        // parameter name as the one being used in the curve or surface!
        if let Dimensions::D0 = matrix_sizes {
        } else {
        //    if geometry_sizes.x == 1 {
        //        // we had a point, our output will be a curve
        //        out_sizes.x = matrix_sizes.x;
        //        out_sizes.y = 1;
        //        out_sizes.z = 1;
        //    } else if geometry_sizes.x != 1 && geometry_sizes.y == 1 {
        //        // if we had a curve, the output will be a surface
        //        out_sizes.x = geometry_sizes.x;
        //        out_sizes.y = matrix_sizes.x;
        //        out_sizes.z = 1;
        //    } else {
        //        panic!("trying to apply a parametric transform to geometry which is already 2D")
        //    }
        //}
        //let output_buffer_size = 4 * std::mem::size_of::<f32>() as u64 * out_sizes.x as u64 * out_sizes.y as u64;
        //let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        //    label: None,
        //    mapped_at_creation: false,
        //    size: output_buffer_size as wgpu::BufferAddress,
        //    usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::MAP_READ,
        //});
        }
        unimplemented!();
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
        unimplemented!();
    }

    fn transform_1d_1d(compute_chain: &ComputeChain, device: &wgpu::Device, in_buff: wgpu::BufferSlice, in_matrix: wgpu::BufferSlice, out_buff: wgpu::BufferSlice) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputBuffer {{
    float in_buff[];
}};

layout(set = 0, binding = 1) buffer InputBuffer {{
    mat4 in_matrix;
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

{header}

void main() {{
    uint index = gl_GlobalInvocationID.x;
    out_buff[index] = in_matrix * in_buff[index];
}}
"##, header=&compute_chain.shader_header, dimx=LOCAL_SIZE_X);
        println!("debug info for 1d->1d transform shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: in_buff,
        });
        // add descriptor for matrix
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer_slice: in_matrix,
        });
        bindings.push(CustomBindDescriptor {
            position: 2,
            buffer_slice: out_buff,
        });
        compute_shader_from_glsl(shader_source.as_str(), &bindings, &compute_chain.globals_bind_layout, device, Some("Interval"))
    }

    fn transform_2d_2d(compute_chain: &ComputeChain, device: &wgpu::Device, in_buff: wgpu::BufferSlice, in_matrix: wgpu::BufferSlice, out_buff: wgpu::BufferSlice) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

layout(set = 0, binding = 0) buffer InputBuffer {{
    float in_buff[];
}};

layout(set = 0, binding = 1) buffer InputBuffer {{
    mat4 in_matrix;
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

{header}

void main() {{
    // the only difference between the 1d->1d and the 2d->2d shader is the local_sizes and the indexing
    uint index = gl_GlobalInvocationID.x + gl_WorkGroupSize.x * gl_GlobalInvocationID.y;
    out_buff[index] = in_matrix * in_buff[index];
}}
"##, header=&compute_chain.shader_header, dimx=LOCAL_SIZE_X, dimy=LOCAL_SIZE_Y);
        println!("debug info for 1d->1d transform shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffer
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: in_buff,
        });
        // add descriptor for matrix
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer_slice: in_matrix,
        });
        bindings.push(CustomBindDescriptor {
            position: 2,
            buffer_slice: out_buff,
        });
        compute_shader_from_glsl(shader_source.as_str(), &bindings, &compute_chain.globals_bind_layout, device, Some("Interval"))
    }
}
