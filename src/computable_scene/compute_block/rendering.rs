use crate::computable_scene::globals::Globals;
use crate::rendering::{StandardVertexData, GLSL_STANDARD_VERTEX_STRUCT};
use crate::node_graph::AVAILABLE_SIZES;
use crate::rendering::model::{ Model, MODEL_CHUNK_VERTICES };
use super::{ComputeBlock, BlockCreationError, Dimensions, BlockId};
use super::{ProcessedMap, ProcessingResult};

const LOCAL_SIZE_X: usize = 16;
const LOCAL_SIZE_Y: usize = 16;

#[derive(Debug)]
pub struct RenderingBlockDescriptor {
    pub geometry: Option<BlockId>,
    pub thickness: usize,
    pub mask: usize,
    pub material: usize,
}
impl RenderingBlockDescriptor {
    pub fn make_block(self, device: &wgpu::Device, globals: &Globals, models: &[Model], processed_blocks: &ProcessedMap) -> ProcessingResult {
        Ok(ComputeBlock::Rendering(RenderingData::new(device, globals, models, processed_blocks, self)?))
    }
}

pub struct RenderingData {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: Option<wgpu::Buffer>,
    pub index_count: u32,
    pub mask_id: usize,
    pub material_id: usize,
    pub compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}

impl RenderingData {
    // TODO: the only data we are extracting from models right now is index buffer count, we can
    // remove it and just forward the prefab_id to the scene renderer and let it handle it the same
    // way it handles the index_buffer
    pub fn new(device: &wgpu::Device, globals: &Globals, models: &[Model], processed_blocks: &ProcessedMap, descriptor: RenderingBlockDescriptor) -> Result<Self, BlockCreationError> {
        let input_id = descriptor.geometry.ok_or(BlockCreationError::InputMissing(" This Renderer node \n has no input "))?;
        let found_element = processed_blocks.get(&input_id).ok_or(BlockCreationError::InternalError("Renderer input does not exist in the block map"))?;
        let input_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;

        match input_block {
            ComputeBlock::Point(point_data) => {
                Self::setup_0d_geometry(device, globals, &point_data.out_buffer, descriptor)
            }
            ComputeBlock::Bezier(bezier_data) => {
                Self::setup_1d_geometry(device, globals, &bezier_data.out_buffer, &bezier_data.out_dim, descriptor)
            }
            ComputeBlock::Curve(curve_data) => {
                Self::setup_1d_geometry(device, globals, &curve_data.out_buffer, &curve_data.out_dim, descriptor)
            }
            ComputeBlock::Surface(surface_data) => {
                Self::setup_2d_geometry(device, globals, &surface_data.out_buffer, &surface_data.out_dim, descriptor)
            }
            ComputeBlock::Prefab(prefab_data) => {
                Self::setup_3d_geometry(device, globals, models, &prefab_data.out_buffer, &prefab_data.out_dim, descriptor)
            }
            ComputeBlock::Transform(transformed_data) => {
                let buffer = &transformed_data.out_buffer;
                let dimensions = &transformed_data.out_dim;
                match dimensions {
                    Dimensions::D0 => Self::setup_0d_geometry(device, globals, buffer, descriptor),
                    Dimensions::D1(_) => Self::setup_1d_geometry(device, globals, buffer, dimensions, descriptor),
                    Dimensions::D2(_, _) => Self::setup_2d_geometry(device, globals, buffer, dimensions, descriptor),
                    Dimensions::D3(_, _) => Self::setup_3d_geometry(device, globals, models, buffer, dimensions, descriptor)
                }
            }
            _ => Err(BlockCreationError::InputInvalid("the input provided to the Renderer is not a geometry kind"))
        }
    }

