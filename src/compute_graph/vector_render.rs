use std::collections::BTreeMap;
use std::rc::Rc;

use crate::rendering::model::MODEL_CHUNK_VERTICES;
use super::Operation;
use crate::rendering::StandardVertexData;
use crate::node_graph::AVAILABLE_SIZES;
use super::{MatcapData, ProcessingError};
use super::{DataID, Data};
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};

pub type MatcapResult = Result<(MatcapData, Operation), ProcessingError>;


pub fn create(
    device: &wgpu::Device,
    data_map: &BTreeMap<DataID, Data>,
    application_point_id: Option<DataID>,
    vector_id: Option<DataID>,
    thickness: usize,
    material_id: usize,
) -> MatcapResult {
    let data_id = application_point_id
        .ok_or_else(|| ProcessingError::InputMissing(" This Vector Rendering node \n is missing its first input ".into()))?;
    let found_appl_point = data_map
        .get(&data_id)
        .ok_or_else(|| ProcessingError::InternalError("Application Point used as input does not exist in the block map".into()))?;
    let appl_point_buffer = if let Data::Geom0D{ buffer } = found_appl_point {
        buffer
    } else {
        return Err(ProcessingError::IncorrectInput(" the first input provided \n is not a point ".into()));
    };

    let data_id = vector_id
        .ok_or_else(|| ProcessingError::InputMissing(" This Vector Rendering node \n is missing its second input ".into()))?;
    let found_vector = data_map
        .get(&data_id)
        .ok_or_else(|| ProcessingError::InternalError("Vector used as input does not exist in the block map".into()))?;
    let vector_buffer = if let Data::Vector{ buffer } = found_vector {
        buffer
    } else {
        return Err(ProcessingError::IncorrectInput(" the second input provided \n is not a vector ".into()));
    };

        // then we create the basic shape of the arrow. It points upwards and has length 1.0
        // we will transform the vertex buffer in our compute shader
    let radius = AVAILABLE_SIZES[thickness];
    let n_circle_points = (thickness + 3)*2;
    let (index_buffer, index_count, prefab_buffer, chunks_count) = create_arrow_buffers(device, radius, n_circle_points);

    let wgsl_source = format!(r##"
struct MatcapVertex {{
    position: vec4<f32>;
    normal: vec4<f32>;
    uv_coords: vec2<f32>;
    padding: vec2<f32>;
}};

// input buffer will contain a single vertex, the actual point coords
[[block]] struct PointBuffer {{
    position: vec4<f32>;
}};

[[block]] struct VectorBuffer {{
    direction: vec4<f32>;
}};

// reference buffer contains all the points for the actual arrow-shaped mesh
// used to represent the vector.
[[block]] struct ReferenceBuffer {{
    vertices: array<MatcapVertex>;
}};

// output buffer contains the final Matcap mesh, as usual for rendering nodes
[[block]] struct OutputBuffer {{
    vertices: array<MatcapVertex>;
}};

[[group(0), binding(0)]] var<storage, read> in_appl_point: PointBuffer;
[[group(0), binding(1)]] var<storage, read> in_vector: VectorBuffer;
[[group(0), binding(2)]] var<storage, read> in_ref: ReferenceBuffer;
[[group(0), binding(3)]] var<storage, read_write> out: OutputBuffer;

[[stage(compute), workgroup_size({vertices_per_chunk})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    let index = global_id.x;

    let vertex: MatcapVertex = in_ref.vertices[index];
    let s_z: f32 = length(in_vector.direction);
    let direction = normalize(in_vector.direction);

    var angle_z: f32;
    // workaround MacOS bug: atan2 seems to give the wrong result
    if (direction.y == 0.0) {{
        if (direction.x > 0.0) {{
            angle_z = -0.5*3.14159265;
        }} else {{
            angle_z =  0.5*3.14159265;
        }}
    }} else {{
        angle_z = -1.0 * atan2(direction.x, direction.y);
    }}

    let angle_x = -1.0 * acos(direction.z);
    let cos_t = cos(angle_z);
    let sin_t = sin(angle_z);
    let cos_p = cos(angle_x);
    let sin_p = sin(angle_x);

    let matrix = mat4x4<f32>(
        vec4<f32>(         cos_t,          sin_t,   0.0, 0.0), // first column
        vec4<f32>(-cos_p * sin_t,  cos_p * cos_t, sin_p, 0.0), // 2nd column
        vec4<f32>( sin_p * sin_t, -sin_p * cos_t, cos_p, 0.0), // 3rd column
        in_appl_point.position, // 4th column
    );

    // We can pretend that the matrix is orthogonal, and apply it to normals as well.
    // BEWARE: this is not correct because the vertex normal should be transformed when we scale
    // the arrow. But the normals were wrong to begin with! (see create_arrow_buffers function)

    out.vertices[index].position = matrix * vec4<f32>(vertex.position.xy, s_z * vertex.position.z, 1.0);
    out.vertices[index].normal = matrix * vertex.normal;
    out.vertices[index].uv_coords = vertex.uv_coords;
    out.vertices[index].padding = vertex.padding;
}}
"##, vertices_per_chunk=MODEL_CHUNK_VERTICES);

    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<StandardVertexData>() * MODEL_CHUNK_VERTICES * chunks_count);

    let bind_info = vec![
        BindInfo {
            buffer: appl_point_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: vector_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &prefab_buffer,
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
        index_count,
        mask_id: 0,
        material_id,
    };
    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [chunks_count as u32, 1, 1],
    };

    Ok((renderable, operation))
}

