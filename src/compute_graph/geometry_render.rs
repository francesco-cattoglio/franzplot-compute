use std::collections::BTreeMap;
use std::rc::Rc;

use crate::rendering::model::MODEL_CHUNK_VERTICES;
use super::Operation;
use crate::rendering::StandardVertexData;
use super::{MatcapData, ProcessingError};
use super::Parameter;
use super::{DataID, Data};
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};
use crate::node_graph::AVAILABLE_SIZES;

pub type MatcapResult = Result<(MatcapData, Operation), ProcessingError>;

pub fn create(
    device: &wgpu::Device,
    data_map: &BTreeMap<DataID, Data>,
    geometry_id: Option<DataID>,
    thickness: usize,
    mask: usize,
    material: usize,
) -> MatcapResult {
    let data_id = geometry_id
        .ok_or_else(|| ProcessingError::InputMissing(" This Curve node \n is missing its input ".into()))?;
    let found_data = data_map
        .get(&data_id)
        .ok_or(ProcessingError::NoInputData)?;

    match found_data {
        Data::Geom0D { buffer,
        } => handle_0d(device, buffer, thickness, material),
        Data::Geom1D {
            buffer, param,
        } => handle_1d(device, buffer, param.n_points(), thickness, mask, material),
        Data::Geom2D {
            buffer, param1, param2,
        } => handle_2d(device, buffer, param1, param2, mask, material),
        Data::Prefab {
            vertex_buffer, chunks_count, index_buffer, index_count,
        } => handle_prefab(device, vertex_buffer, *chunks_count, index_buffer, *index_count, mask, material),
        _ => Err(ProcessingError::InternalError("Geometry render operation cannot handle the kind of data provided as input".into()))
    }
}

