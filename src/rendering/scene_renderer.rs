use crate::rendering::texture::{Texture, Masks};
use crate::rendering::*;
use crate::device_manager;
use crate::computable_scene::compute_chain::ComputeChain;
use crate::computable_scene::compute_block::{ComputeBlock, RenderingData};
use wgpu::util::DeviceExt;
use glam::Mat4;

#[repr(C)]
#[derive(Copy, Clone)]
struct Uniforms {
    view: Mat4,
    proj: Mat4,
    mouse_pos: [i32; 2],
    _padding: [f32; 2],
}

const SAMPLE_COUNT: u32 = 4;

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    fn new() -> Self {
        Self {
            view: Mat4::identity(),
            proj: Mat4::identity(),
            mouse_pos: [0, 0],
            _padding: [0.0, 0.0],
        }
    }
}

/// The SceneRenderer is the structure responsible for rendeding the results
/// provided by a ComputeChain.
///
/// This is meant to be as much self-sufficient as possible. It holds its own
/// multisampled depth buffer and target buffer, its own rendering pipeline
/// and everything else that is needed for producing the image of the scene
/// that imgui picks up and shows to the user.
///
pub struct SceneRenderer {
    pipeline_solid: wgpu::RenderPipeline,
    picking_buffer_length: usize,
    picking_buffer: wgpu::Buffer,
    picking_bind_group: wgpu::BindGroup,
    renderables: Vec<wgpu::RenderBundle>,
    uniforms: Uniforms,
    uniforms_buffer: wgpu::Buffer,
    uniforms_bind_group: wgpu::BindGroup,
    depth_texture: Texture,
    output_texture: Texture,
}

impl SceneRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        // the object picking buffer is initially created with a reasonable default length
        // If the user displays more than this many objects, the buffer will get resized.
        let picking_buffer_length = 16;
        let (picking_buffer, picking_bind_layout, picking_bind_group) = create_picking_buffer(device, picking_buffer_length);

        // set up uniforms
        let uniforms = Uniforms::new();
        let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });
        let uniforms_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None,
                    },
                },
            ],
            label: Some("uniforms bind group layout"),
        });
        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &uniforms_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(uniforms_buffer.slice(..)),
                },
            ],
            label: Some("Uniforms bind group"),
        });

        // set up pipeline and render targets
        let pipeline_solid = create_solid_pipeline(&device, &uniforms_bind_layout, &picking_bind_layout);
        let depth_texture = Texture::create_depth_texture(&device, wgpu::Extent3d::default(), SAMPLE_COUNT);
        let output_texture = Texture::create_output_texture(&device, wgpu::Extent3d::default(), SAMPLE_COUNT);

        Self {
            picking_buffer_length,
            picking_buffer,
            picking_bind_group,
            renderables: Vec::new(),
            depth_texture,
            output_texture,
            uniforms,
            uniforms_buffer,
            uniforms_bind_group,
            pipeline_solid,
        }
    }

    pub fn update_depth_buffer_size(&mut self, device: &wgpu::Device, size: wgpu::Extent3d) {
        self.output_texture = Texture::create_output_texture(device, size, SAMPLE_COUNT);
        self.depth_texture = Texture::create_depth_texture(device, size, SAMPLE_COUNT);
    }

    pub fn update_renderables(&mut self, device: &wgpu::Device, avail_masks: &Masks, avail_textures: &Vec<Texture>, chain: &ComputeChain) {
        self.renderables.clear();
        // go through all blocks,
        // chose the "Rendering" ones,
        // turn their data into a renderable
        let rendering_data: Vec<&RenderingData> = chain.valid_blocks()
            .filter_map(|block| {
                if let ComputeBlock::Rendering(data) = block {
                    Some(data)
                } else {
                    None
                }
            })
        .collect();

        // if the buffer used for object picking is not big enough, resize it (i.e create a new one)
        if rendering_data.len() > self.picking_buffer_length {
            let (picking_buffer, _picking_bind_layout, picking_bind_group) = create_picking_buffer(device, rendering_data.len());
            self.picking_buffer_length = rendering_data.len();
            self.picking_buffer = picking_buffer;
            self.picking_bind_group = picking_bind_group;
        }

        for (id, data) in rendering_data.iter().enumerate() {
            self.add_renderable(device, avail_masks, avail_textures, data, id as u32);
        }
    }

    fn add_renderable(&mut self, device: &wgpu::Device, masks: &Masks, textures: &Vec<Texture>, rendering_data: &RenderingData, object_id: u32) {
        let mut render_bundle_encoder = device.create_render_bundle_encoder(
            &wgpu::RenderBundleEncoderDescriptor{
                label: Some("Render bundle encoder for RenderingData"),
                color_formats: &[SWAPCHAIN_FORMAT],
                depth_stencil_format: Some(DEPTH_FORMAT),
                sample_count: SAMPLE_COUNT,
            }
        );
        render_bundle_encoder.set_pipeline(&self.pipeline_solid);
        render_bundle_encoder.set_vertex_buffer(0, rendering_data.vertex_buffer.slice(..));
        render_bundle_encoder.set_index_buffer(rendering_data.index_buffer.slice(..));
        render_bundle_encoder.set_bind_group(0, &self.uniforms_bind_group, &[]);
        render_bundle_encoder.set_bind_group(1, &self.picking_bind_group, &[]);
        render_bundle_encoder.set_bind_group(2, &masks[rendering_data.mask_id].bind_group, &[]);
        render_bundle_encoder.set_bind_group(3, &textures[object_id as usize].bind_group, &[]);
        //render_bundle_encoder.set_bind_group(3, &textures[rendering_data.texture_id].bind_group, &[]);
        // encode the object_id in the instance used for indexed rendering, so that the shader
        // will be able to recover the id by reading the gl_InstanceIndex variable
        let instance_id = object_id;
        render_bundle_encoder.draw_indexed(0..rendering_data.index_count, 0, instance_id..instance_id+1);
        let render_bundle = render_bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
            label: Some("Render bundle for a single scene object"),
        });
        self.renderables.push(render_bundle);
    }

    pub fn update_view(&mut self, view: Mat4) {
        self.uniforms.view = view;
    }

    pub fn update_proj(&mut self, proj: Mat4) {
        self.uniforms.proj = proj;
    }

    pub fn update_mouse_pos(&mut self, mouse_pos: &[f32; 2]) {
        self.uniforms.mouse_pos[0] = mouse_pos[0] as i32;
        self.uniforms.mouse_pos[1] = mouse_pos[1] as i32;
    }

    pub fn render(&self, manager: &device_manager::Manager, target_view: &wgpu::TextureView) {
        let clear_color = wgpu::Color::BLACK;
        // update the uniforms buffer
        manager.queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));

        let initialize_picking = vec![std::f32::NAN; self.picking_buffer_length];
        manager.queue.write_buffer(&self.picking_buffer, 0, bytemuck::cast_slice(&initialize_picking));

        // run the render pipeline
        let mut encoder =
            manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &self.output_texture.view,
                        resolve_target: Some(target_view),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(clear_color),
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
            render_pass.execute_bundles(self.renderables.iter());
        }
        let render_queue = encoder.finish();
        manager.queue.submit(std::iter::once(render_queue));

        // after rendering: recover the contents of the picking vector
        if !self.renderables.is_empty() {
            use crate::util::copy_buffer_as_f32;
            let picking_distances = copy_buffer_as_f32(&self.picking_buffer, &manager.device);
            //dbg!(picking_distances);
        }
    }
}

