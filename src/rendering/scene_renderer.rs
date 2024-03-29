use crate::node_graph::NodeID;
use crate::state::Assets;
use crate::rendering::texture::Texture;
use crate::rendering::*;
use crate::device_manager;
use crate::compute_graph::{MatcapData, MatcapIter};
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

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    fn new() -> Self {
        Self {
            view: Mat4::IDENTITY,
            proj: Mat4::IDENTITY,
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
    pipelines: Pipelines,
    picking_buffer_length: usize,
    picking_buffer: wgpu::Buffer,
    picking_bind_group: wgpu::BindGroup,
    billboards: Vec<wgpu::RenderBundle>,
    wireframe_axes: Option<wgpu::RenderBundle>,
    renderables: Vec<wgpu::RenderBundle>,
    renderable_ids: Vec<NodeID>,
    uniforms: Uniforms,
    uniforms_buffer: wgpu::Buffer,
    texture_extent: wgpu::Extent3d,
    depth_texture: Texture,
    output_texture: Texture,
}

struct Pipelines {
    matcap: wgpu::RenderPipeline,
    billboard: wgpu::RenderPipeline,
    wireframe: wgpu::RenderPipeline,
}

impl SceneRenderer {
    pub fn new_with_axes(manager: &device_manager::Manager) -> Self {
        let mut renderer = Self::new(manager);
        renderer.set_wireframe_axes(manager, 2, 0.075);
        renderer.set_axes_labels(manager, 2.0, 0.15);
        renderer
    }

    pub fn new(manager: &device_manager::Manager) -> Self {
        let device = &manager.device;
        // the object picking buffer is initially created with a reasonable default length
        // If the user displays more than this many objects, the buffer will get resized.
        let picking_buffer_length = 16;
        let (picking_buffer, picking_bind_layout, picking_bind_group) = create_picking_buffer(device, picking_buffer_length);

        // pipelines from the wgsl shader, layouts are auto-deduced
        let pipelines = create_pipelines(manager);

        // create the buffer that will be needed for the uniforms
        let uniforms = Uniforms::new();
        let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let texture_extent = wgpu::Extent3d::default();
        let depth_texture = Texture::create_depth_texture(device, texture_extent, manager.sample_count);
        let output_texture = Texture::create_output_texture(device, texture_extent, manager.sample_count);

        Self {
            picking_buffer_length,
            picking_buffer,
            picking_bind_group,
            wireframe_axes: None,
            billboards: Vec::new(),
            renderables: Vec::new(),
            renderable_ids: Vec::new(),
            texture_extent,
            depth_texture,
            output_texture,
            uniforms,
            uniforms_buffer,
            pipelines,
        }
    }

    pub fn highlight_object(&mut self, object: Option<NodeID>) {
        if let Some(id) = object {
            if let Some(idx) = self.renderable_ids.iter().position(|elem| *elem == id) {
                self.uniforms.highlight_idx = idx as i32;
            }
        } else {
            self.uniforms.highlight_idx = std::i32::MAX;
        }
    }

    pub fn resize_if_needed(&mut self, manager: &device_manager::Manager, new_size: wgpu::Extent3d) {
        if new_size.ne(&self.texture_extent) {
            let device = &manager.device;
            self.texture_extent = new_size;
            self.output_texture = Texture::create_output_texture(device, new_size, manager.sample_count);
            self.depth_texture = Texture::create_depth_texture(device, new_size, manager.sample_count);
        }
    }

    pub fn clear_wireframe_axes(&mut self) {
        self.wireframe_axes = None;
    }

    pub fn clear_axes_labels(&mut self) {
        self.billboards.clear();
    }

    pub fn set_axes_labels(&mut self, manager: &device_manager::Manager, distance_from_origin: f32, size: f32) {
        self.clear_axes_labels();
        // we create the letters using triangles with repeated vertices. They are composed just by a
        // bunch of triangles anyway.
        let pos_2d_x = vec![
            // first segment of the X
            [-0.43*size, -0.50*size], [-0.19*size, -0.50*size], [ 0.43*size,  0.50*size],
            [ 0.43*size,  0.50*size], [ 0.19*size,  0.50*size], [-0.43*size, -0.50*size],
            // second segment of the X
            [-0.43*size,  0.50*size], [ 0.19*size, -0.50*size], [ 0.43*size, -0.50*size],
            [ 0.43*size, -0.50*size], [-0.19*size,  0.50*size], [-0.43*size,  0.50*size],
        ];
        let pos_2d_y = vec![
            // quad for the leg
            [-0.12*size, -0.50*size], [ 0.12*size, -0.50*size], [ 0.12*size, -0.16*size],
            [ 0.12*size, -0.16*size], [-0.12*size, -0.16*size], [-0.12*size, -0.50*size],
            // quad for the left arm
            [-0.12*size, -0.16*size], [ 0.12*size, -0.16*size], [-0.19*size,  0.50*size],
            [-0.19*size,  0.50*size], [-0.43*size,  0.50*size], [-0.12*size, -0.16*size],
            // quad for the right arm
            [-0.12*size, -0.16*size], [ 0.12*size, -0.16*size], [ 0.43*size,  0.50*size],
            [ 0.43*size,  0.50*size], [ 0.19*size,  0.50*size], [-0.12*size, -0.16*size],
        ];
        let pos_2d_z = vec![
            // bottom line quad
            [-0.38*size, -0.50*size], [ 0.38*size, -0.50*size], [ 0.38*size, -0.30*size],
            [ 0.38*size, -0.30*size], [-0.38*size, -0.30*size], [-0.38*size, -0.50*size],
            // top line quad
            [-0.38*size,  0.30*size], [ 0.38*size,  0.30*size], [ 0.38*size,  0.50*size],
            [ 0.38*size,  0.50*size], [-0.38*size,  0.50*size], [-0.38*size,  0.30*size],
             // oblique line
            [-0.38*size, -0.30*size], [-0.08*size, -0.30*size], [ 0.38*size,  0.30*size],
            [ 0.38*size,  0.30*size], [ 0.08*size,  0.30*size], [-0.38*size, -0.30*size],
        ];

        let z_offset = 1.0 * size;
        let color_x: [u8; 4] = [255, 0, 0, 255];
        let color_y: [u8; 4] = [0, 255, 0, 255];
        let color_z: [u8; 4] = [0, 0, 255, 255];

        // turn all the 2d coordinates into a single, buffer of BillboardVertexData structs
        let vertices: Vec::<BillboardVertexData> = pos_2d_x.into_iter()
            .map(|p_2d| BillboardVertexData{
                position: p_2d,
                offset: [distance_from_origin, 0.0, z_offset],
                color: color_x,
            })
            .chain(
                pos_2d_y.into_iter()
                .map(|p_2d| BillboardVertexData{
                    position: p_2d,
                    offset: [0.0, distance_from_origin, z_offset],
                    color: color_y,
                })
            )
            .chain(
                pos_2d_z.into_iter()
                .map(|p_2d| BillboardVertexData{
                    position: p_2d,
                    offset: [0.0, 0.0, distance_from_origin + z_offset],
                    color: color_z,
                })
            )
            .collect();
        let vertex_buffer = manager.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        self.add_billboard(manager, &vertex_buffer, vertices.len() as u32);
    }

    fn add_billboard(&mut self, manager: &device_manager::Manager, vertex_buffer: &wgpu::Buffer, vertex_count: u32) {
        let device = &manager.device;
        let mut render_bundle_encoder = device.create_render_bundle_encoder(
            &wgpu::RenderBundleEncoderDescriptor{
                label: Some("Render bundle encoder for billboard"),
                color_formats: &[SCENE_FORMAT],
                depth_stencil: Some(wgpu::RenderBundleDepthStencil{
                    format: DEPTH_FORMAT,
                    depth_read_only: false,
                    stencil_read_only: false,
                }),
                multiview: None,
                sample_count: manager.sample_count,
            }
        );
        // In order to create a correct uniforms bind group, we need to recover the layour from the correct pipeline
        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &self.pipelines.billboard.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniforms_buffer.as_entire_binding(),
                },
            ],
            label: Some("Uniforms bind group"),
        });
        render_bundle_encoder.set_pipeline(&self.pipelines.billboard);
        render_bundle_encoder.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_bundle_encoder.set_bind_group(0, &uniforms_bind_group, &[]);
        render_bundle_encoder.draw(0..vertex_count, 0..1);
        let render_bundle = render_bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
            label: Some("Render bundle for a billboard"),
        });
        self.billboards.push(render_bundle);
    }

    pub fn set_wireframe_axes(&mut self, manager: &device_manager::Manager, length: i32, cross_size: f32) {
        // for each of the three axis
        // create a line plus as many small cross as needed, to help visualize unit lengths
        let mut vertices = Vec::new();

        let colo_h = 255u8;
        let colo_l = 64u8;
        for i in 1..=length {
            let i = i as f32;
            vertices.append(&mut create_wireframe_cross(glam::Vec3::new(  i, 0.0, 0.0), cross_size, [colo_h, 0, 0, 255]));
            vertices.append(&mut create_wireframe_cross(glam::Vec3::new( -i, 0.0, 0.0), cross_size, [colo_l, 0, 0, 255]));
            vertices.append(&mut create_wireframe_cross(glam::Vec3::new(0.0,   i, 0.0), cross_size, [0, colo_h, 0, 255]));
            vertices.append(&mut create_wireframe_cross(glam::Vec3::new(0.0,  -i, 0.0), cross_size, [0, colo_l, 0, 255]));
            vertices.append(&mut create_wireframe_cross(glam::Vec3::new(0.0, 0.0,   i), cross_size, [0, 0, colo_h, 255]));
            vertices.append(&mut create_wireframe_cross(glam::Vec3::new(0.0, 0.0,  -i), cross_size, [0, 0, colo_l, 255]));
        }
        // add the actual lines
        vertices.append(&mut vec![
            WireframeVertexData { position: [ length as f32, 0.0, 0.0], color: [colo_h, 0, 0, 255] },
            WireframeVertexData { position: [           0.0, 0.0, 0.0], color: [colo_h, 0, 0, 255] },
            WireframeVertexData { position: [-length as f32, 0.0, 0.0], color: [colo_l, 0, 0, 255] },
            WireframeVertexData { position: [           0.0, 0.0, 0.0], color: [colo_l, 0, 0, 255] },
            WireframeVertexData { position: [0.0,  length as f32, 0.0], color: [0, colo_h, 0, 255] },
            WireframeVertexData { position: [0.0,            0.0, 0.0], color: [0, colo_h, 0, 255] },
            WireframeVertexData { position: [0.0, -length as f32, 0.0], color: [0, colo_l, 0, 255] },
            WireframeVertexData { position: [0.0,            0.0, 0.0], color: [0, colo_l, 0, 255] },
            WireframeVertexData { position: [0.0, 0.0,  length as f32], color: [0, 0, colo_h, 255] },
            WireframeVertexData { position: [0.0, 0.0,            0.0], color: [0, 0, colo_h, 255] },
            WireframeVertexData { position: [0.0, 0.0, -length as f32], color: [0, 0, colo_l, 255] },
            WireframeVertexData { position: [0.0, 0.0,            0.0], color: [0, 0, colo_l, 255] },
        ]);

        let vertex_buffer = manager.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        self.add_wireframe(manager, &vertex_buffer, vertices.len() as u32);
    }

    fn add_wireframe(&mut self, manager: &device_manager::Manager, vertex_buffer: &wgpu::Buffer, vertex_count: u32) {
        let device = &manager.device;
        let mut render_bundle_encoder = device.create_render_bundle_encoder(
            &wgpu::RenderBundleEncoderDescriptor{
                label: Some("Render bundle encoder for wireframe"),
                color_formats: &[SCENE_FORMAT],
                depth_stencil: Some(wgpu::RenderBundleDepthStencil{
                    format: DEPTH_FORMAT,
                    depth_read_only: false,
                    stencil_read_only: false,
                }),
                multiview: None,
                sample_count: manager.sample_count,
            }
        );
        // In order to create a correct uniforms bind group, we need to recover the layour from the correct pipeline
        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &self.pipelines.wireframe.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniforms_buffer.as_entire_binding(),
                },
            ],
            label: Some("Uniforms bind group"),
        });
        render_bundle_encoder.set_pipeline(&self.pipelines.wireframe);
        render_bundle_encoder.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_bundle_encoder.set_bind_group(0, &uniforms_bind_group, &[]);
        render_bundle_encoder.draw(0..vertex_count, 0..1);
        let render_bundle = render_bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
            label: Some("Render bundle for a wireframe"),
        });
        self.wireframe_axes = Some(render_bundle);
    }

    pub fn clear_matcaps(&mut self) {
        self.renderables.clear();
        self.renderable_ids.clear();
    }

    pub fn recreate_matcaps(&mut self, manager: &device_manager::Manager, assets: &Assets, matcaps: MatcapIter<'_>) {
        self.clear_matcaps();
        // go through all blocks,
        // chose the "Rendering" ones,
        // turn their data into a renderable

        // if the buffer used for object picking is not big enough, resize it (i.e create a new one)
        if matcaps.len() > self.picking_buffer_length {
            let (picking_buffer, _picking_bind_layout, picking_bind_group) = create_picking_buffer(&manager.device, matcaps.len());
            self.picking_buffer_length = matcaps.len();
            self.picking_buffer = picking_buffer;
            self.picking_bind_group = picking_bind_group;
        }

        for (idx, (data_id, matcap)) in matcaps.enumerate() {
            self.renderable_ids.push(*data_id);
            self.add_matcap(manager, assets, matcap, idx as u32);
        }
    }

    fn add_matcap(&mut self, manager: &device_manager::Manager, assets: &Assets, matcap_data: &MatcapData, object_id: u32) {
        let device = &manager.device;
        let mut render_bundle_encoder = device.create_render_bundle_encoder(
            &wgpu::RenderBundleEncoderDescriptor{
                label: Some("Render bundle encoder for MatcapData"),
                color_formats: &[SCENE_FORMAT],
                depth_stencil: Some(wgpu::RenderBundleDepthStencil{
                    format: DEPTH_FORMAT,
                    depth_read_only: false,
                    stencil_read_only: false,
                }),
                multiview: None,
                sample_count: manager.sample_count,
            }
        );
        // In order to create a correct uniforms bind group, we need to recover the layour from the correct pipeline
        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &self.pipelines.matcap.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniforms_buffer.as_entire_binding(),
                },
            ],
            label: Some("Uniforms bind group"),
        });
        render_bundle_encoder.set_pipeline(&self.pipelines.matcap);
        render_bundle_encoder.set_vertex_buffer(0, matcap_data.vertex_buffer.slice(..));
        render_bundle_encoder.set_index_buffer(matcap_data.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_bundle_encoder.set_bind_group(0, &uniforms_bind_group, &[]);
        render_bundle_encoder.set_bind_group(1, &self.picking_bind_group, &[]);
        render_bundle_encoder.set_bind_group(2, &assets.masks[matcap_data.mask_id].bind_group, &[]);
        render_bundle_encoder.set_bind_group(3, &assets.materials[matcap_data.material_id].bind_group, &[]);
        // encode the object_id in the instance used for indexed rendering, so that the shader
        // will be able to recover the id by reading the gl_InstanceIndex variable
        let instance_id = object_id;
        //render_bundle_encoder.draw_indexed(0..rendering_data.index_count, 0, instance_id..instance_id+1);
        render_bundle_encoder.draw_indexed(0..matcap_data.index_count, 0, 0..1);
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
            let (view, resolve_target);
            if manager.sample_count > 1 {
                view = &self.output_texture.view;
                resolve_target = Some(target_view);
            } else {
                view = target_view;
                resolve_target = None;
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene render pass"),
                color_attachments: &[
                    wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(clear_color),
                            store: true,
                        },
                    }
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: false,
                    }),
                    stencil_ops: None,
                }),
            });

            // actual render calls
            render_pass.execute_bundles(self.wireframe_axes.iter());
            render_pass.execute_bundles(self.billboards.iter());
            render_pass.execute_bundles(self.renderables.iter());
        }
        let render_queue = encoder.finish();
        manager.queue.submit(std::iter::once(render_queue));
    }

    pub fn object_under_cursor(&self, device: &wgpu::Device) -> Option<NodeID> {
        use crate::util::copy_buffer_as;
        let picking_distances = copy_buffer_as::<i32>(&self.picking_buffer, device);
        // extract the min value in the picking distances array and its index
        let (min_idx, min_value) = picking_distances
            .into_iter()
            .enumerate()
            .min_by_key(|(_idx, value)| *value)?;
        // if the value is different from the initialization value, we can use this
        // idx to recover the NodeID of the renderable that is closer to the camera
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
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::MAP_READ,
        });
        let picking_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    visibility: wgpu::ShaderStages::FRAGMENT,
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

