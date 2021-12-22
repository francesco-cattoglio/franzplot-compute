use std::rc::Rc;

use super::Operation;
use super::globals::Globals;
use crate::rendering::model::{Model, MODEL_CHUNK_VERTICES};
use crate::rendering::StandardVertexData;
use super::Data;
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};
use super::{SingleDataResult, ProcessingError};


pub fn create(
    device: &wgpu::Device,
    models: &[Model],
    globals: &Globals,
    primitive_id: usize,
    size: String,
) -> SingleDataResult {
    if size.is_empty() {
        return Err(ProcessingError::IncorrectAttributes(" please provide a value \n for the primitive size ".into()));
    }

    // Sanitize all input expressions
    let sanitized_size = globals.sanitize_expression_2(&[], &size)?;

    let model = models.get(primitive_id).unwrap();

    let wgsl_source = format!(r##"
{wgsl_globals}

struct MatcapVertex {{
    position: vec4<f32>;
    normal: vec4<f32>;
    uv_coords: vec2<f32>;
    padding: vec2<f32>;
}};

[[block]] struct VertexBuffer {{
    vertices: array<MatcapVertex>;
}};

[[group(0), binding(1)]] var<storage, read> in_buff: VertexBuffer;
[[group(0), binding(2)]] var<storage, read_write> out_buff: VertexBuffer;

[[stage(compute), workgroup_size({vertices_per_chunk})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    let index: u32 = global_id.x;

    // We could use the w coordinate to just do the uniform scaling, but
    // I would rather not do that to make it easier to debug shaders via RenderDoc.
    let scale_factor: f32 = {scaling};
    let scaled_pos: vec3<f32> = scale_factor * in_buff.vertices[index].position.xyz;
    out_buff.vertices[index].position = vec4<f32>(scaled_pos, 1.0);
    out_buff.vertices[index].normal = in_buff.vertices[index].normal;
    out_buff.vertices[index].uv_coords = in_buff.vertices[index].uv_coords;
    out_buff.vertices[index].padding = in_buff.vertices[index].padding;
}}
"##, wgsl_globals=&globals.get_wgsl_header(), vertices_per_chunk = MODEL_CHUNK_VERTICES, scaling=sanitized_size
);
    // println!("prefab shader source:\n {}", &wgsl_source);

    let out_buffer = util::create_storage_buffer(device, std::mem::size_of::<StandardVertexData>() * model.chunks_count * MODEL_CHUNK_VERTICES);

    let bind_info = [
        globals.get_bind_info(),
        BindInfo {
            buffer: &model.vertex_buffer,
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
        chunks_count: model.chunks_count,
        index_count: model.index_count,
        index_buffer: Rc::clone(&model.index_buffer),
    };
    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [model.chunks_count as u32, 1, 1],
    };

    Ok((new_data, operation))
}
