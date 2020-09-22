use crate::texture;
use anyhow::{Result, Context};

use wgpu::util::DeviceExt;

pub trait Vertex {
    fn description<'a>() -> wgpu::VertexBufferDescriptor<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ModelVertex {
    position: [f32; 3],
    uv_coords: [f32; 2],
    normal: [f32; 3],
}

unsafe impl bytemuck::Pod for ModelVertex {}
unsafe impl bytemuck::Zeroable for ModelVertex {}

impl Vertex for ModelVertex {
    fn description<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3
                },
                wgpu::VertexAttributeDescriptor {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float2
                },
                wgpu::VertexAttributeDescriptor {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float3
                },
            ],
        }
    }
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

impl Model {
    pub fn load<P: AsRef<std::path::Path>>(
        device: &wgpu::Device,
        bindgroup_layout: &wgpu::BindGroupLayout,
        path: P
        ) -> Result<(Self, Vec<wgpu::CommandBuffer>)> {

        dbg!(path.as_ref());
        let (obj_models, obj_materials) = tobj::load_obj(path.as_ref(), true).context("Error loading the obj file")?;

            // we assume the folder also containse the material
        let containing_folder = path.as_ref().parent().unwrap();

        let mut command_buffers = Vec::<wgpu::CommandBuffer>::new();
        let mut materials = Vec::<Material>::new();

        for mat in obj_materials {
            let texture_filename = mat.diffuse_texture;
            let diffuse_texture = texture::Texture::load(device, containing_folder.join(&texture_filename), &texture_filename).context("failed to load texture")?;

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: bindgroup_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                ],
                label: Some("all_materials")
            });

            materials.push(Material {
                name: mat.name,
                diffuse_texture,
                bind_group
            });
            command_buffers.push(command);
        }

        let mut meshes = Vec::<Mesh>::new();
        for model in &obj_models {
            let mut vertices = Vec::<ModelVertex>::new();
            // tobj gives us flat arrays, make them into chunks
            // NOTE: we would like to use array_chunks but it is nightly-only
            let model_positions = model.mesh.positions.chunks_exact(3);
            let model_normals = model.mesh.normals.chunks_exact(3);
            let model_uv = model.mesh.texcoords.chunks_exact(2);
            // check that our chunks make sense
            if model_positions.len() == 0 {
                return Err(anyhow::anyhow!("model has no vertex positions"));
            }
            if model_normals.len() != model_positions.len() {
                return Err(anyhow::anyhow!("model has wrong number of normals"));
            }
            if model_uv.len() != model_positions.len() {
                return Err(anyhow::anyhow!("model has wrong number of texture coordinates"));
            }

            // zip the chunk iterators into a single one, then convert the slices into arrays.
            // we need the TryInto trait, we wouldn't need it if array_chunks was used.
            use itertools::izip;
            use std::convert::TryInto;
            let vertex_iter = itertools::izip!(model_positions, model_normals, model_uv);
            for it in vertex_iter {
                let model_vertex = ModelVertex {
                    position: it.0.try_into().unwrap(),
                    normal: it.1.try_into().unwrap(),
                    uv_coords: it.2.try_into().unwrap(),
                };
                vertices.push(model_vertex);
            }

            // now our vertices vector is ready to be turned into a webgpu buffer
            let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsage::VERTEX,
            });
            // the index buffer is already flat!
            let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&model.mesh.indices),
                usage: wgpu::BufferUsage::INDEX,
            });

            meshes.push(Mesh {
                index_buffer,
                material: model.mesh.material_id.unwrap_or(0),
                name: model.name.clone(),
                num_elements: model.mesh.indices.len() as u32,
                vertex_buffer,
            });
        }

        Ok((Self { meshes, materials }, command_buffers))
    }
}

pub trait DrawModel<'a, 'b>
where 'b : 'a
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        uniforms: &'b wgpu::BindGroup
        );
    fn draw_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: std::ops::Range<u32>,
        uniforms: &'b wgpu::BindGroup
        );
}

impl<'a, 'b> DrawModel<'a, 'b> for wgpu::RenderPass<'a>
where 'b : 'a
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        uniforms: &'b wgpu::BindGroup
        )
    {
        self.draw_instanced(mesh, material, 0..1, uniforms);
    }

    fn draw_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: std::ops::Range<u32>,
        uniforms: &'b wgpu::BindGroup
        ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..));
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, &uniforms, &[]);

        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

}
