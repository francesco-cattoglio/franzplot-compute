use super::camera::Camera;
use super::texture;
use super::device_manager;
use wgpu::util::DeviceExt;
use crate::compute_block::Dimensions;

pub fn create_grid_buffer_index(x_size: usize, y_size: usize, flag_pattern: bool) -> Vec<u32> {
    // the grid has indices growing first along x, then along y
    let mut index_buffer = Vec::<u32>::new();
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
                // triangulate the quad using the other pattern
                index_buffer.push(bot_left_idx);
                index_buffer.push(bot_right_idx);
                index_buffer.push(top_left_idx);

                index_buffer.push(top_right_idx);
                index_buffer.push(top_left_idx);
                index_buffer.push(bot_right_idx);
            } else {
                // triangulate the quad using the "standard" pattern
                index_buffer.push(bot_left_idx);
                index_buffer.push(bot_right_idx);
                index_buffer.push(top_right_idx);

                index_buffer.push(top_right_idx);
                index_buffer.push(top_left_idx);
                index_buffer.push(bot_left_idx);
            }
        }
    }

    index_buffer
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SurfaceVertex {
    position: [f32; 3],
    uv_coords: [f32; 2],
    normal: [f32; 3],
}

unsafe impl bytemuck::Pod for SurfaceVertex {}
unsafe impl bytemuck::Zeroable for SurfaceVertex {}

impl SurfaceVertex {
    fn description<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<SurfaceVertex>() as wgpu::BufferAddress,
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


#[repr(C)]
#[derive(Copy, Clone)]
struct Uniforms {
    view_proj: ultraviolet::Mat4,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    fn new() -> Self {
        Self {
            view_proj: ultraviolet::Mat4::identity(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix();
    }
}

// Approximative code structure:
// we get a "surface model" object which will contain:
// - the index buffer for out surface
// - a material to use
// - either the output of our surface compute node used as a vertex buffer,
//   plus some normals informations
// - or a shader that computes normals on the fly (can be tricky, just imagine
// the issues for normal computation for a parametrix sphere or for z=sqrt(x + y))

struct SurfaceRenderer {
    pipeline: wgpu::RenderPipeline,
    model: SurfaceMesh,
    texture: texture::Texture,
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture: texture::Texture,
    clear_color: wgpu::Color,
}

impl SurfaceRenderer {
    pub fn new(manager: &device_manager::Manager, dimensions: &Dimensions, computed_positions: &wgpu::Buffer) -> Self {
        let texture_bind_group_layout =
            manager.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        count: None,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            component_type: wgpu::TextureComponentType::Float,
                            dimension: wgpu::TextureViewDimension::D2,
                        },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        count: None,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler { comparison: false },
                    },
                ],
                label: Some("texture bind group layout"),
            });

        let camera = Camera {
            eye: (-3.2, 1.6, 1.5).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: (0.0, 1.0, 0.0).into(),
            aspect: manager.sc_desc.width as f32 / manager.sc_desc.height as f32,
            fov_y: 45.0,
            z_near: 0.1,
            z_far: 100.0,
        };

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);

        let camera_uniform_buffer = manager.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });
        let camera_bind_layout =
            manager.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        count: None,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer {
                            dynamic: false,
                            min_binding_size: None,
                        },
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        count: None,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            min_binding_size: None,
                            readonly: false,
                        },
                    },
                ],
                label: Some("camera bind group layout"),
            });
        let camera_bind_group = manager.device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &camera_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer (camera_uniform_buffer.slice(..)),
                },
            ],
            label: Some("Camera bind group"),
        });
        use anyhow::Context;
        let path = std::path::Path::new("/home/franz/rust/franzplot-compute/resources/cube-diffuse.jpg");
        let diffuse_texture = texture::Texture::load(&manager.device, &manager.queue, path, "cube-diffuse").context("failed to load texture").unwrap();

        let render_pipeline_layout =
            manager.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                push_constant_ranges: &[],
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_layout]
            });

        // shader compiling
        let mut shader_compiler = shaderc::Compiler::new().unwrap();
        let vert_src = include_str!("surface_shader.vert");
        let frag_src = include_str!("surface_shader.frag");
        let vert_spirv = shader_compiler.compile_into_spirv(vert_src, shaderc::ShaderKind::Vertex, "surface_shader.vert", "main", None).unwrap();
        let frag_spirv = shader_compiler.compile_into_spirv(frag_src, shaderc::ShaderKind::Fragment, "surface_shader.frag", "main", None).unwrap();
        let vert_data = wgpu::util::make_spirv(vert_spirv.as_binary_u8());
        let frag_data = wgpu::util::make_spirv(frag_spirv.as_binary_u8());
        let vert_module = manager.device.create_shader_module(vert_data);
        let frag_module = manager.device.create_shader_module(frag_data);

        let pipeline = manager.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
            layout: Some(&render_pipeline_layout),
            label: None,
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vert_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &frag_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                clamp_depth: false,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor{
                format: manager.sc_desc.format,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL
            }],
            depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilStateDescriptor {
                    front: wgpu::StencilStateFaceDescriptor::IGNORE,
                    back: wgpu::StencilStateFaceDescriptor::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                }
            }),
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint32,
                vertex_buffers: &[SurfaceVertex::description()],
            },
        });

        let depth_texture = texture::Texture::create_depth_texture(&manager.device, &manager.sc_desc, "depth_texture");

        let clear_color = wgpu::Color::BLACK;

        let model = SurfaceMesh::new(&manager.device, dimensions, computed_positions);

        Self {
            clear_color,
            model,
            texture: diffuse_texture,
            depth_texture,
            camera_uniform_buffer,
            camera_bind_group,
            pipeline,
        }
    }
}

struct SurfaceMesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
}

impl SurfaceMesh {

    pub fn new(device: &wgpu::Device, dimensions: &Dimensions, computed_positions: &wgpu::Buffer) -> Self {
        // The index buffer is the easy part
        let (param_1, param_2) = dimensions.as_2d().unwrap();
        let index_vector = create_grid_buffer_index(param_1.size, param_2.size, true);
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&index_vector),
                usage: wgpu::BufferUsage::INDEX,
            });

        // the vertex part however is a bit more tricky, if we want to interleave informations like
        // vector coordinates or computing the normals we need to fetch the data inside the
        // buffer returned from the compute shader, elaborate it and put it inside the vertex
        // buffer.
        let computed_copy = super::copy_buffer_as_f32(computed_positions, device);
        let vertex_vector: Vec<SurfaceVertex>;
        for j in 0..param_2.size {
            for i in 0..param_1.size {
                let idx = i + param_1.size * j;
                vertex_vector.push(SurfaceVertex {
                    normal: [0.0, 0.0, 1.0],
                    position: [computed_copy[idx], computed_copy[idx+1], computed_copy[idx+2]],
                    uv_coords: [i as f32 / param_1.size as f32, j as f32 / param_2.size as f32]
                });
            }
        }
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&vertex_vector),
                usage: wgpu::BufferUsage::VERTEX,
            });


        Self {
            name: "Surface".to_string(),
            index_buffer,
            vertex_buffer,
            num_elements: (param_1.size * param_2.size) as u32,
        }
    }
}
