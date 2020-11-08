use crate::rendering::{SurfaceVertex, GLSL_VERTEX_STRUCT};
use crate::rendering::compute_block_processing::create_shader_constants;
use super::{ComputeBlock, BlockCreationError, Dimensions, BlockId};
use serde::{Deserialize, Serialize};
use super::{ProcessedMap, ProcessingResult};

const LOCAL_SIZE_X: usize = 16;
const LOCAL_SIZE_Y: usize = 16;

#[derive(Debug, Deserialize, Serialize)]
pub struct SurfaceRendererBlockDescriptor {
    pub surface: Option<BlockId>,
}
impl SurfaceRendererBlockDescriptor {
    pub fn to_block(&self, device: &wgpu::Device, processed_blocks: &ProcessedMap) -> ProcessingResult {
        Ok(ComputeBlock::SurfaceRenderer(SurfaceRendererData::new(device, processed_blocks, &self)?))
    }

    pub fn get_input_ids(&self) -> Vec<BlockId> {
        match self.surface {
            Some(id) => vec![id],
            None => vec![]
        }
    }
}

// TODO: do we really want to have this here, or should it belong to the rendering system?
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CurveProcessingData {
    position: [f32; 4],
    tangent: [f32; 4],
    normal: [f32; 4],
}

pub const GLSL_CURVEDATA_STRUCT: & str = r##"
struct CurveData {
    vec4 position;
    vec4 normal;
    vec4 tangent;
};
"##;


pub struct SurfaceRendererData {
    pub out_buffer: wgpu::Buffer,
    pub compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}
impl SurfaceRendererData {
    pub fn new(device: &wgpu::Device, processed_blocks: &ProcessedMap, descriptor: &SurfaceRendererBlockDescriptor) -> Result<Self, BlockCreationError> {
        let input_id = descriptor.surface.ok_or(BlockCreationError::InputMissing(" This Renderer node \n has no input "))?;
        let found_element = processed_blocks.get(&input_id).ok_or(BlockCreationError::InternalError("Renderer input does not exist in the block map"))?;
        let input_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;

        match input_block {
            ComputeBlock::Point(point_data) => {
                Self::setup_0d_geometry(device, &point_data.out_buffer, &point_data.out_dim)
            }
            ComputeBlock::Curve(curve_data) => {
                Self::setup_1d_geometry(device, &curve_data.out_buffer, &curve_data.out_dim)
            }
            ComputeBlock::Surface(surface_data) => {
                Self::setup_2d_geometry(device, &surface_data.out_buffer, &surface_data.out_dim)
            }
            ComputeBlock::Transform(transformed_data) => {
                let buffer = &transformed_data.out_buffer;
                let dimensions = &transformed_data.out_dim;
                match dimensions {
                    Dimensions::D0 => Self::setup_0d_geometry(device, buffer, dimensions),
                    Dimensions::D1(_) => Self::setup_1d_geometry(device, buffer, dimensions),
                    Dimensions::D2(_, _) => Self::setup_2d_geometry(device, buffer, dimensions),
                }
            }
            _ => Err(BlockCreationError::InputInvalid("the input provided to the Renderer is not a geometry kind"))
        }
    }

    fn setup_0d_geometry(_device: &wgpu::Device, _data_buffer: &wgpu::Buffer, _dimensions: &Dimensions) -> Result<Self, BlockCreationError> {
        unimplemented!("point rendering not implemented yet")
    }

