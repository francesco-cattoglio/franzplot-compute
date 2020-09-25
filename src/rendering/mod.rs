use super::camera::Camera;
use super::texture;
use super::device_manager;
use crate::compute_block::ComputeBlock;
use wgpu::util::DeviceExt;

mod compute_block_processing;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    position: [f32; 4],
    normal: [f32; 4],
    uv_coords: [f32; 2],
    _padding: [f32; 2],
}

pub const GLSL_VERTEX_STRUCT: &'static str = r##"
struct Vertex {
    vec4 position;
    vec4 normal;
    vec2 uv_coords;
    vec2 _padding;
};
"##;

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    fn description<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        wgpu::VertexBufferDescriptor {
            stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float3
                },
                wgpu::VertexAttributeDescriptor {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float4
                },
                wgpu::VertexAttributeDescriptor {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float2
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

pub struct SurfaceRenderer {
    pipeline: wgpu::RenderPipeline,
    renderables: Vec<wgpu::RenderBundle>,
    texture: texture::Texture,
    texture_bind_group: wgpu::BindGroup,
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture: texture::Texture,
    clear_color: wgpu::Color,
}

const TEXTURE_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
wgpu::BindGroupLayoutDescriptor {
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
};
const CAMERA_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
wgpu::BindGroupLayoutDescriptor {
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
                ],
                label: Some("camera bind group layout"),
            };


impl SurfaceRenderer {
    pub fn new(manager: &device_manager::Manager, computed_surface: &ComputeBlock) -> Self {
        let texture_bind_group_layout =
            manager.device.create_bind_group_layout(&TEXTURE_LAYOUT_DESCRIPTOR);

        let camera = Camera {
            eye: (3.5, 3.5, 3.5).into(),
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
            manager.device.create_bind_group_layout(&CAMERA_LAYOUT_DESCRIPTOR);
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
        let path = std::path::Path::new("/home/franz/rust/franzplot-compute/resources/grid_color.png");
        let diffuse_texture = texture::Texture::load(&manager.device, &manager.queue, path, "cube-diffuse").context("failed to load texture").unwrap();
        let texture_bind_group = manager.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
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
                vertex_buffers: &[Vertex::description()],
            },
        });

        let depth_texture = texture::Texture::create_depth_texture(&manager.device, &manager.sc_desc, "depth_texture");

        let clear_color = wgpu::Color::BLACK;

        let renderables = Vec::<wgpu::RenderBundle>::new();

        Self {
            clear_color,
            renderables,
            texture: diffuse_texture,
            texture_bind_group,
            depth_texture,
            camera_uniform_buffer,
            camera_bind_group,
            pipeline,
        }
    }

    pub fn render(&self, manager: &device_manager::Manager, frame: &mut wgpu::SwapChainFrame) {
        // run the render pipeline
        let mut encoder =
            manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.output.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: true,
                        },
                    }
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: false,
                    }),
                    stencil_ops: None,
                }),
            });

            // actual render call
            //render_pass.set_pipeline(&self.pipeline);
            //render_pass.set_vertex_buffer(0, self.model.vertex_buffer_slice);
            //render_pass.set_index_buffer(self.model.index_buffer.slice(..));
            //render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            //render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            //render_pass.draw_indexed(0..self.model.num_elements, 0, 0..1);
            render_pass.execute_bundles(std::iter::once(&self.renderable.render_bundle));
        }
        let render_queue = encoder.finish();
        manager.queue.submit(std::iter::once(render_queue));
    }
}