fn create_picking_buffer(device: &wgpu::Device, length: usize) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let picking_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            label: None,
            size: (length * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::STORAGE,
        });
        let picking_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::StorageBuffer {
                        readonly: false,
                        dynamic: false,
                        min_binding_size: None,
                    },
                },
            ],
            label: Some("object picking bind group layout"),
        });
        // no need to store it, we will need to rebuild it on next scene creation anyway
        let picking_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &picking_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(picking_buffer.slice(..)),
                },
            ],
            label: Some("Camera bind group"),
        });

        (picking_buffer, picking_bind_layout, picking_bind_group)
}

fn create_solid_pipeline(device: &wgpu::Device, uniforms_bind_layout: &wgpu::BindGroupLayout, picking_bind_layout: &wgpu::BindGroupLayout) -> wgpu::RenderPipeline {
    // shader compiling
    let mut shader_compiler = shaderc::Compiler::new().unwrap();
    let vert_src = include_str!("matcap.vert");
    let frag_src = include_str!("matcap.frag");
    let vert_spirv = shader_compiler.compile_into_spirv(vert_src, shaderc::ShaderKind::Vertex, "matcap.vert", "main", None).unwrap();
    let frag_spirv = shader_compiler.compile_into_spirv(frag_src, shaderc::ShaderKind::Fragment, "matcap.frag", "main", None).unwrap();
    let vert_data = wgpu::util::make_spirv(vert_spirv.as_binary_u8());
    let frag_data = wgpu::util::make_spirv(frag_spirv.as_binary_u8());
    let vert_module = device.create_shader_module(vert_data);
    let frag_module = device.create_shader_module(frag_data);

    let texture_bind_layout = Texture::default_bind_layout(device);
    let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            push_constant_ranges: &[],
            bind_group_layouts: &[uniforms_bind_layout, picking_bind_layout, &texture_bind_layout, &texture_bind_layout]
        });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
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
            format: SWAPCHAIN_FORMAT,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL
        }],
        depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilStateDescriptor {
                front: wgpu::StencilStateFaceDescriptor::IGNORE,
                back: wgpu::StencilStateFaceDescriptor::IGNORE,
                read_mask: 0,
                write_mask: 0,
            }
        }),
        sample_count: SAMPLE_COUNT,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: &[
                wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<StandardVertexData>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float4, 1 => Float4, 2 => Float2, 3 => Float2],
                },
            ],
        },
    })
}