    fn setup_0d_geometry(device: &wgpu::Device, globals: &Globals, data_buffer: &wgpu::Buffer, descriptor: RenderingBlockDescriptor) -> Result<Self, BlockCreationError> {
        // Never go above a certain refinement level: the local group size for a compute shader
        // invocation should never exceed 512, otherwise the GPU might error out, and with
        // a refine level of 6 we already hit the 492 points count.
        // TODO: on mobile group size limit might be 256, and max refine might be 4
        let refine_amount = std::cmp::min(descriptor.thickness, 6);
        let sphere_radius = AVAILABLE_SIZES[descriptor.thickness];

        let (shader_consts, vertices_count, index_buffer, index_count) = create_point_data(device, refine_amount);

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

{vertex_struct}

layout(set = 0, binding = 0) buffer InputVertices {{
    vec4 in_buff[];
}};

layout(set = 0, binding = 1) buffer OutputData {{
    Vertex out_buff[];
}};

{shader_constants}

void main() {{
    // this shader prepares the data for point rendering.
    // there is very little work to do, we just translate the constants
    // to their correct location and store the normals.
    vec4 point_coords = in_buff[0];
    uint idx = gl_GlobalInvocationID.x;

    vec4 normal = vec4(sphere_points[idx], 0.0);
    out_buff[idx].position = point_coords + {radius}*normal;
    out_buff[idx].normal = normal;
    out_buff[idx].uv_coords = vec2(0.0, 0.0);
    out_buff[idx]._padding = vec2(0.123, 0.456);
}}
"##, vertex_struct=GLSL_STANDARD_VERTEX_STRUCT, radius=sphere_radius, shader_constants=shader_consts, dimx=vertices_count);

        // BEWARE: for 0D dimensions, the buffer that will be created is just the
        // "size of the element" that we pass in. For this reason, we need to put the size of
        // a single vertex times the number of vertices that we used to represent the point
        let out_dim = Dimensions::D0;
        let vertex_buffer = out_dim.create_storage_buffer(vertices_count*std::mem::size_of::<StandardVertexData>(), device);

        let bindings = [
            // add descriptor for input buffer
            CustomBindDescriptor {
                position: 0,
                buffer: &data_buffer
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 1,
                buffer: &vertex_buffer
            }
        ];

        use crate::shader_processing::*;
        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, &shader_source, &bindings, Some(&globals.bind_layout), Some("Curve Normals"))?;

