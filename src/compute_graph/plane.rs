use std::collections::BTreeMap;
use std::rc::Rc;

use super::{MatcapData, Operation};
use crate::computable_scene::globals::Globals;
use crate::rendering::model::{Model, MODEL_CHUNK_VERTICES};
use crate::rendering::{StandardVertexData};
use super::Parameter;
use super::{DataID, Data, NodeID};
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};
use crate::node_graph::AVAILABLE_SIZES;
use super::{SingleDataResult, ProcessingError};


pub fn create(
    device: &wgpu::Device,
    data_map: &BTreeMap<DataID, Data>,
    center: Option<DataID>,
    normal: Option<DataID>,
    side_length: usize,
) -> SingleDataResult {

    let data_id = center.ok_or(ProcessingError::InputMissing(" This Plane node \n is missing its point input "))?;
    let found_center = data_map.get(&data_id).ok_or(ProcessingError::InternalError("Geometry used as input does not exist in the block map".into()))?;
    let center_buffer = match found_center {
        Data::Geom0D { buffer } => buffer,
        _ => return Err(ProcessingError::IncorrectInput(" Plane first input \n is not a point "))
    };

    let data_id = normal.ok_or(ProcessingError::InputMissing(" This Plane node \n is missing its normal input "))?;
    let found_normal = data_map.get(&data_id).ok_or(ProcessingError::InternalError("Vector used as input does not exist in the block map".into()))?;
    let normal_buffer = match found_normal {
        Data::Vector { buffer } => buffer,
        _ => return Err(ProcessingError::IncorrectInput(" Plane second input \n is not a vector "))
    };


    // create a vertex and an index buffer to be used for the plane prefab
    let (mut prefab_vertices, plane_indices) = plane_prefab(side_length as f32);
    let vertices_remainder = prefab_vertices.len() % MODEL_CHUNK_VERTICES;
    if vertices_remainder != 0 {
        // extend the vertices to round up its size up to the next MODEL_CHUNK_VERTICES
        let new_size = prefab_vertices.len() + (MODEL_CHUNK_VERTICES - vertices_remainder);
        prefab_vertices.resize_with(new_size, StandardVertexData::default);
    }

    use wgpu::util::DeviceExt;
    let prefab_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
        label: Some("model vertex buffer"),
        contents: bytemuck::cast_slice(&prefab_vertices),
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::MAP_READ,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
        label: Some("model index buffer"),
        contents: bytemuck::cast_slice(&plane_indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    let chunks_count = prefab_vertices.len() / MODEL_CHUNK_VERTICES;
    assert!(prefab_vertices.len() % MODEL_CHUNK_VERTICES == 0);
    let wgsl_source = format!(r##"
struct MatcapVertex {{
    position: vec4<f32>;
    normal: vec4<f32>;
    uv_coords: vec2<f32>;
    padding: vec2<f32>;
}};

[[block]] struct PointBuffer {{
    position: vec4<f32>;
}};

[[block]] struct NormalBuffer {{
    direction: vec4<f32>;
}};

[[block]] struct VertexBuffer {{
    vertices: array<MatcapVertex>;
}};

[[group(0), binding(0)]] var<storage, read> center: PointBuffer;
[[group(0), binding(1)]] var<storage, read> normal: NormalBuffer;
[[group(0), binding(2)]] var<storage, read> prefab: VertexBuffer;
[[group(0), binding(3)]] var<storage, read_write> out_buff: VertexBuffer;

[[stage(compute), workgroup_size({vertices_per_chunk})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    let index: u32 = global_id.x;

    let direction = normalize(normal.direction);

    // TODO: DRY: the same workaround and most of the code is taken from vector_render.rs
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
        center.position, // 4th column
    );

    // There is no need to compute the plane normal as rotation:
    // it is known from the input!
    out_buff.vertices[index].position = matrix * prefab.vertices[index].position;
    out_buff.vertices[index].normal = normal.direction;
    out_buff.vertices[index].uv_coords = prefab.vertices[index].uv_coords;
    out_buff.vertices[index].padding = prefab.vertices[index].padding;
}}
"##, vertices_per_chunk = MODEL_CHUNK_VERTICES,
);
    // println!("prefab shader source:\n {}", &wgsl_source);

    let out_buffer = util::create_storage_buffer(device, std::mem::size_of::<StandardVertexData>() * prefab_vertices.len());

    let bind_info = [
        BindInfo {
            buffer: &center_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true, }
        },
        BindInfo {
            buffer: &normal_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true, }
        },
        BindInfo {
            buffer: &prefab_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true, }
        },
        BindInfo {
            buffer: &out_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false, }
        },
    ];

    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let new_data = Data::Prefab {
        vertex_buffer: out_buffer,
        chunks_count,
        index_count: plane_indices.len() as u32,
        index_buffer: Rc::new(index_buffer),
    };
    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [chunks_count as u32, 1, 1],
    };

    Ok((new_data, operation))
}

fn plane_prefab(side_length: f32) -> (Vec<StandardVertexData>, Vec<u32>) {
    let indices: Vec<u32> = vec![0, 1, 2, 2, 3, 0];

    let uv_scaling = 1.0/8.0;
    let corner_uv = side_length as f32 * uv_scaling;

    let vertices = vec![
        StandardVertexData{
            position: [-side_length/2.0, -side_length/2.0, 0.0, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            uv_coords: [0.0, 0.0],
            _padding: [2.22, 3.33],
        },
        StandardVertexData{
            position: [side_length/2.0, -side_length/2.0, 0.0, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            uv_coords: [corner_uv, 0.0],
            _padding: [2.22, 3.33],
        },
        StandardVertexData{
            position: [side_length/2.0, side_length/2.0, 0.0, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            uv_coords: [corner_uv, corner_uv],
            _padding: [2.22, 3.33],
        },
        StandardVertexData{
            position: [-side_length/2.0, side_length/2.0, 0.0, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            uv_coords: [0.0, corner_uv],
            _padding: [2.22, 3.33],
        },
    ];

    (vertices, indices)
}
