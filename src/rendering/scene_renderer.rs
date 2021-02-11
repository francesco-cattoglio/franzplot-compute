use crate::state::Assets;
use crate::rendering::texture::{Texture};
use crate::rendering::*;
use crate::device_manager;
use crate::computable_scene::compute_chain::ComputeChain;
use crate::computable_scene::compute_block::{BlockId, ComputeBlock, Dimensions, RenderingData, VectorRenderingData};
use wgpu::util::DeviceExt;
use glam::Mat4;

#[repr(C)]
#[derive(Copy, Clone)]
struct Uniforms {
    view: Mat4,
    proj: Mat4,
    mouse_pos: [i32; 2],
    highlight_idx: i32,
    _padding: f32,
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
            highlight_idx: std::i32::MAX,
            _padding: 0.0,
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
    solid_pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: wgpu::RenderPipeline,
    picking_buffer_length: usize,
    picking_buffer: wgpu::Buffer,
    picking_bind_group: wgpu::BindGroup,
    wireframes: Vec<wgpu::RenderBundle>,
    renderables: Vec<wgpu::RenderBundle>,
    renderable_ids: Vec<BlockId>,
    uniforms: Uniforms,
    uniforms_buffer: wgpu::Buffer,
    uniforms_bind_group: wgpu::BindGroup,
    depth_texture: Texture,
    output_texture: Texture,
}