        Ok(Self {
            mask_id: descriptor.mask,
            material_id: descriptor.material,
            compute_pipeline,
            compute_bind_group,
            out_dim,
            vertex_buffer,
            index_buffer: Some(index_buffer),
            index_count
        })
    }

    fn setup_1d_geometry(device: &wgpu::Device, globals: &Globals, data_buffer: &wgpu::Buffer, dimensions: &Dimensions, descriptor: RenderingBlockDescriptor) -> Result<Self, BlockCreationError> {
        let section_diameter = AVAILABLE_SIZES[descriptor.thickness];
        let n_section_points = (descriptor.thickness + 3)*2;

        let size = dimensions.as_1d()?.size;
        let (index_buffer, index_count) = create_curve_buffer_index(device, size, n_section_points);
        let vertex_buffer = dimensions.create_storage_buffer(n_section_points*std::mem::size_of::<StandardVertexData>(), device);

        let shader_consts = create_curve_shader_constants(section_diameter/2.0, n_section_points);
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

{vertex_struct}

shared vec3 tangent_buff[{dimx}];
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

    tangent = normalize(tangent);
    tangent_buff[idx] = tangent;

    memoryBarrierShared();
    barrier();

    if (idx == 0) {{
        vec3 ref_curr = (abs(tangent.x) > 0.2) ? vec3(0.0, 0.0, 1.0) : vec3(1.0, 0.0, 0.0);
        for (int i = 0; i < x_size; i++) {{
            vec3 next_dir = tangent_buff[i];
            // TODO: handle 90 degrees curve
            ref_buff[i] = normalize(ref_curr - next_dir * dot(ref_curr, next_dir));
            ref_curr = ref_buff[i];
        }}
    }}

    memoryBarrierShared();
    barrier();

    // now all the compute threads can access the ref_buff, which contains a reference
    // vector for every frame. Each thread computes the transformed section.
    vec4 section_position = in_buff[idx];
    // compute the three directions for the frame: forward direction
    vec4 frame_forward = vec4(tangent, 0.0);
    // up direction
    vec3 ref_vector = ref_buff[idx];
    vec4 frame_up = vec4(ref_vector, 0.0);
    // and left direction
    vec3 left_dir = -1.0 * normalize(cross(frame_forward.xyz, frame_up.xyz));
    vec4 frame_left = vec4(left_dir, 0.0);
    // we can now assemble the matrix that we will be using to transform all the section points

    mat4 new_basis = {{
        frame_forward,
        frame_left,
        frame_up,
        section_position,
    }};
    for (int i = 0; i < {n_points}; i++) {{
        // the curve section is written as list of vec2 constant points, turn them into actual positions
        // or directions and multiply them by the transform matrix. Note that the new_basis
        // is orthonormal, so there is no need to compute the inverse transpose
        uint out_idx = idx * {n_points} + i;
        vec3 section_point = vec3(0.0, section_points[i].x, section_points[i].y);
        out_buff[out_idx].position = new_basis * vec4(section_point, 1.0);
        out_buff[out_idx].normal = new_basis * vec4(normalize(section_point), 0.0);
        out_buff[out_idx].uv_coords = vec2(idx/(x_size-1.0), i/({n_points}-1.0));
        out_buff[out_idx]._padding = vec2(0.123, 0.456);
    }}
}}
"##, vertex_struct=GLSL_STANDARD_VERTEX_STRUCT, shader_constants=shader_consts, n_points=n_section_points, dimx=size);

        let bindings = [
            // add descriptor for input buffer
            CustomBindDescriptor {
                position: 0,
                buffer: &data_buffer
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 1,
                buffer: &vertex_buffer
            }
        ];

        use crate::shader_processing::*;
        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, &shader_source, &bindings, Some(&globals.bind_layout), Some("Curve Normals"))?;

        Ok(Self {
            mask_id: descriptor.mask,
            material_id: descriptor.material,
            compute_pipeline,
            compute_bind_group,
            out_dim: dimensions.clone(),
            vertex_buffer,
            index_buffer: Some(index_buffer),
            index_count
        })
    }

    fn setup_2d_geometry(device: &wgpu::Device, globals: &Globals, data_buffer: &wgpu::Buffer, dimensions: &Dimensions, descriptor: RenderingBlockDescriptor) -> Result<Self, BlockCreationError> {
        let (param_1, param_2) = dimensions.as_2d()?;
        let flag_pattern = true;
        let (index_buffer, index_count) = create_grid_buffer_index(device, param_1.size, param_2.size, flag_pattern);
        let vertex_buffer = dimensions.create_storage_buffer(std::mem::size_of::<StandardVertexData>(), device);
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

{globals_header}

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

    vec3 normal = cross(x_tangent, y_tangent);
    float len_x = length(x_tangent);
    float len_y = length(y_tangent);
    float len_n = length(normal);
    normal = (len_n > 1e-3 * len_x * len_y) ? 1.0/len_n*normal : vec3(0.0, 0.0, 0.0);

    float u_coord, v_coord;
    if ({i_interval_uv}) {{
        float u_delta = ({i_interval_end} - {i_interval_begin}) / (x_size - 1.0);
        u_coord = {i_interval_begin} + u_delta * i;
    }} else {{
        u_coord = i/(x_size-1.0);
    }}
    if ({j_interval_uv}) {{
        float v_delta = ({j_interval_end} - {j_interval_begin}) / (y_size - 1.0);
        v_coord = {j_interval_begin} + v_delta * j;
    }} else {{
        v_coord = j/(y_size-1.0);
    }}

    out_buff[idx].position = in_buff[idx];
    out_buff[idx].normal = vec4(normal, 0.0);
    out_buff[idx].uv_coords = vec2(u_coord, v_coord);
    out_buff[idx]._padding = vec2(0.0, 0.0);
}}
"##, globals_header=&globals.shader_header, vertex_struct=GLSL_STANDARD_VERTEX_STRUCT, dimx=LOCAL_SIZE_X, dimy=LOCAL_SIZE_Y,
i_interval_begin=&param_1.begin, i_interval_end=&param_1.end, i_interval_uv=param_1.use_interval_as_uv,
j_interval_begin=&param_2.begin, j_interval_end=&param_2.end, j_interval_uv=param_2.use_interval_as_uv);

        let bindings = [
            // add descriptor for input buffers
            CustomBindDescriptor {
                position: 0,
                buffer: &data_buffer
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 1,
                buffer: &vertex_buffer
            }
        ];

        use crate::shader_processing::*;
        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, &shader_source, &bindings, Some(&globals.bind_layout), Some("Surface Normals"))?;

        Ok(Self {
            mask_id: descriptor.mask,
            material_id: descriptor.material,
            compute_pipeline,
            compute_bind_group,
            out_dim: dimensions.clone(),
            vertex_buffer,
            index_buffer: Some(index_buffer),
            index_count
        })

    }

    fn setup_3d_geometry(device: &wgpu::Device, globals: &Globals, models: &[Model], data_buffer: &wgpu::Buffer, dimensions: &Dimensions, descriptor: RenderingBlockDescriptor) -> Result<Self, BlockCreationError> {
        let (vertex_count, prefab_id) = dimensions.as_3d()?;
        let model = models.get(prefab_id as usize).unwrap();

        let index_count = model.index_count;

        let vertex_buffer = dimensions.create_storage_buffer(std::mem::size_of::<StandardVertexData>(), device);

        // TODO: we need to find a way to just issue a copy of a buffer at encoding stage, instead
        // of running a compute shader that does nothing other then copy stuff over.
        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

{vertex_struct}

layout(set = 0, binding = 0) buffer InputVertices {{
    Vertex in_buff[];
}};

layout(set = 0, binding = 1) buffer OutputVertices {{
    Vertex out_buff[];
}};

void main() {{
    uint idx = gl_GlobalInvocationID.x;
    out_buff[idx]= in_buff[idx];
}}
"##, vertex_struct=GLSL_STANDARD_VERTEX_STRUCT, dimx=MODEL_CHUNK_VERTICES,);

        let bindings = [
            // add descriptor for input buffers
            CustomBindDescriptor {
                position: 0,
                buffer: &data_buffer
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 1,
                buffer: &vertex_buffer
            }
        ];

        use crate::shader_processing::*;
        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, &shader_source, &bindings, Some(&globals.bind_layout), Some("Copy 3D data"))?;

        Ok(Self {
            mask_id: descriptor.mask,
            material_id: descriptor.material,
            compute_pipeline,
            compute_bind_group,
            out_dim: dimensions.clone(),
            vertex_buffer,
            index_buffer: None,
            index_count
        })

    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{
            label: Some("rendering compute pass")
        });
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
        compute_pass.set_bind_group(1, variables_bind_group, &[]);
        match &self.out_dim {
            Dimensions::D0 => {
                // BEWARE: as described before, we wrote the size of the buffer inside the local shader
                // dimensions, therefore the whole compute will always take just 1 dispatch
                compute_pass.dispatch(1, 1, 1);
            }
            Dimensions::D1(_par_1) => {
                // BEWARE: as described before, we wrote the size of the buffer inside the local shader
                // dimensions, therefore the whole compute will always take just 1 dispatch
                compute_pass.dispatch(1, 1, 1);
            }
            Dimensions::D2(par_1, par_2) => {
                compute_pass.dispatch((par_1.size/LOCAL_SIZE_X) as u32, (par_2.size/LOCAL_SIZE_Y) as u32, 1);
            }
            Dimensions::D3(vertex_count, _prefab_id) => {
                compute_pass.dispatch((vertex_count/MODEL_CHUNK_VERTICES) as u32, 1, 1);
            }
        }
    }
}

