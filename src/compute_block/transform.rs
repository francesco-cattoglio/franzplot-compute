use crate::compute_chain::ComputeChain;
use crate::shader_processing::*;
use super::ComputeBlock;
use super::Dimensions;

const LOCAL_SIZE_X: usize = 16;
const LOCAL_SIZE_Y: usize = 16;

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

enum TransformKind {
    SameDim,
    Dim01,
    Dim12,
    D1Multi,
    D2Multi1,
    D2Multi2,
}
pub struct TransformData {
    pub out_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
    dispatch_sizes: (usize, usize),
}

impl TransformData {
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &TransformBlockDescriptor) -> Self {
        let geometry_block = compute_chain.chain.get(&descriptor.geometry_id).expect("could not find input geometry");
        let matrix_block = compute_chain.chain.get(&descriptor.matrix_id).expect("could not find input matrix");
        let (geometry_dim, geometry_buffer_slice) = match geometry_block {
            ComputeBlock::Curve(data) => (data.out_dim.clone(), data.out_buffer.slice(..)),
            ComputeBlock::Surface(data) => (data.out_dim.clone(), data.out_buffer.slice(..)),
            _ => panic!("Internal error"),
        };
        let (matrix_dim, matrix_buffer_slice) = match matrix_block {
            ComputeBlock::Matrix(data) => (data.out_dim.clone(), data.out_buffer.slice(..)),
            _ => panic!("internal error"),
        };
        // now we need to do something different depending on the size of the matrices: if the
        // matrix is a simple one, then we just need a shader that reads every vector in the
        // input buffer and multiplies it by the matrix before outputting something.
        // Otherwise, we will need to compute new sizes depending on the initial geometry size!
        let out_dim: Dimensions;
        let out_buffer: wgpu::Buffer;
        let compute_pipeline: wgpu::ComputePipeline;
        let compute_bind_group: wgpu::BindGroup;
        let dispatch_sizes: (usize, usize);
        // TODO: make this into a match statement
        // TODO: we need to handle the situation in which I am applying a transform in the same
        // parameter name as the one being used in the curve or surface!
        let elem_size = 4 * std::mem::size_of::<f32>();
        match (geometry_dim, matrix_dim) {
            (Dimensions::D0, Dimensions::D0) => {
                out_dim = Dimensions::D0;
                unimplemented!();
            },
            (Dimensions::D0, Dimensions::D1(mat_param)) => {
                out_dim = Dimensions::D1(mat_param.clone());
                unimplemented!();
            },
            (Dimensions::D1(geo_param), Dimensions::D0) => {
                out_dim = Dimensions::D1(geo_param.clone());
                out_buffer = out_dim.create_storage_buffer(elem_size, &device);
                dispatch_sizes = (geo_param.size/LOCAL_SIZE_X, 1);
                let (pipeline, bind_group) = Self::transform_1d_1d(
                    &compute_chain,
                    &device,
                    geometry_buffer_slice,
                    matrix_buffer_slice,
                    out_buffer.slice(..),
                    );
                compute_pipeline = pipeline;
                compute_bind_group = bind_group;
            },
            (Dimensions::D1(geo_param), Dimensions::D1(mat_param)) if geo_param == mat_param => {
                out_dim = Dimensions::D1(geo_param.clone());
                out_buffer = out_dim.create_storage_buffer(elem_size, &device);
                dispatch_sizes = (geo_param.size/LOCAL_SIZE_X, 1);
                let (pipeline, bind_group) = Self::transform_1d_multi(
                    &compute_chain,
                    &device,
                    geometry_buffer_slice,
                    matrix_buffer_slice,
                    out_buffer.slice(..),
                    );
                compute_pipeline = pipeline;
                compute_bind_group = bind_group;
            },
            (Dimensions::D1(geo_param), Dimensions::D1(mat_param)) => {
                out_dim = Dimensions::D2(geo_param.clone(), mat_param.clone());
                out_buffer = out_dim.create_storage_buffer(elem_size, &device);
                dispatch_sizes = (geo_param.size/LOCAL_SIZE_X, mat_param.size/LOCAL_SIZE_Y);
                let (pipeline, bind_group) = Self::transform_1d_up2(
                    &compute_chain,
                    &device,
                    geometry_buffer_slice,
                    matrix_buffer_slice,
                    out_buffer.slice(..),
                    );
                compute_pipeline = pipeline;
                compute_bind_group = bind_group;
            },
            (Dimensions::D2(geo_p1, geo_p2), Dimensions::D0) => {
                out_dim = Dimensions::D2(geo_p1.clone(), geo_p2.clone());
                out_buffer = out_dim.create_storage_buffer(elem_size, &device);
                dispatch_sizes = (geo_p1.size/LOCAL_SIZE_X, geo_p2.size/LOCAL_SIZE_Y);
                let (pipeline, bind_group) = Self::transform_2d_2d(
                    &compute_chain,
                    &device,
                    geometry_buffer_slice,
                    matrix_buffer_slice,
                    out_buffer.slice(..),
                    );
                compute_pipeline = pipeline;
                compute_bind_group = bind_group;
            },
            (Dimensions::D2(geo_p1, geo_p2), Dimensions::D1(mat_param)) if geo_p1 == mat_param => {
                out_dim = Dimensions::D2(geo_p1.clone(), geo_p2.clone());
                unimplemented!();
            },
            (Dimensions::D2(geo_p1, geo_p2), Dimensions::D1(mat_param)) if geo_p2 == mat_param => {
                out_dim = Dimensions::D2(geo_p1.clone(), geo_p2.clone());
                unimplemented!();
            },
            (Dimensions::D2(geo_p1, geo_p2), Dimensions::D1(mat_param)) => {
                panic!("We are applying a parametric transform to a surface");
            },
            (_, Dimensions::D2(_, _)) => {
                panic!("Matrix has 2 parameters!");
            },
        }
        Self {
            compute_pipeline,
            compute_bind_group,
            dispatch_sizes,
            out_dim,
            out_buffer,
        }
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            // BEWARE: as described before, we wrote the size of the buffer inside the local shader
            // dimensions, therefore the whole compute will always take just 1 dispatch
            compute_pass.dispatch(self.dispatch_sizes.0 as u32, self.dispatch_sizes.1 as u32, 1);
    }

    fn transform_1d_1d(compute_chain: &ComputeChain, device: &wgpu::Device, in_buff: wgpu::BufferSlice, in_matrix: wgpu::BufferSlice, out_buff: wgpu::BufferSlice) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputCurve {{
    vec4 in_buff[];
}};

