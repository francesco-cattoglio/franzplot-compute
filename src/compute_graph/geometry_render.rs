use std::collections::BTreeMap;
use super::Operation;
use crate::computable_scene::globals::Globals;
use crate::rendering::{StandardVertexData};
use super::{MatcapData, ProcessingError};
use super::Parameter;
use super::{DataID, Data, NodeID};
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};
use crate::node_graph::AVAILABLE_SIZES;

pub type GeometryResult = Result<(MatcapData, Operation), ProcessingError>;

pub fn create(
    device: &wgpu::Device,
    data_map: &BTreeMap<DataID, Data>,
    geometry_id: Option<DataID>,
    thickness: usize,
    output_id: DataID,
) -> GeometryResult {
    println!("new geometry rendering processing");
    let data_id = geometry_id.ok_or(ProcessingError::InputMissing(" This Curve node \n is missing its input "))?;
    let found_data = data_map.get(&data_id).ok_or(ProcessingError::InternalError("Geometry used as input does not exist in the block map".into()))?;

    match found_data {
        Data::Geom1D {
            buffer, param
        } => handle_1d(device, buffer, param.size, thickness, output_id),
        Data::Geom2D {
            ..
        } => todo!(),
        Data::Prefab {
            ..
        } => todo!(),
        _ => return Err(ProcessingError::InternalError("Geometry render operation cannot handle the kind of data provided as input".into()))
    }
}

fn handle_1d(device: &wgpu::Device, input_buffer: &wgpu::Buffer, size: usize, thickness: usize, graph_node_id: NodeID) -> GeometryResult {

    let section_diameter = AVAILABLE_SIZES[thickness];
    let n_section_points = (thickness + 3)*2;

    let (index_buffer, index_count) = create_curve_buffer_index(device, size, n_section_points);
    let vertex_buffer = util::create_storage_buffer(device, size * n_section_points * std::mem::size_of::<StandardVertexData>());

    let curve_consts = create_curve_shader_constants(section_diameter/2.0, n_section_points);
    let wgsl_source = format!(r##"
struct MatcapVertex {{
    position: vec4<f32>;
    normal: vec4<f32>;
    uv_coords: vec2<f32>;
    _padding: vec2<f32>;
}};

[[block]] struct InputBuffer {{
positions: array<vec4<f32>>;
}};

[[block]] struct OutputBuffer {{
vertices: array<MatcapVertex>;
}};

[[group(0), binding(0)]] var<storage, read> _in: InputBuffer;
[[group(0), binding(1)]] var<storage, read_write> _out: OutputBuffer;

var<workgroup> tangent_buff: array<vec3<f32>, {dimx}>;
var<workgroup> ref_buff: array<vec3<f32>, {dimx}>;

{curve_constants}

[[stage(compute), workgroup_size({n_points})]]
fn main([[builtin(global_invocation_id)]] _global_id: vec3<u32>) {{
    // this shader prepares the data for curve rendering.

    let x_size: i32 = {dimx};

    let idx: i32 = i32(_global_id.x);

    var tangent: vec3<f32>;
    if (idx == 0) {{
        tangent = (-1.5*_in.positions[idx] + 2.0*_in.positions[idx + 1] - 0.5*_in.positions[idx + 2]).xyz;
    }} elseif (idx == x_size - 1) {{
        tangent = ( 1.5*_in.positions[idx] - 2.0*_in.positions[idx - 1] + 0.5*_in.positions[idx - 2]).xyz;
    }} else {{
        tangent = (-0.5*_in.positions[idx - 1] + 0.5*_in.positions[idx+1]).xyz;
    }}

    tangent = normalize(tangent);
    tangent_buff[idx] = tangent;

    workgroupBarrier();

    if (idx == 0) {{
        var ref_curr: vec3<f32>;
        if (abs(tangent.x) > 0.2) {{
            ref_curr = vec3<f32>(0.0, 0.0, 1.0);
        }} else {{
            ref_curr = vec3<f32>(1.0, 0.0, 0.0);
        }}
        for (var i: i32 = 0; i < x_size; i = i + 1) {{
            let next_dir: vec3<f32> = tangent_buff[i];
            // TODO: handle 90 degrees curve
            ref_buff[i] = normalize(ref_curr - next_dir * dot(ref_curr, next_dir));
            ref_curr = ref_buff[i];
        }}
    }}

    workgroupBarrier();

    // now all the compute threads can access the ref_buff, which contains a reference
    // vector for every frame. Each thread computes the transformed section.
    let section_position: vec4<f32> = _in.positions[idx];
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
    // workaround WGSL limitations: assign section_points to a temporary var
    // so that we can index it with dinamic indices
    for (var i: i32 = 0; i < {n_points}; i = i + 1) {{
        // the curve section is written as list of vec2 constant points, turn them into actual positions
        // or directions and multiply them by the transform matrix. Note that the new_basis
        // is orthonormal, so there is no need to compute the inverse transpose
        let out_idx = idx * {n_points} + i;
        let section_point = vec3<f32>(0.0, sp[i].x, sp[i].y);
        _out.vertices[out_idx].position = new_basis * vec4<f32>(section_point, 1.0);
        _out.vertices[out_idx].normal = new_basis * vec4<f32>(normalize(section_point), 0.0);
        _out.vertices[out_idx].uv_coords = vec2<f32>(f32(idx)/(f32(x_size) - 1.0), f32(i)/(f32({n_points}) - 1.0));
        _out.vertices[out_idx]._padding = vec2<f32>(0.123, 0.456);
    }}
}}
"##, curve_constants=curve_consts, n_points=n_section_points, dimx=size);

    println!("shader source:\n {}", &wgsl_source);
    // We are creating a curve from an interval, output vertex count is the same as interval
    // one, but buffer size is 4 times as much, because we are storing a Vec4 instead of a f32

    let bind_info = vec![
        BindInfo {
            buffer: &input_buffer,
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
        index_buffer,
        index_count,
        mask_id: 0,
        material_id: 0,
        graph_node_id,
    };
    let operation = Operation {
        bind_group,
        pipeline,
        dim: [1, 1, 1],
    };

    Ok((renderable, operation))
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
            usage: wgpu::BufferUsages::INDEX,
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
            usage: wgpu::BufferUsages::INDEX,
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
    let mut shader_consts = String::new();
    shader_consts += &format!("let section_points = array<vec2<f32>, {n}> (\n", n=n_section_points);
    for i in 0 .. n_section_points {
        let theta = 2.0 * std::f32::consts::PI * i as f32 / (n_section_points - 1) as f32;
        shader_consts += &format!("\tvec2<f32>({:#?}, {:#?}),\n", radius*theta.cos(), radius*theta.sin() );
    }
    shader_consts += &format!(");\n");
    shader_consts += &format!("var sp: array<vec2<f32>, {n}> = section_points;\n", n=n_section_points);

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
            usage: wgpu::BufferUsages::INDEX,
    });
    (index_buffer, index_vector.len() as u32)
}