// TODO: DRY!!! We should take the following code, the code from geometry_rendering.rs
// and find a new place for both. This function in particular could be shared
fn create_arrow_segment(segment: (usize, usize), circle_points: usize) -> Vec::<u32> {
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

fn create_arrow_cap(segment: usize, cap_point_index: u32, circle_points: usize) -> Vec::<u32> {
    let mut indices = Vec::<u32>::new();
    // the variable names are a bit misleading, so here is an explanation:
    // segment_start is the index of the first vertex in the first circle of the segment
    // segment_end is the index of the first vertex in the second circle.
    let segment_start = (segment * circle_points) as u32;

    // first go through all the sides except for the very last one
    for i in 0 .. (circle_points - 1) as u32 {
        // two triangles per each face
        indices.extend_from_slice(&[segment_start + i, segment_start + i + 1, cap_point_index]);
    }
    // then add in the last one. We could have used a % to make sure the output would be correct
    // but it is not worth it, KISS principle!
    indices.extend_from_slice(&[segment_start + circle_points as u32 - 1, segment_start, cap_point_index]);

    indices
}

fn create_arrow_buffers(device: &wgpu::Device, radius: f32, circle_points: usize) -> (wgpu::Buffer, u32, wgpu::Buffer, usize) {
    assert!(circle_points > 3);
    let head_z_start: f32 = 1.0 - 1.5*radius;
    let mut index_vector = Vec::<u32>::new();
    let mut vertex_vector = Vec::<StandardVertexData>::new();

    // first segment: the cylinder part of the arrow
    {
        let segment = (0, 1);
        let mut segment_indices = create_arrow_segment(segment, circle_points);
        index_vector.append(&mut segment_indices);
        // we also compute the corresponding vertices.
        // start with the first circle
        for i in 0 .. circle_points {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / (circle_points - 1) as f32;
            // we add a very tiny amount of bias to z to prevent z-fighting when showing the vector of a plane
            let position = [0.5*radius*theta.cos(), 0.5*radius*theta.sin(), 0.001, 1.0];
            let normal = [theta.cos(), theta.sin(), 0.0, 0.0];
            vertex_vector.push(StandardVertexData {
                position,
                normal,
                uv_coords: [0.0, 0.0],
                _padding: [0.123, 0.456],
            });
        }
        // add the second circle: same normals, different z for the points
        for i in 0 .. circle_points {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / (circle_points - 1) as f32;
            let position = [0.5*radius*theta.cos(), 0.5*radius*theta.sin(), head_z_start, 1.0];
            let normal = [theta.cos(), theta.sin(), 0.0, 0.0];
            vertex_vector.push(StandardVertexData {
                position,
                normal,
                uv_coords: [0.0, 0.0],
                _padding: [0.123, 0.456],
            });
        }
    }
    // second segment: the expanding part of the arrow
    {
        let segment = (2, 3);
        let mut segment_indices = create_arrow_segment(segment, circle_points);
        index_vector.append(&mut segment_indices);
        // we need to add the points that correspond to the circle number 2. The ones for the
        // add the circle number 2: normals look downwards
        for i in 0 .. circle_points {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / (circle_points - 1) as f32;
            let position = [0.5*radius*theta.cos(), 0.5*radius*theta.sin(), head_z_start, 1.0];
            let normal = [0.0, 0.0, -1.0, 0.0];
            vertex_vector.push(StandardVertexData {
                position,
                normal,
                uv_coords: [0.0, 0.0],
                _padding: [0.123, 0.456],
            });
        }
        // This time we change and use the entire radius, instead of only half,
        // and set the normals to look downwards
        for i in 0 .. circle_points {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / (circle_points - 1) as f32;
            let position = [radius*theta.cos(), radius*theta.sin(), head_z_start, 1.0];
            let normal = [0.0, 0.0, -1.0, 0.0];
            vertex_vector.push(StandardVertexData {
                position,
                normal,
                uv_coords: [0.0, 0.0],
                _padding: [0.123, 0.456],
            });
        }
    }
    // last part: the hat of the arrow
    // before proceding, we want to create a new circle, because we need to duplicate the vertices
    // (to show a proper edge we need to split the normals!)
    // we don't point the normals correctly, we just point them outwards, which should
    // look good enough.
    {
        for i in 0 .. circle_points {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / (circle_points - 1) as f32;
            let position = [radius*theta.cos(), radius*theta.sin(), head_z_start, 1.0];
            let normal = [theta.cos(), theta.sin(), 0.0, 0.0];
            vertex_vector.push(StandardVertexData {
                position,
                normal,
                uv_coords: [0.0, 0.0],
                _padding: [0.123, 0.456],
            });
        }
        // now, add the final point for the cap.
        vertex_vector.push(StandardVertexData {
            position: [0.0, 0.0, 1.0, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            uv_coords: [0.0, 0.0],
            _padding: [0.123, 0.456],
        });
        // and create the indices for it
        let circle = 4;
        let last_vertex_index = (vertex_vector.len() - 1) as u32;
        let mut cap_indices = create_arrow_cap(circle, last_vertex_index, circle_points);
        index_vector.append(&mut cap_indices);
    }
    // one-pass-last part: close off the bottom of the the arrow
    {
        vertex_vector.push(StandardVertexData {
            position: [0.0, 0.0, 0.0, 1.0],
            normal: [0.0, 0.0, -1.0, 0.0],
            uv_coords: [0.0, 0.0],
            _padding: [0.123, 0.456],
        });
        // and create the indices for it
        let circle = 0;
        let last_vertex_index = (vertex_vector.len() - 1) as u32;
        // BEWARE: this will have the triangle to come out reversed, i.e. with CW ordering
        // instead of CCW
        let mut cap_indices = create_arrow_cap(circle, last_vertex_index, circle_points);
        index_vector.append(&mut cap_indices);
    }

    // Add as many empty vertices as we need to round up the buffer size up to the next MODEL_CHUNK_VERTICES
    let vertices_remainder = vertex_vector.len() % MODEL_CHUNK_VERTICES;
    if vertices_remainder != 0 {
        let new_size = vertex_vector.len() + (MODEL_CHUNK_VERTICES - vertices_remainder);
        vertex_vector.resize_with(new_size, StandardVertexData::default);
    }

    use wgpu::util::DeviceExt;
    let index_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&index_vector),
            usage: wgpu::BufferUsages::INDEX,
    });
    let vertex_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertex_vector),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::MAP_READ,
    });
    dbg!(vertex_vector.len());
    (index_buffer, index_vector.len() as u32, vertex_buffer, vertex_vector.len() / MODEL_CHUNK_VERTICES)
}