layout(set = 0, binding = 1) buffer InputMatrix {{
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

    fn transform_1d_multi(compute_chain: &ComputeChain, device: &wgpu::Device, in_buff: wgpu::BufferSlice, in_matrix: wgpu::BufferSlice, out_buff: wgpu::BufferSlice) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputBuffer {{
    float in_buff[];
}};

layout(set = 0, binding = 1) buffer InputBuffer {{
    mat4 in_matrix[];
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

{header}

void main() {{
    uint index = gl_GlobalInvocationID.x;
    out_buff[index] = in_matrix[index] * in_buff[index];
}}
"##, header=&compute_chain.shader_header, dimx=LOCAL_SIZE_X);
        println!("debug info for 1d multi 1d transform shader: \n{}", shader_source);
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

    fn transform_2d_2d(compute_chain: &ComputeChain, device: &wgpu::Device, in_buff: wgpu::BufferSlice, in_matrix: wgpu::BufferSlice, out_buff: wgpu::BufferSlice,
                     ) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
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

    fn transform_1d_up2(compute_chain: &ComputeChain, device: &wgpu::Device, in_buff: wgpu::BufferSlice, in_matrix: wgpu::BufferSlice, out_buff: wgpu::BufferSlice,
                     ) -> (wgpu::ComputePipeline, wgpu::BindGroup) {
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

layout(set = 0, binding = 0) buffer InputBuffer {{
    float in_buff[];
}};

layout(set = 0, binding = 1) buffer InputBuffer {{
    mat4 in_matrix[];
}};

layout(set = 0, binding = 2) buffer OutputBuffer {{
    vec4 out_buff[];
}};

{header}

void main() {{
    // the output index should be the same as the common 2D -> 2D transform
    uint index = gl_GlobalInvocationID.x + gl_WorkGroupSize.x * gl_GlobalInvocationID.y;
    // while the index used for accessing the inputs are the global invocation id for x and y
    out_buff[index] = in_matrix[gl_GlobalInvocationID.y] * in_buff[glGlobalInvocationID.x];
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