impl SceneRenderer {
    pub fn new_with_axes(device: &wgpu::Device) -> Self {
        let mut renderer = Self::new(device);
        renderer.add_wireframe_axes(device);
        renderer
    }

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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
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
                    resource: uniforms_buffer.as_entire_binding(),
                },
            ],
            label: Some("Uniforms bind group"),
        });

        // set up pipeline and render targets
        let solid_pipeline = create_solid_pipeline(&device, &uniforms_bind_layout, &picking_bind_layout);
        let wireframe_pipeline = create_wireframe_pipeline(&device, &uniforms_bind_layout);
        let depth_texture = Texture::create_depth_texture(&device, wgpu::Extent3d::default(), SAMPLE_COUNT);
        let output_texture = Texture::create_output_texture(&device, wgpu::Extent3d::default(), SAMPLE_COUNT);

        Self {
            picking_buffer_length,
            picking_buffer,
            picking_bind_group,
            wireframes: Vec::new(),
            renderables: Vec::new(),
            renderable_ids: Vec::new(),
            depth_texture,
            output_texture,
            uniforms,
            uniforms_buffer,
            uniforms_bind_group,
            solid_pipeline,
            wireframe_pipeline,
        }
    }

    pub fn highlight_object(&mut self, object: Option<BlockId>) {
        if let Some(id) = object {
            if let Some(idx) = self.renderable_ids.iter().position(|elem| *elem == id) {
                self.uniforms.highlight_idx = idx as i32;
            }
        } else {
            self.uniforms.highlight_idx = std::i32::MAX;
        }
    }

    pub fn update_depth_buffer_size(&mut self, device: &wgpu::Device, size: wgpu::Extent3d) {
        self.output_texture = Texture::create_output_texture(device, size, SAMPLE_COUNT);
        self.depth_texture = Texture::create_depth_texture(device, size, SAMPLE_COUNT);
    }

    pub fn update_renderables(&mut self, device: &wgpu::Device, assets: &Assets, chain: &ComputeChain) {
        self.renderables.clear();
        self.renderable_ids.clear();
        // go through all blocks,
        // chose the "Rendering" ones,
        // turn their data into a renderable
        let rendering_data: Vec<(&BlockId, &RenderingData)> = chain.valid_blocks()
            .filter_map(|(id, block)| {
                if let ComputeBlock::Rendering(data) = block {
                    Some((id, data))
                } else {
                    None
                }
            })
        .collect();

        let vector_rendering_data: Vec<(&BlockId, &VectorRenderingData)> = chain.valid_blocks()
            .filter_map(|(id, block)| {
                if let ComputeBlock::VectorRendering(data) = block {
                    Some((id, data))
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

        for (idx, (block_id, data)) in rendering_data.into_iter().enumerate() {
            self.renderable_ids.push(*block_id);
            self.add_renderable(device, assets, data, idx as u32);
        }
        for (block_id, data) in vector_rendering_data.into_iter() {
            self.renderable_ids.push(*block_id);
            self.add_renderable_vector(device, assets, data);
        }
    }

    fn add_wireframe_axes(&mut self, device: &wgpu::Device) {
        let mut render_bundle_encoder = device.create_render_bundle_encoder(
            &wgpu::RenderBundleEncoderDescriptor{
                label: Some("Render bundle encoder for RenderingData"),
                color_formats: &[SWAPCHAIN_FORMAT],
                depth_stencil_format: Some(DEPTH_FORMAT),
                sample_count: SAMPLE_COUNT,
            }
        );

        let vertices: Vec<WireframeVertexData> = vec![
            WireframeVertexData { position: [0.0, 0.0, 0.0], color: [255, 0, 0, 255] },
            WireframeVertexData { position: [1.0, 0.0, 0.0], color: [255, 0, 0, 255] },
            WireframeVertexData { position: [0.0, 0.0, 0.0], color: [0, 255, 0, 255] },
            WireframeVertexData { position: [0.0, 1.0, 0.0], color: [0, 255, 0, 255] },
            WireframeVertexData { position: [0.0, 0.0, 0.0], color: [0, 0, 255, 255] },
            WireframeVertexData { position: [0.0, 0.0, 1.0], color: [0, 0, 255, 255] },
        ];
        let indices: Vec<u32> = vec![
            0, 1,
            2, 3,
            4, 5,
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsage::INDEX,
        });

        render_bundle_encoder.set_pipeline(&self.wireframe_pipeline);
        render_bundle_encoder.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_bundle_encoder.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_bundle_encoder.set_bind_group(0, &self.uniforms_bind_group, &[]);
        render_bundle_encoder.draw_indexed(0..indices.len() as u32, 0, 0..1);
        let render_bundle = render_bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
            label: Some("Render bundle for wireframe"),
        });
        self.wireframes.push(render_bundle);
    }

    fn add_renderable(&mut self, device: &wgpu::Device, assets: &Assets, rendering_data: &RenderingData, object_id: u32) {
        let mut render_bundle_encoder = device.create_render_bundle_encoder(
            &wgpu::RenderBundleEncoderDescriptor{
                label: Some("Render bundle encoder for RenderingData"),
                color_formats: &[SWAPCHAIN_FORMAT],
                depth_stencil_format: Some(DEPTH_FORMAT),
                sample_count: SAMPLE_COUNT,
            }
        );
        let index_buffer = match rendering_data.out_dim {
            Dimensions::D0 => rendering_data.index_buffer.as_ref().unwrap(),
            Dimensions::D1(_) => rendering_data.index_buffer.as_ref().unwrap(),
            Dimensions::D2(_, _) => rendering_data.index_buffer.as_ref().unwrap(),
            Dimensions::D3(_, prefab_id) => &assets.models[prefab_id as usize].index_buffer,
        };
        render_bundle_encoder.set_pipeline(&self.solid_pipeline);
        render_bundle_encoder.set_vertex_buffer(0, rendering_data.vertex_buffer.slice(..));
        render_bundle_encoder.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_bundle_encoder.set_bind_group(0, &self.uniforms_bind_group, &[]);
        render_bundle_encoder.set_bind_group(1, &self.picking_bind_group, &[]);
        render_bundle_encoder.set_bind_group(2, &assets.masks[rendering_data.mask_id].bind_group, &[]);
        render_bundle_encoder.set_bind_group(3, &assets.materials[rendering_data.material_id].bind_group, &[]);
        // encode the object_id in the instance used for indexed rendering, so that the shader
        // will be able to recover the id by reading the gl_InstanceIndex variable
        let instance_id = object_id;
        render_bundle_encoder.draw_indexed(0..rendering_data.index_count, 0, instance_id..instance_id+1);
        let render_bundle = render_bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
            label: Some("Render bundle for a single scene object"),
        });
        self.renderables.push(render_bundle);
    }

    fn add_renderable_vector(&mut self, device: &wgpu::Device, assets: &Assets, rendering_data: &VectorRenderingData) {
        let mut render_bundle_encoder = device.create_render_bundle_encoder(
            &wgpu::RenderBundleEncoderDescriptor{
                label: Some("Render bundle encoder for VectorRenderingData"),
                color_formats: &[SWAPCHAIN_FORMAT],
                depth_stencil_format: Some(DEPTH_FORMAT),
                sample_count: SAMPLE_COUNT,
            }
        );
        render_bundle_encoder.set_pipeline(&self.solid_pipeline);
        render_bundle_encoder.set_vertex_buffer(0, rendering_data.out_buffer.slice(..));
        render_bundle_encoder.set_index_buffer(rendering_data.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_bundle_encoder.set_bind_group(0, &self.uniforms_bind_group, &[]);
        render_bundle_encoder.set_bind_group(1, &self.picking_bind_group, &[]);
        render_bundle_encoder.set_bind_group(2, &assets.masks[0].bind_group, &[]);
        render_bundle_encoder.set_bind_group(3, &assets.materials[rendering_data.material_id].bind_group, &[]);
        // encode the object_id in the instance used for indexed rendering, so that the shader
        // will be able to recover the id by reading the gl_InstanceIndex variable
        let instance_id = 0; //TODO: this needs fixing, otherwise this breaks the object picking
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

    pub fn update_mouse_pos(&mut self, mouse_pos: &[i32; 2]) {
        self.uniforms.mouse_pos[0] = mouse_pos[0];
        self.uniforms.mouse_pos[1] = mouse_pos[1];
    }

    pub fn render(&self, manager: &device_manager::Manager, target_view: &wgpu::TextureView) {
        let clear_color = wgpu::Color::BLACK;
        // update the uniforms buffer
        manager.queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));

        let initialize_picking = vec![std::i32::MAX; self.picking_buffer_length];
        manager.queue.write_buffer(&self.picking_buffer, 0, bytemuck::cast_slice(&initialize_picking));

        // run the render pipeline
        let mut encoder =
            manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene render pass"),
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

            // actual render calls
            render_pass.execute_bundles(self.wireframes.iter());
            render_pass.execute_bundles(self.renderables.iter());
        }
        let render_queue = encoder.finish();
        manager.queue.submit(std::iter::once(render_queue));
    }

    pub fn object_under_cursor(&self, device: &wgpu::Device) -> Option<BlockId> {
        use crate::util::copy_buffer_as;
        let picking_distances = copy_buffer_as::<i32>(&self.picking_buffer, device);
        // extract the min value in the picking distances array and its index
        let (min_idx, min_value) = picking_distances
            .into_iter()
            .enumerate()
            .min_by_key(|(_idx, value)| *value)?;
        // if the value is different from the initialization value, we can use this
        // idx to recover the BlockId of the renderable that is closer to the camera
        if min_value != std::i32::MAX {
            Some(self.renderable_ids[min_idx])
        } else {
            None
        }
    }
}