fn create_pipelines(manager: &device_manager::Manager) -> Pipelines {
    // read/import the shader source code and create a module from it
    let wgsl_source = include_str!("matcap.wgsl");
    let wgsl_module = manager.device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: Some("matcap shader module"),
        source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
    });

    // define some state variables that will be used many times when creating the actual pipelines
    let color_target_state = wgpu::ColorTargetState {
        format: super::SCENE_FORMAT,
        blend: Some(wgpu::BlendState {
            color: wgpu::BlendComponent::REPLACE,
            alpha: wgpu::BlendComponent::REPLACE,
        }),
        write_mask: wgpu::ColorWrites::ALL,
    };
    let depth_stencil_state = Some(wgpu::DepthStencilState {
        format: super::DEPTH_FORMAT,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        bias: wgpu::DepthBiasState::default(),
        stencil: wgpu::StencilState::default(),
    });
    let multisample_state = wgpu::MultisampleState {
        count: manager.sample_count,
        mask: !0,
        alpha_to_coverage_enabled: false,
    };

    // in particular, there are two primitive kinds: the triangles for the billboard and matcap
    // objects or the lines only for the wireframe effect
    let primitive_triangles = wgpu::PrimitiveState {
        unclipped_depth: false,
        conservative: false,
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        polygon_mode: wgpu::PolygonMode::Fill,
    };
    let primitive_lines = wgpu::PrimitiveState {
        unclipped_depth: false,
        conservative: false,
        topology: wgpu::PrimitiveTopology::LineList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        polygon_mode: wgpu::PolygonMode::Fill,
    };

    let device = &manager.device;

    let matcap = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        vertex: wgpu::VertexState {
            module: &wgsl_module,
            entry_point: "matcap_vs_main",
            buffers: &[StandardVertexData::vertex_buffer_layout()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &wgsl_module,
            entry_point: "matcap_fs_main",
            targets: &[color_target_state.clone()],
        }),
        layout: None,
        label: None,
        primitive: primitive_triangles,
        depth_stencil: depth_stencil_state.clone(),
        multisample: multisample_state,
        multiview: None,
    });

    let billboard = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        vertex: wgpu::VertexState {
            module: &wgsl_module,
            entry_point: "billboard_vs_main",
            buffers: &[BillboardVertexData::vertex_buffer_layout()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &wgsl_module,
            entry_point: "color_fs_main",
            targets: &[color_target_state.clone()],
        }),
        layout: None,
        label: None,
        primitive: primitive_triangles,
        depth_stencil: depth_stencil_state.clone(),
        multisample: multisample_state,
        multiview: None,
    });

    let wireframe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        vertex: wgpu::VertexState {
            module: &wgsl_module,
            entry_point: "wireframe_vs_main",
            buffers: &[WireframeVertexData::vertex_buffer_layout()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &wgsl_module,
            entry_point: "color_fs_main",
            targets: &[color_target_state],
        }),
        layout: None,
        label: None,
        primitive: primitive_lines,
        depth_stencil: depth_stencil_state,
        multisample: multisample_state,
        multiview: None,
    });
    Pipelines {
        matcap,
        billboard,
        wireframe,
    }
}

fn create_wireframe_cross(pos: glam::Vec3, size: f32, color: [u8; 4]) -> Vec<WireframeVertexData> {
    vec![
        WireframeVertexData { position: (pos - size*glam::Vec3::X).into(), color },
        WireframeVertexData { position: (pos + size*glam::Vec3::X).into(), color },
        WireframeVertexData { position: (pos - size*glam::Vec3::Y).into(), color },
        WireframeVertexData { position: (pos + size*glam::Vec3::Y).into(), color },
        WireframeVertexData { position: (pos - size*glam::Vec3::Z).into(), color },
        WireframeVertexData { position: (pos + size*glam::Vec3::Z).into(), color },
    ]
}

