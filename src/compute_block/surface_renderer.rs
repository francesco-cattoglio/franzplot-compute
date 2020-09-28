use crate::compute_chain::ComputeChain;
use crate::rendering::{Vertex, GLSL_VERTEX_STRUCT};
use super::ComputeBlock;
use super::SurfaceData;
use super::Dimensions;
use serde::{Deserialize, Serialize};

const LOCAL_SIZE_X: usize = 16;
const LOCAL_SIZE_Y: usize = 16;

#[derive(Debug, Deserialize, Serialize)]
pub struct SurfaceRendererBlockDescriptor {
    pub surface_id: String,
}
impl SurfaceRendererBlockDescriptor {
    pub fn to_block(&self, chain: &ComputeChain, device: &wgpu::Device) -> ComputeBlock {
        ComputeBlock::SurfaceRenderer(SurfaceRendererData::new(chain, device, &self))
    }
}

pub struct SurfaceRendererData {
    pub vertex_buffer: wgpu::Buffer,
    pub compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}
impl SurfaceRendererData {
    pub fn new(compute_chain: &ComputeChain, device: &wgpu::Device, descriptor: &SurfaceRendererBlockDescriptor) -> Self {

        let surface_block = compute_chain.chain.get(&descriptor.surface_id).expect("unable to find dependency for surface renderer block");
        let surface_data: &SurfaceData;
        if let ComputeBlock::Surface(data) = surface_block {
            surface_data = data;
        } else {
            panic!("internal error");
        }
        let computed_surface = &surface_data.out_buffer;
        let dimensions = surface_data.out_dim.clone();

        let vertex_buffer = dimensions.create_storage_buffer(std::mem::size_of::<Vertex>(), device);
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

{vertex_struct}

layout(set = 0, binding = 0) buffer InputVertices {{
    vec4 in_buff[];
}};

layout(set = 0, binding = 1) buffer OutputData {{
    vec4 out_buff[];
}};


void main() {{
    // this shader prepares the data for surface rendering.
    // output data will have the following format:
    // for each vertex, we have a vec4 representing the position,
    // then a vec4 representing the normal

    // normal computation is done computing the tangent and cotangent of the surface via finite differences
    // and then crossing the two vectors.
    uint x_size = gl_NumWorkGroups.x * gl_WorkGroupSize.x;
    uint y_size = gl_NumWorkGroups.y * gl_WorkGroupSize.y;

    // I still need to test how bad the performance can be when branching inside a compute shader.
    uint i = gl_GlobalInvocationID.x;
    uint j = gl_GlobalInvocationID.y;
    uint idx = i + j * x_size;
    vec3 x_tangent;
    if (i == 0) {{
        x_tangent = (-1.5*in_buff[idx] + 2.0*in_buff[idx+1] - 0.5*in_buff[idx+2]).xyz;
    }} else if (i == x_size-1) {{
        x_tangent = ( 1.5*in_buff[idx] - 2.0*in_buff[idx-1] + 0.5*in_buff[idx-2]).xyz;
    }} else {{
        x_tangent = (-0.5*in_buff[idx-1] + 0.5*in_buff[idx+1]).xyz;
    }}
    vec3 y_tangent;
    if (j == 0) {{
        y_tangent = (-1.5*in_buff[idx] + 2.0*in_buff[idx+x_size] - 0.5*in_buff[idx+2*x_size]).xyz;
    }} else if (j == y_size-1) {{
        y_tangent = ( 1.5*in_buff[idx] - 2.0*in_buff[idx-x_size] + 0.5*in_buff[idx-2*x_size]).xyz;
    }} else {{
        y_tangent = (-0.5*in_buff[idx-x_size] + 0.5*in_buff[idx+x_size]).xyz;
    }}

    vec3 crossed = cross(y_tangent, x_tangent);
    float len = length(crossed);
    vec3 normal = (len > 1e-4) ? 1.0/len*crossed : vec3(0.0, 0.0, 0.0);
    out_buff[idx*3] = in_buff[idx];
    out_buff[idx*3+1] = vec4(normal.xyz, 0.0);
    out_buff[idx*3+2] = vec4(i/(x_size-1.0), j/(y_size-1.0), 0.0, 0.0);
}}
"##, vertex_struct=GLSL_VERTEX_STRUCT, dimx=LOCAL_SIZE_X, dimy=LOCAL_SIZE_Y,);
        //println!("debug info for surface rendering shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffers
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: computed_surface.slice(..)
        });
        use crate::shader_processing::*;
        // add descriptor for output buffer
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer_slice: vertex_buffer.slice(..)
        });
        let (compute_pipeline, compute_bind_group) = compute_shader_no_globals(&shader_source, &bindings, &device, Some("Surface Normals"));

        Self {
            compute_pipeline,
            compute_bind_group,
            out_dim: dimensions,
            vertex_buffer,
        }
    }

    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass();
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
        let (par_1, par_2) = self.out_dim.as_2d().unwrap();
        compute_pass.dispatch((par_1.size/LOCAL_SIZE_X) as u32, (par_2.size/LOCAL_SIZE_Y) as u32, 1);
    }
}