fn create_picking_buffer(device: &wgpu::Device, length: usize) -> (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let picking_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            label: None,
            size: (length * std::mem::size_of::<i32>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::STORAGE,
        });
        let picking_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
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
                    resource: picking_buffer.as_entire_binding(),
                },
            ],
            label: Some("Camera bind group"),
        });

        (picking_buffer, picking_bind_layout, picking_bind_group)
}

fn create_wireframe_pipeline(device: &wgpu::Device, uniforms_bind_layout: &wgpu::BindGroupLayout) -> wgpu::RenderPipeline {
    // shader compiling
    let mut shader_compiler = shaderc::Compiler::new().unwrap();
    let vert_src = include_str!("wireframe.vert");
    let frag_src = include_str!("wireframe.frag");
    let vert_spirv = shader_compiler.compile_into_spirv(vert_src, shaderc::ShaderKind::Vertex, "wireframe.vert", "main", None).unwrap();
    let frag_spirv = shader_compiler.compile_into_spirv(frag_src, shaderc::ShaderKind::Fragment, "wireframe.frag", "main", None).unwrap();
    let vert_data = wgpu::util::make_spirv(vert_spirv.as_binary_u8());
    let frag_data = wgpu::util::make_spirv(frag_spirv.as_binary_u8());
    let vert_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor{
        label: Some("wireframe vertex shader module"),
        source: vert_data,
        flags: wgpu::ShaderFlags::empty(), // TODO: maybe use VALIDATION flags
    });
    let frag_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor{
        label: Some("wireframe fragment shader module"),
        source: frag_data,
        flags: wgpu::ShaderFlags::empty(), // TODO: maybe use VALIDATION flags
    });

    let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            push_constant_ranges: &[],
            bind_group_layouts: &[uniforms_bind_layout]
        });

    let vertex_buffer_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<WireframeVertexData>() as wgpu::BufferAddress,
        step_mode: wgpu::InputStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Uchar4Norm],
    };
    let color_target_state = wgpu::ColorTargetState {
        format: super::SWAPCHAIN_FORMAT,
        alpha_blend: wgpu::BlendState::REPLACE,
        color_blend: wgpu::BlendState::REPLACE,
        write_mask: wgpu::ColorWrite::ALL,
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
        layout: Some(&render_pipeline_layout),
        label: None,
        vertex: wgpu::VertexState {
            module: &vert_module,
            entry_point: "main",
            buffers: &[vertex_buffer_layout],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            polygon_mode: wgpu::PolygonMode::Fill,
        },
        fragment: Some(wgpu::FragmentState {
            module: &frag_module,
            entry_point: "main",
            targets: &[color_target_state],
        }),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: super::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Greater,
            bias: wgpu::DepthBiasState::default(),
            stencil: wgpu::StencilState::default(),
            clamp_depth: false,
        }),
        multisample: wgpu::MultisampleState {
            count: SAMPLE_COUNT,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
    })
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
    let vert_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor{
        label: Some("solid pipeline vertex shader module"),
        source: vert_data,
        flags: wgpu::ShaderFlags::empty(), // TODO: maybe use VALIDATION flags
    });
    let frag_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor{
        label: Some("solid pipeline fragment shader module"),
        source: frag_data,
        flags: wgpu::ShaderFlags::empty(), // TODO: maybe use VALIDATION flags
    });

    let texture_bind_layout = Texture::default_bind_layout(device);
    let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            push_constant_ranges: &[],
            bind_group_layouts: &[uniforms_bind_layout, picking_bind_layout, &texture_bind_layout, &texture_bind_layout]
        });

    let vertex_buffer_descriptor = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<StandardVertexData>() as wgpu::BufferAddress,
        step_mode: wgpu::InputStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float4, 1 => Float4, 2 => Float2, 3 => Float2],
    };
    let color_target_state = wgpu::ColorTargetState {
        format: super::SWAPCHAIN_FORMAT,
        alpha_blend: wgpu::BlendState::REPLACE,
        color_blend: wgpu::BlendState::REPLACE,
        write_mask: wgpu::ColorWrite::ALL,
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
        layout: Some(&render_pipeline_layout),
        label: None,
        vertex: wgpu::VertexState {
            module: &vert_module,
            entry_point: "main",
            buffers: &[vertex_buffer_descriptor],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            polygon_mode: wgpu::PolygonMode::Fill,
        },
        fragment: Some(wgpu::FragmentState {
            module: &frag_module,
            entry_point: "main",
            targets: &[color_target_state],
        }),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: super::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Greater,
            bias: wgpu::DepthBiasState::default(),
            stencil: wgpu::StencilState::default(),
            clamp_depth: false,
        }),
        multisample: wgpu::MultisampleState {
            count: SAMPLE_COUNT,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
    })
}