    fn setup_1d_geometry(device: &wgpu::Device, data_buffer: &wgpu::Buffer, dimensions: &Dimensions) -> Result<Self, BlockCreationError> {
        let size = dimensions.as_1d().unwrap().size;
        let section_radius = 0.25;
        let n_section_points = 4usize;
        let vertexdata_buffer = dimensions.create_storage_buffer(n_section_points*std::mem::size_of::<SurfaceVertex>(), device);
        let shader_consts = create_shader_constants(section_radius, n_section_points);
        dbg!(&shader_consts);
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

{vertex_struct}

shared vec3 mid_buff[{dimx}];
shared vec3 ref_buff[{dimx}];

layout(set = 0, binding = 0) buffer InputVertices {{
    vec4 in_buff[];
}};

layout(set = 0, binding = 1) buffer OutputData {{
    Vertex out_buff[];
}};

{shader_constants}

void main() {{
    // this shader prepares the data for curve rendering.

    uint x_size = gl_NumWorkGroups.x * gl_WorkGroupSize.x;

    uint idx = gl_GlobalInvocationID.x;

    vec3 tangent;
    if (idx == 0) {{
        tangent = (-1.5*in_buff[idx] + 2.0*in_buff[idx+1] - 0.5*in_buff[idx+2]).xyz;
    }} else if (idx == x_size-1) {{
        tangent = ( 1.5*in_buff[idx] - 2.0*in_buff[idx-1] + 0.5*in_buff[idx-2]).xyz;
    }} else {{
        tangent = (-0.5*in_buff[idx-1] + 0.5*in_buff[idx+1]).xyz;
    }}

    float t_len = length(tangent);

    mid_buff[idx] = tangent/t_len;

    memoryBarrierShared();
    barrier();

    if (idx == 0) {{
        // TODO: better choice of starting vector, this one fails if t = [0, 0, 1]
        vec3 ref_curr = vec3(0.0, 0.0, 1.0);
        vec3 ref_next = ref_curr;
        for (int i = 0; i < x_size; i++) {{
            vec3 next_dir = mid_buff[i];
            // TODO: handle 90 degrees curve
            ref_next = normalize(ref_curr - next_dir * dot(ref_curr, next_dir));
            ref_buff[i] = ref_next;
            ref_curr = ref_next;
        }}
    }}

    memoryBarrierShared();
    barrier();

    // now all the compute threads can access the ref_buff, which contains a reference
    // vector for every frame. Each thread computes the transformed section.
    vec4 section_position = in_buff[idx];
    // compute the three directions for the frame: forward
    vec4 frame_forward = vec4(tangent, 0.0);
    // up direction
    vec3 ref_vector = ref_buff[idx];
    vec4 frame_up = vec4(ref_vector, 0.0);
    // and left direction
    vec3 left_dir = -1.0 * normalize(cross(frame_forward.xyz, frame_up.xyz));
    vec4 frame_left = vec4(left_dir, 0.0);
    // we can now assemble the matrix that we will be using to transform all the section points

    mat4 new_basis;
    new_basis[0] = frame_forward;
    new_basis[1] = frame_left;
    new_basis[2] = frame_up;
    new_basis[3] = section_position;
    for (int i = 0; i < {n_points}; i++) {{
        // the constants making up the section are vec2, turn them into vec3 or vec4 and multiply
        // them by the transform matrix
        uint out_idx = idx * {n_points} + i;
        vec3 section_point = vec3(0.0, section_points[i].x, section_points[i].y);
        out_buff[out_idx].position = new_basis * vec4(section_point, 1.0);
        out_buff[out_idx].normal = vec4(normalize(section_point), 0.0);
        out_buff[out_idx].uv_coords = vec2(0.4, 0.5);
        out_buff[out_idx]._padding = vec2(0.123, 0.456);
    }}
}}
"##, vertex_struct=GLSL_VERTEX_STRUCT, shader_constants=shader_consts, n_points=n_section_points, dimx=size);
        println!("debug info for curve rendering shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffers
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: data_buffer.slice(..)
        });
        use crate::shader_processing::*;
        // add descriptor for output buffer
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer_slice: vertexdata_buffer.slice(..)
        });
        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, &shader_source, &bindings, None, Some("Curve Normals"))?;

        Ok(Self {
            compute_pipeline,
            compute_bind_group,
            out_dim: dimensions.clone(),
            out_buffer: vertexdata_buffer,
        })
    }

    fn setup_2d_geometry(device: &wgpu::Device, data_buffer: &wgpu::Buffer, dimensions: &Dimensions) -> Result<Self, BlockCreationError> {
        let vertex_buffer = dimensions.create_storage_buffer(std::mem::size_of::<SurfaceVertex>(), device);
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = {dimy}) in;

{vertex_struct}

layout(set = 0, binding = 0) buffer InputVertices {{
    vec4 in_buff[];
}};

layout(set = 0, binding = 1) buffer OutputData {{
    Vertex out_buff[];
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

    /* TODO: investigate the best criterion for deciding when to zero out the normal vector.
     * If we get it wrong, we might produce artifacts (black spots) even in very simple cases,
     * e.g: sin(x) or a planar surface which has been subdivided a lot
     * First criterion: normalize the two tangents (or zero them out if they are very short)
     * we zero them out in two slightly different ways but according to RenderDoc
     * the disassembly is almost identical.
     */

    // float x_len = length(x_tangent);
    // x_tangent *= (x_len > 1e-6) ? 1.0/x_len : 0.0;
    // float y_len = length(y_tangent);
    // y_tangent = (y_len > 1e-6) ? 1.0/y_len*y_tangent : vec3(0.0, 0.0, 0.0);
    // vec3 crossed = cross(y_tangent, x_tangent);
    // float len = length(crossed);
    // vec3 normal = (len > 1e-3) ? 1.0/len*crossed : vec3(0.0, 0.0, 0.0);

    /* Second criterion measure the length of the two tangents, cross them, check if the
     * length of the cross is far smaller then the product of the two lengths.
     */

    vec3 normal = cross(y_tangent, x_tangent);
    float len_x = length(x_tangent);
    float len_y = length(y_tangent);
    float len_n = length(normal);
    normal = (len_n > 1e-3 * len_x * len_y) ? 1.0/len_n*normal : vec3(0.0, 0.0, 0.0);

    out_buff[idx].position = in_buff[idx];
    out_buff[idx].normal = vec4(normal, 0.0);
    out_buff[idx].uv_coords = vec2(i/(x_size-1.0), j/(y_size-1.0));
    out_buff[idx]._padding = vec2(0.0, 0.0);
}}
"##, vertex_struct=GLSL_VERTEX_STRUCT, dimx=LOCAL_SIZE_X, dimy=LOCAL_SIZE_Y);
        println!("debug info for surface rendering shader: \n{}", shader_source);
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffers
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: data_buffer.slice(..)
        });
        use crate::shader_processing::*;
        // add descriptor for output buffer
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer_slice: vertex_buffer.slice(..)
        });
        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, &shader_source, &bindings, None, Some("Surface Normals"))?;

        Ok(Self {
            compute_pipeline,
            compute_bind_group,
            out_dim: dimensions.clone(),
            out_buffer: vertex_buffer,
        })

    }

    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass();
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
        match &self.out_dim {
            Dimensions::D0 => {
                unimplemented!();
            }
            Dimensions::D1(_par_1) => {
                // BEWARE: as described before, we wrote the size of the buffer inside the local shader
                // dimensions, therefore the whole compute will always take just 1 dispatch
                compute_pass.dispatch(1, 1, 1);
            }
            Dimensions::D2(par_1, par_2) => {
                compute_pass.dispatch((par_1.size/LOCAL_SIZE_X) as u32, (par_2.size/LOCAL_SIZE_Y) as u32, 1);
            }
        }
    }
}