fn handle_0d(device: &wgpu::Device, input_buffer: &wgpu::Buffer, thickness: usize, material_id: usize) -> MatcapResult {
    // Never go above a certain refinement level: the local group size for a compute shader
    // invocation should never exceed 512, due to the requested limits, and with
    // a refine level of 6 we already hit the 492 points count.
    let refine_amount = std::cmp::min(thickness, 6);
    let sphere_radius = AVAILABLE_SIZES[thickness];

    use hexasphere::shapes::IcoSphere;
    let sphere = IcoSphere::new(refine_amount, |_| ());

    let raw_points = sphere.raw_points();
    let vertex_count = raw_points.len();
    let reference_vertices: Vec<glam::Vec4> = raw_points
        .iter()
        .map(|v| {glam::Vec4::new(v.x, v.y, v.z, 0.0)})
        .collect();

    let indices = sphere.get_all_indices();

    use wgpu::util::DeviceExt;
    let reference_vertex_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&reference_vertices),
            usage: wgpu::BufferUsages::STORAGE,
    });
    let index_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
    });

    let wgsl_source = format!(r##"
struct MatcapVertex {{
    position: vec4<f32>;
    normal: vec4<f32>;
    uv_coords: vec2<f32>;
    padding: vec2<f32>;
}};

// input buffer will contain a single vertex, the actual point coords
struct InputBuffer {{
    position: vec4<f32>;
}};

// reference buffer will contain all the deltas needed to turn a single
// point into an actual icosphere that can be rendered to video
// NOTE: we use a storage buffer instead of a uniform to keep
// the code similar to the handle_1d shader.
struct ReferenceBuffer {{
    delta: array<vec4<f32>>;
}};

// output buffer contains the final Matcap mesh, as usual for rendering nodes
struct OutputBuffer {{
    vertices: array<MatcapVertex>;
}};

[[group(0), binding(0)]] var<storage, read> in: InputBuffer;
[[group(0), binding(1)]] var<storage, read> ref: ReferenceBuffer;
[[group(0), binding(2)]] var<storage, read_write> out: OutputBuffer;

[[stage(compute), workgroup_size({dimx})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    // this shader prepares the data for point rendering.
    // there is very little work to do, we just set the final location
    // of the vertices and store the normals.
    let point_coords: vec4<f32> = in.position;
    let normal: vec4<f32> = ref.delta[global_id.x];
    let idx = global_id.x;

    out.vertices[idx].position = point_coords + {radius} * normal;
    out.vertices[idx].normal = normal;
    out.vertices[idx].uv_coords = vec2<f32>(0.0, 0.0);
    out.vertices[idx].padding = vec2<f32>(0.123, 0.456);
}}
"##, radius=sphere_radius, dimx=vertex_count);

    let output_buffer = util::create_storage_buffer(device, vertex_count * std::mem::size_of::<StandardVertexData>());

    let bind_info = vec![
        BindInfo {
            buffer: input_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &reference_vertex_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &output_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let renderable = MatcapData {
        vertex_buffer: output_buffer,
        index_buffer: Rc::new(index_buffer),
        index_count: indices.len() as u32,
        mask_id: 0,
        material_id,
    };
    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [1, 1, 1],
    };

    Ok((renderable, operation))
}

// TODO: maybe we can skip a memory barrier if we do store some extra information in the curve
// geometry, so that we do not need to compute the entire "ref_buff". We also need to handle
// the 90 degree curve anyway, so this code requires a bit of a rework anyway
fn handle_1d(device: &wgpu::Device, input_buffer: &wgpu::Buffer, n_points: usize, thickness: usize, mask_id: usize, material_id: usize) -> MatcapResult {

    let section_diameter = AVAILABLE_SIZES[thickness];
    let n_section_points = (thickness + 3)*2;

    let (index_buffer, index_count) = create_curve_index_buffer(device, n_points, n_section_points);
    let vertex_buffer = util::create_storage_buffer(device, n_points * n_section_points * std::mem::size_of::<StandardVertexData>());

    let reference_vertices = create_curve_reference_points(section_diameter/2.0, n_section_points);
    use wgpu::util::DeviceExt;
    let reference_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&reference_vertices),
            usage: wgpu::BufferUsages::STORAGE,
    });

    let wgsl_source = format!(r##"
struct MatcapVertex {{
    position: vec4<f32>;
    normal: vec4<f32>;
    uv_coords: vec2<f32>;
    padding: vec2<f32>;
}};

struct InputBuffer {{
    positions: array<vec4<f32>>;
}};

// reference buffer will contain the 2D coordinates of the points
// that make up a single section (or slice) of the curve.
// NOTE: we use a storage buffer instead of a uniform due to
// the strict layout limitations on UBOs
struct ReferenceBuffer {{
    coords: array<vec2<f32>>;
}};

struct OutputBuffer {{
    vertices: array<MatcapVertex>;
}};

[[group(0), binding(0)]] var<storage, read> in: InputBuffer;
[[group(0), binding(1)]] var<storage, read> ref: ReferenceBuffer;
[[group(0), binding(2)]] var<storage, read_write> out: OutputBuffer;

var<workgroup> ref_buff: array<vec3<f32>, {dimx}>;

[[stage(compute), workgroup_size({dimx})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    // this shader prepares the data for curve rendering.
    let x_size: i32 = {dimx};

    let idx: i32 = i32(global_id.x);

    if (idx == 0) {{
        var ref_curr: vec3<f32>;
        for (var i: i32 = 0; i < x_size; i = i + 1) {{
            // first: compute the tangent
            var tangent: vec3<f32>;
            if (i == 0) {{
                tangent = (-1.5*in.positions[i] + 2.0*in.positions[i + 1] - 0.5*in.positions[i + 2]).xyz;
            }} else if (i == x_size - 1) {{
                tangent = ( 1.5*in.positions[i] - 2.0*in.positions[i - 1] + 0.5*in.positions[i - 2]).xyz;
            }} else {{
                tangent = (-0.5*in.positions[i - 1] + 0.5*in.positions[i+1]).xyz;
            }}
            tangent = normalize(tangent);

            // initialize ref_curr if we are at the first execution of this loop
            if (i == 0) {{
                if (abs(tangent.x) > 0.2) {{
                    ref_curr = vec3<f32>(0.0, 0.0, 1.0);
                }} else {{
                    ref_curr = vec3<f32>(1.0, 0.0, 0.0);
                }}
            }}

            let next_dir: vec3<f32> = tangent;
            // TODO: handle 90 degrees curve
            ref_buff[i] = normalize(ref_curr - next_dir * dot(ref_curr, next_dir));
            ref_curr = ref_buff[i];
        }}
    }}

    workgroupBarrier();

    var tangent: vec3<f32>;
    if (idx == 0) {{
        tangent = (-1.5*in.positions[idx] + 2.0*in.positions[idx + 1] - 0.5*in.positions[idx + 2]).xyz;
    }} else if (idx == x_size - 1) {{
        tangent = ( 1.5*in.positions[idx] - 2.0*in.positions[idx - 1] + 0.5*in.positions[idx - 2]).xyz;
    }} else {{
        tangent = (-0.5*in.positions[idx - 1] + 0.5*in.positions[idx+1]).xyz;
    }}

    tangent = normalize(tangent);
    // now all the compute threads can access the ref_buff, which contains a reference
    // vector for every frame. Each thread computes the transformed section.
    let section_position: vec4<f32> = in.positions[idx];
    // compute the three directions for the frame: forward direction
    let frame_forward = vec4<f32>(tangent, 0.0);
    // up direction
    let ref_vector: vec3<f32> = ref_buff[idx];
    let frame_up = vec4<f32>(ref_vector, 0.0);
    // and left direction
    let left_dir: vec3<f32> = -1.0 * normalize(cross(frame_forward.xyz, frame_up.xyz));
    let frame_left = vec4<f32>(left_dir, 0.0);
    // we can now assemble the matrix that we will be using to transform all the section points

    let new_basis = mat4x4<f32> (
        frame_forward,
        frame_left,
        frame_up,
        section_position,
    );
    for (var i: i32 = 0; i < {points_per_section}; i = i + 1) {{
        // the curve section is written as list of vec2 constant points, turn them into actual positions
        // or directions and multiply them by the transform matrix. Note that the new_basis
        // is orthonormal, so there is no need to compute the inverse transpose
        let out_idx = idx * {points_per_section} + i;
        let section_point = vec3<f32>(0.0, ref.coords[i].x, ref.coords[i].y);
        out.vertices[out_idx].position = new_basis * vec4<f32>(section_point, 1.0);
        out.vertices[out_idx].normal = new_basis * vec4<f32>(normalize(section_point), 0.0);
        out.vertices[out_idx].uv_coords = vec2<f32>(f32(idx)/(f32(x_size) - 1.0), f32(i)/(f32({points_per_section}) - 1.0));
        out.vertices[out_idx].padding = vec2<f32>(1.123, 1.456);
    }}
}}
"##, points_per_section=n_section_points, dimx=n_points);

    //println!("shader source:\n {}", &wgsl_source);
    // We are creating a curve from an interval, output vertex count is the same as interval
    // one, but buffer size is 4 times as much, because we are storing a Vec4 instead of a f32

    let bind_info = vec![
        BindInfo {
            buffer: input_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &reference_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &vertex_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let renderable = MatcapData {
        vertex_buffer,
        index_buffer: Rc::new(index_buffer),
        index_count,
        mask_id,
        material_id,
    };
    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [1, 1, 1],
    };

    Ok((renderable, operation))
}

fn handle_2d(device: &wgpu::Device, input_buffer: &wgpu::Buffer, param1: &Parameter, param2: &Parameter, mask_id: usize, material_id: usize) -> MatcapResult {
    let flag_pattern = true;
    let (index_buffer, index_count) = create_grid_index_buffer(device, param1.n_points(), param2.n_points(), flag_pattern);
    let vertex_buffer = util::create_storage_buffer(device, param1.n_points() * param2.n_points() * std::mem::size_of::<StandardVertexData>());
    let wgsl_source = format!(r##"
struct MatcapVertex {{
    position: vec4<f32>;
    normal: vec4<f32>;
    uv_coords: vec2<f32>;
    padding: vec2<f32>;
}};

struct InputBuffer {{
    pos: array<vec4<f32>>;
}};

struct OutputBuffer {{
    vertices: array<MatcapVertex>;
}};

[[group(0), binding(0)]] var<storage, read> in: InputBuffer;
[[group(0), binding(1)]] var<storage, read_write> out: OutputBuffer;

[[stage(compute), workgroup_size({pps}, {pps})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    // this shader prepares the data for surface rendering.
    // normal computation is done computing the tangent and cotangent of the surface via finite differences
    // and then crossing the two vectors.
    let x_size = {size_x}u;
    let y_size = {size_y}u;

    let i = global_id.x;
    let j = global_id.y;
    let idx = i + j * x_size;
    var x_tangent: vec3<f32>;
    if (i == 0u) {{
        x_tangent = (-1.5 * in.pos[idx] + 2.0 * in.pos[idx + 1u] - 0.5 * in.pos[idx + 2u]).xyz;
    }} else if (i == x_size - 1u) {{
        x_tangent = ( 1.5 * in.pos[idx] - 2.0 * in.pos[idx - 1u] + 0.5 * in.pos[idx - 2u]).xyz;
    }} else {{
        x_tangent = (-0.5 * in.pos[idx - 1u] + 0.5 * in.pos[idx + 1u]).xyz;
    }}
    var y_tangent: vec3<f32>;
    if (j == 0u) {{
        y_tangent = (-1.5 * in.pos[idx] + 2.0 * in.pos[idx + x_size] - 0.5 * in.pos[idx + 2u * x_size]).xyz;
    }} else if (j == y_size - 1u) {{
        y_tangent = ( 1.5 * in.pos[idx] - 2.0 * in.pos[idx - x_size] + 0.5 * in.pos[idx - 2u * x_size]).xyz;
    }} else {{
        y_tangent = (-0.5 * in.pos[idx - x_size] + 0.5 * in.pos[idx + x_size]).xyz;
    }}

    //// TODO: investigate the best criterion for deciding when to zero out the normal vector.
    ///  If we get it wrong, we might produce artifacts (black spots) even in very simple cases,
    ///  e.g: sin(x) or a planar surface which has been subdivided a lot
    ///  First criterion: normalize the two tangents (or zero them out if they are very short)
    ///  we zero them out in two slightly different ways but according to RenderDoc
    ///  the disassembly is almost identical.
    /// /

    // float x_len = length(x_tangent);
    // x_tangent *= (x_len > 1e-6) ? 1.0/x_len : 0.0;
    // float y_len = length(y_tangent);
    // y_tangent = (y_len > 1e-6) ? 1.0/y_len*y_tangent : vec3(0.0, 0.0, 0.0);
    // vec3 crossed = cross(y_tangent, x_tangent);
    // float len = length(crossed);
    // vec3 normal = (len > 1e-3) ? 1.0/len*crossed : vec3(0.0, 0.0, 0.0);

    //// Second criterion measure the length of the two tangents, cross them, check if the
    ///  length of the cross is far smaller then the product of the two lengths.
    /// /

    var normal = cross(x_tangent, y_tangent);
    let len_x = length(x_tangent);
    let len_y = length(y_tangent);
    let len_n = length(normal);
    if (len_n > 1e-3 * len_x * len_y) {{
        normal = 1.0 / len_n * normal;
    }} else {{
        normal = vec3<f32>(0.0, 0.0, 0.0);
    }}

    let u_coord = f32(i) / f32(x_size - 1u);
    let v_coord = f32(j) / f32(y_size - 1u);

    out.vertices[idx].position = in.pos[idx];
    out.vertices[idx].normal = vec4<f32>(normal, 0.0);
    out.vertices[idx].uv_coords = vec2<f32>(u_coord, v_coord);
    out.vertices[idx].padding = vec2<f32>(2.123, 2.456);
}}
"##, pps=Parameter::POINTS_PER_SEGMENT,
size_x=param1.n_points(), size_y=param2.n_points());

    //println!("2d shader source:\n {}", &wgsl_source);
    // We are creating a curve from an interval, output vertex count is the same as interval
    // one, but buffer size is 4 times as much, because we are storing a Vec4 instead of a f32

    let bind_info = vec![
        BindInfo {
            buffer: input_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &vertex_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [param1.segments, param2.segments, 1],
    };
    let renderable = MatcapData {
        vertex_buffer,
        index_buffer: Rc::new(index_buffer),
        index_count,
        mask_id,
        material_id,
    };

    Ok((renderable, operation))
}

fn handle_prefab(device: &wgpu::Device, vertex_buffer: &wgpu::Buffer, chunks_count: usize, index_buffer: &Rc<wgpu::Buffer>, index_count: u32, mask_id: usize, material_id: usize) -> MatcapResult {

    let wgsl_source = format!(r##"
struct MatcapVertex {{
    position: vec4<f32>;
    normal: vec4<f32>;
    uv_coords: vec2<f32>;
    padding: vec2<f32>;
}};

struct VertexBuffer {{
    vertices: array<MatcapVertex>;
}};

[[group(0), binding(0)]] var<storage, read> in_buff: VertexBuffer;
[[group(0), binding(1)]] var<storage, read_write> out_buff: VertexBuffer;

[[stage(compute), workgroup_size({vertices_per_chunk})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    let index = global_id.x;
    out_buff.vertices[index]= in_buff.vertices[index];
}}
"##, vertices_per_chunk=MODEL_CHUNK_VERTICES,);
    // println!("3d shader source:\n {}", &wgsl_source);

    let out_buffer = util::create_storage_buffer(device, std::mem::size_of::<StandardVertexData>() * chunks_count * MODEL_CHUNK_VERTICES);

    let bind_info = [
        BindInfo {
            buffer: vertex_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &out_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [chunks_count as u32, 1, 1],
    };
    let renderable = MatcapData {
        vertex_buffer: out_buffer,
        index_buffer: Rc::clone(index_buffer),
        index_count: index_count as u32,
        mask_id,
        material_id,
    };

    Ok((renderable, operation))
}

// UTILITY FUNCTIONS
// those are used to:
// - create the index buffers for curve and surface rendering
// - create the default vertices positions for each curve section

fn create_curve_index_buffer(device: &wgpu::Device, x_size: usize, circle_points: usize) -> (wgpu::Buffer, u32) {
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
            // TODO: remove map_read from usage flags
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::MAP_READ,
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

fn create_curve_reference_points(radius: f32, n_section_points: usize) -> Vec<glam::Vec2> {
    let mut reference_points = Vec::<glam::Vec2>::new();
    for i in 0 .. n_section_points {
        let theta = 2.0 * std::f32::consts::PI * i as f32 / (n_section_points - 1) as f32;
        reference_points.push(glam::Vec2::new(radius*theta.cos(), radius*theta.sin() ));
    }
    reference_points
}

fn create_grid_index_buffer(device: &wgpu::Device, x_size: usize, y_size: usize, flag_pattern: bool) -> (wgpu::Buffer, u32) {
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

            // some code is shared between branches, but this makes it easier to read the code IMO
            #[allow(clippy::branches_sharing_code)]
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
            usage: wgpu::BufferUsages::INDEX,
    });
    (index_buffer, index_vector.len() as u32)
}