// UTILITY FUNCTIONS
// those are used to:
// - create the index buffers for point, curve and surface rendering
// - create the default vertices for the icosahedron representing the point
// - create the default vertices positions for each curve section

fn create_point_data(device: &wgpu::Device, refine: usize) -> (String, usize, wgpu::Buffer, u32) {
    use hexasphere::shapes::IcoSphere;
    let sphere = IcoSphere::new(refine, |_| ());

    let points = sphere.raw_points();
    let point_count = points.len();
    dbg!(point_count);
    let mut shader_consts = String::new();
    shader_consts += &format!("const vec3 sphere_points[{n}] = {{\n", n=point_count);
    for p in points {
        shader_consts += &format!("\tvec3({x}, {y}, {z}),\n", x=p.x, y=p.y, z=p.z );
    }
    shader_consts += &format!("}};\n");
    let indices = sphere.get_all_indices();

    use wgpu::util::DeviceExt;
    let index_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsage::INDEX,
    });
    (shader_consts, point_count, index_buffer, indices.len() as u32)
}

fn create_curve_buffer_index(device: &wgpu::Device, x_size: usize, circle_points: usize) -> (wgpu::Buffer, u32) {
    assert!(circle_points > 3);
    let mut index_vector = Vec::<u32>::new();

    for i in 0 .. x_size - 1 {
        let segment = (i, i+1);
        let mut segment_indices = create_curve_segment(segment, circle_points);
        index_vector.append(&mut segment_indices);
    }

    // TODO: add caps

    use wgpu::util::DeviceExt;
    let index_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&index_vector),
            usage: wgpu::BufferUsage::INDEX,
    });
    (index_buffer, index_vector.len() as u32)
}

