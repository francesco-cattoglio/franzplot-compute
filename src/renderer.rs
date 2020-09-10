use crate::camera::*;
use crate::model;
use crate::model::*;
use crate::texture;
use crate::device_manager;

use wgpu::util::DeviceExt;

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

#[repr(C)]
#[derive(Copy, Clone)]
struct ModelMatrix {
    model_matrix: ultraviolet::Mat4
}
unsafe impl bytemuck::Pod for ModelMatrix {}
unsafe impl bytemuck::Zeroable for ModelMatrix {}



pub struct Renderer {
    pub a: i32,
    pipeline: wgpu::RenderPipeline,
    model: model::Model,
    camera: Camera,
    uniforms: Uniforms,
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture: texture::Texture,
    clear_color: wgpu::Color,
}

impl Renderer {
    pub fn new(manager: &device_manager::Manager, compute_buffer_slice: wgpu::BufferSlice) -> Self {
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
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer (compute_buffer_slice),
                },
            ],
            label: Some("Camera bind group"),
        });
        let (model, cmds) = model::Model::load(
            &manager.device,
            &texture_bind_group_layout,
            "/home/franz/rust/franzplot-compute/resources/flat.obj",
        )
        .unwrap();
        manager.queue.submit(cmds);

        let render_pipeline_layout =
            manager.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                push_constant_ranges: &[],
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_layout]
            });

        // shader compiling
        let mut shader_compiler = shaderc::Compiler::new().unwrap();
        let vert_src = include_str!("shader.vert");
        let frag_src = include_str!("shader.frag");
        let vert_spirv = shader_compiler.compile_into_spirv(vert_src, shaderc::ShaderKind::Vertex, "shader.vert", "main", None).unwrap();
        let frag_spirv = shader_compiler.compile_into_spirv(frag_src, shaderc::ShaderKind::Fragment, "shader.frag", "main", None).unwrap();
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
                vertex_buffers: &[model::ModelVertex::description()],
            },
        });

        let depth_texture = texture::Texture::create_depth_texture(&manager.device, &manager.sc_desc, "depth_texture");

        let clear_color = wgpu::Color::BLACK;

        Renderer {
            a: 0,
            clear_color,
            uniforms,
            model,
            depth_texture,
            camera,
            camera_uniform_buffer,
            camera_bind_group,
            pipeline,
        }
    }

    pub fn render(&self, frame: &mut wgpu::SwapChainFrame, manager: &device_manager::Manager) -> () {
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

            render_pass.set_pipeline(&self.pipeline);
            let mesh = &self.model.meshes[0];
            let material = &self.model.materials[mesh.material];
            // actual render call
            render_pass.draw_instanced(
                mesh, material,
                0..1,
                &self.camera_bind_group
            );
        }
        let render_queue = encoder.finish();
        manager.queue.submit(std::iter::once(render_queue));


    }
}
