use crate::computable_scene::globals::Globals;
use crate::rendering::model::{ Model, MODEL_CHUNK_VERTICES };
use crate::rendering::{ GLSL_STANDARD_VERTEX_STRUCT, StandardVertexData };
use crate::shader_processing::*;
use super::{ ComputeBlock, BlockCreationError, Dimensions };
use super::ProcessingResult;

#[derive(Debug)]
pub struct PrefabBlockDescriptor {
    pub size: String,
    pub prefab_id: i32,
}

impl PrefabBlockDescriptor {
    pub fn make_block(self, device: &wgpu::Device, models: &[Model], globals: &Globals) -> ProcessingResult {
        Ok(ComputeBlock::Prefab(PrefabData::new(device, models, globals, self)?))
    }
}

pub struct PrefabData {
    pub out_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}

impl PrefabData {
    pub fn new(device: &wgpu::Device, models: &[Model], globals: &Globals, descriptor: PrefabBlockDescriptor) -> Result<Self, BlockCreationError> {
        if descriptor.size.is_empty() {
            return Err(BlockCreationError::IncorrectAttributes(" please provide a value \n for the primitive size "));
        }

        dbg!(models.len());
        dbg!(descriptor.prefab_id);
        let model = models.get(descriptor.prefab_id as usize).unwrap();

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {n_chunk_vertices}, local_size_y = 1) in;

{vertex_struct}

layout(set = 0, binding = 0) buffer InputBuffer {{
    Vertex in_buff[];
}};

layout(set = 0, binding = 1) buffer OutputBuffer {{
    Vertex out_buff[];
}};

{globals_header}

void main() {{
    uint idx = gl_GlobalInvocationID.x;

    // TODO: maybe just use the w coordinate to do the uniform scaling
    float scale_factor = {scaling};
    vec3 scaled_pos = scale_factor * in_buff[idx].position.xyz;
    out_buff[idx].position = vec4(scaled_pos, 1.0);
    out_buff[idx].normal = in_buff[idx].normal;
    out_buff[idx].uv_coords = in_buff[idx].uv_coords;
    out_buff[idx]._padding = in_buff[idx]._padding;
}}
"##, globals_header=&globals.shader_header, vertex_struct=GLSL_STANDARD_VERTEX_STRUCT, n_chunk_vertices = MODEL_CHUNK_VERTICES, scaling=&descriptor.size
);

        let out_dim = Dimensions::D3(model.vertex_count, descriptor.prefab_id);
        let out_buffer = out_dim.create_storage_buffer(std::mem::size_of::<StandardVertexData>(), device);

        let bindings = [
            // add descriptor for input buffer
            CustomBindDescriptor {
                position: 0,
                buffer_slice: model.vertex_buffer.slice(..)
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 1,
                buffer_slice: out_buffer.slice(..)
            }
        ];

        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, Some(&globals.bind_layout), Some("Interval"))?;
        Ok(Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
        })
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass();
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
        compute_pass.set_bind_group(1, variables_bind_group, &[]);
        let (vertex_count, _) = self.out_dim.as_3d().unwrap();
        compute_pass.dispatch((vertex_count/MODEL_CHUNK_VERTICES) as u32, 1, 1);
    }
}
