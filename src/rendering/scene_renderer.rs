use crate::rendering::camera::Camera;
use crate::rendering::texture;
use crate::rendering::*;
use crate::device_manager;
use crate::computable_scene::compute_chain::ComputeChain;
use crate::computable_scene::compute_block::{ComputeBlock, RenderingData};
use wgpu::util::DeviceExt;
use glam::Mat4;

#[repr(C)]
#[derive(Copy, Clone)]
struct Uniforms {
    view_proj: Mat4,
}

unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

impl Uniforms {
    fn new() -> Self {
        Self {
            view_proj: Mat4::identity(),
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

pub struct SceneRenderer {
    pipeline_solid: wgpu::RenderPipeline,
    picking_buffer: wgpu::Buffer,
    renderables: Vec<wgpu::RenderBundle>,
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture: texture::Texture,
    clear_color: wgpu::Color,
}

impl SceneRenderer {
    // TODO: when we manage to get the diffuse texture in its own class,
    // change this to only take a &wgpu::Device as input
    pub fn new(manager: &device_manager::Manager) -> Self {
        let camera = Camera::from_height_width(manager.sc_desc.width as f32, manager.sc_desc.height as f32);

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);

        // the object picking buffer is null because the scene is empty
        let picking_buffer = manager.device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            label: None,
            size: 0,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::STORAGE,
        });
        let picking_bind_layout =
            manager.device.create_bind_group_layout(&PICKING_LAYOUT_DESCRIPTOR);

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
                    resource: wgpu::BindingResource::Buffer(camera_uniform_buffer.slice(..)),
                },
            ],
            label: Some("Camera bind group"),
        });
        let path = std::path::Path::new("./resources/grid_color.png");
        //let diffuse_texture = texture::Texture::load(&manager.device, &manager.queue, path, "cube-diffuse").context("failed to load texture").unwrap();

        //let texture_bind_layout =
        //    manager.device.create_bind_group_layout(&TEXTURE_LAYOUT_DESCRIPTOR);
        //let texture_bind_group = manager.device.create_bind_group(&wgpu::BindGroupDescriptor {
        //    layout: &texture_bind_layout,
        //    entries: &[
        //        wgpu::BindGroupEntry {
        //            binding: 0,
        //            resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
        //        },
        //        wgpu::BindGroupEntry {
        //            binding: 1,
        //            resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
        //        },
        //    ],
        //    label: Some("all_materials")
        //});

        let pipeline_solid = create_solid_pipeline(&manager.device);
        let output_texture = texture::Texture::create_output_texture(&manager.device, wgpu::Extent3d::default());
        let depth_texture = texture::Texture::create_depth_texture(&manager.device, wgpu::Extent3d::default());

        let clear_color = wgpu::Color::BLACK;

        let renderables = Vec::<wgpu::RenderBundle>::new();

        Self {
            picking_buffer,
            clear_color,
            renderables,
            depth_texture,
            camera_uniform_buffer,
            camera_bind_group,
            pipeline_solid,
        }
    }

    pub fn update_depth_buffer_size(&mut self, device: &wgpu::Device, size: wgpu::Extent3d) {
        self.depth_texture = texture::Texture::create_depth_texture(device, size);
    }

    pub fn update_renderables(&mut self, device: &wgpu::Device, chain: &ComputeChain,) {
        self.renderables.clear();
        // go through all blocks, chose the rendering ones,
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
        // resize the buffer that will be used for object picking
        // the object picking buffer is null because the scene is empty
        self.picking_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            label: None,
            size: (rendering_data.len() * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::STORAGE,
        });
        let picking_bind_layout = device.create_bind_group_layout(&PICKING_LAYOUT_DESCRIPTOR);
        // no need to store it, we will need to rebuild it on next scene creation anyway
        let picking_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            layout: &picking_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(self.picking_buffer.slice(..)),
                },
            ],
            label: Some("Camera bind group"),
        });

        for (id, data) in rendering_data.iter().enumerate() {
            self.add_renderable(device, data, &picking_bind_group, id as u32);
        }
    }

    fn add_renderable(&mut self, device: &wgpu::Device, rendering_data: &RenderingData, picking_bind_group: &wgpu::BindGroup, object_id: u32) {
        let mut render_bundle_encoder = device.create_render_bundle_encoder(
            &wgpu::RenderBundleEncoderDescriptor{
                label: Some("Render bundle encoder for RenderingData"),
                color_formats: &[SWAPCHAIN_FORMAT],
                depth_stencil_format: Some(DEPTH_FORMAT),
                sample_count: 1,
            }
        );
        render_bundle_encoder.set_pipeline(&self.pipeline_solid);
        render_bundle_encoder.set_vertex_buffer(0, rendering_data.vertex_buffer.slice(..));
        render_bundle_encoder.set_index_buffer(rendering_data.index_buffer.slice(..));
        render_bundle_encoder.set_bind_group(0, &self.camera_bind_group, &[]);
        render_bundle_encoder.set_bind_group(1, picking_bind_group, &[]);
//        render_bundle_encoder.set_bind_group(2, &self.texture_bind_group, &[]);
        // encode the object_id in the index used for the rendering. The shader will be able to
        // recover the id by reading the gl_InstanceIndex variable
        dbg!(object_id);
        render_bundle_encoder.draw_indexed(0..rendering_data.index_count, 0, object_id..object_id+1);
        let render_bundle = render_bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
            label: Some("Render bundle for Rendering Block"),
        });
        self.renderables.push(render_bundle);
    }

    pub fn render(&self, manager: &device_manager::Manager, target_texture: &wgpu::TextureView, camera: &Camera) {
        // update the uniform buffer containing the camera
        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);
        manager.queue.write_buffer(&self.camera_uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let initialize_picking = vec![std::f32::NAN; self.renderables.len()];
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
                        attachment: target_texture,
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