fn create_curve_segment(segment: (usize, usize), circle_points: usize) -> Vec::<u32> {
    let mut indices = Vec::<u32>::new();
    // the variable names are a bit misleading, so here is an explanation:
    // segment_start is the index of the first vertex in the first circle of the segment
    // segment_end is the index of the first vertex in the second circle.
    let segment_start = (segment.0 * circle_points) as u32;
    let segment_end = (segment.1 * circle_points) as u32;

    // first go through all the sides except for the very last one
    for i in 0 .. (circle_points - 1) as u32 {
        // two triangles per each face
        indices.extend_from_slice(&[segment_start + i, segment_start + i + 1, segment_end + i + 1]);
        indices.extend_from_slice(&[segment_start + i, segment_end + i + 1, segment_end + i]);
    }
    // then add in the last one. We could have used a % to make sure the output would be correct
    // but it is not worth it, KISS principle!
    indices.extend_from_slice(&[segment_end - 1, segment_start, segment_end]);
    indices.extend_from_slice(&[segment_end + (circle_points - 1) as u32, segment_end - 1, segment_end]);

    indices
}

fn create_curve_shader_constants(radius: f32, n_section_points: usize) -> String {
    // TODO: verify that an extra comma is *ALWAYS* allowed in GLSL initializer lists
    let mut shader_consts = String::new();
    shader_consts += &format!("const vec2 section_points[{n}] = {{\n", n=n_section_points);
    for i in 0 .. n_section_points {
        let theta = 2.0 * std::f32::consts::PI * i as f32 / (n_section_points - 1) as f32;
        shader_consts += &format!("\tvec2({x}, {y}),\n", x=radius*theta.cos(), y=radius*theta.sin() );
    }
    shader_consts += &format!("}};\n");

    shader_consts
}

fn create_grid_buffer_index(device: &wgpu::Device, x_size: usize, y_size: usize, flag_pattern: bool) -> (wgpu::Buffer, u32) {
    // the grid has indices growing first along x, then along y
    let mut index_vector = Vec::<u32>::new();
    let num_triangles_x = x_size - 1;
    let num_triangles_y = y_size - 1;
    for j in 0..num_triangles_y {
        for i in 0..num_triangles_x {
            // process every quad element of the grid by producing 2 triangles
            let bot_left_idx =  ( i  +   j   * x_size) as u32;
            let bot_right_idx = (i+1 +   j   * x_size) as u32;
            let top_left_idx =  ( i  + (j+1) * x_size) as u32;
            let top_right_idx = (i+1 + (j+1) * x_size) as u32;

            if (i+j)%2==1 && flag_pattern {
                // triangulate the quad using the "flag" pattern
                index_vector.push(bot_left_idx);
                index_vector.push(bot_right_idx);
                index_vector.push(top_left_idx);

                index_vector.push(top_right_idx);
                index_vector.push(top_left_idx);
                index_vector.push(bot_right_idx);
            } else {
                // triangulate the quad using the "standard" pattern
                index_vector.push(bot_left_idx);
                index_vector.push(bot_right_idx);
                index_vector.push(top_right_idx);

                index_vector.push(top_right_idx);
                index_vector.push(top_left_idx);
                index_vector.push(bot_left_idx);
            }
        }
    }

    use wgpu::util::DeviceExt;
    let index_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&index_vector),
            usage: wgpu::BufferUsage::INDEX,
    });
    (index_buffer, index_vector.len() as u32)
}

