use crate::rendering::camera::Camera;
use crate::rendering::texture;
use crate::rendering::{ CAMERA_LAYOUT_DESCRIPTOR, TEXTURE_LAYOUT_DESCRIPTOR, DEPTH_FORMAT, SWAPCHAIN_FORMAT };
use crate::device_manager;
use super::compute_chain::ComputeChain;
use super::compute_block::ComputeBlock;
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
    camera: Camera,
    pipeline_2d: wgpu::RenderPipeline,
    renderables: Vec<wgpu::RenderBundle>,
    texture: texture::Texture,
    texture_bind_group: wgpu::BindGroup,
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture: texture::Texture,
    clear_color: wgpu::Color,
}

impl SceneRenderer {
    pub fn new(manager: &device_manager::Manager) -> Self {
        let camera = Camera::from_height_width(manager.sc_desc.width as f32, manager.sc_desc.height as f32);

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
                    resource: wgpu::BindingResource::Buffer(camera_uniform_buffer.slice(..)),
                },
            ],
            label: Some("Camera bind group"),
        });
        use anyhow::Context;
        let path = std::path::Path::new("./resources/grid_color.png");
        let diffuse_texture = texture::Texture::load(&manager.device, &manager.queue, path, "cube-diffuse").context("failed to load texture").unwrap();

        let texture_bind_layout =
            manager.device.create_bind_group_layout(&TEXTURE_LAYOUT_DESCRIPTOR);
        let texture_bind_group = manager.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_layout,
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

        use crate::rendering::create_2d_pipeline;
        let pipeline_2d = create_2d_pipeline(&manager.device, &camera_bind_layout, &texture_bind_layout);
        let depth_texture = texture::Texture::create_depth_texture(&manager.device, &manager.sc_desc, "depth_texture");

        let clear_color = wgpu::Color::BLACK;

        let renderables = Vec::<wgpu::RenderBundle>::new();

        Self {
            camera,
            clear_color,
            renderables,
            texture: diffuse_texture,
            texture_bind_group,
            depth_texture,
            camera_uniform_buffer,
            camera_bind_group,
            pipeline_2d,
        }
    }

    pub fn update_renderables (&mut self, device: &wgpu::Device, chain: &ComputeChain,) {
        self.renderables.clear();
        for compute_block in chain.valid_blocks() {
            let maybe_renderable = block_to_renderable(device, compute_block, &self);
            if let Some(renderable) = maybe_renderable {
                self.renderables.push(renderable);
            }
        }
    }

    pub fn render(&self, manager: &device_manager::Manager, target_texture: &wgpu::TextureView, camera: &Camera) {
        // update the uniform buffer containing the camera
        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);
        manager.queue.write_buffer(&self.camera_uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

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
    }
}

// UTILITY FUNCTIONS
// TODO: decide about moving this into the SceneRenderer impl block

fn block_to_renderable(device: &wgpu::Device, compute_block: &ComputeBlock, renderer: &SceneRenderer) -> Option<wgpu::RenderBundle> {
        match compute_block {
            ComputeBlock::Rendering(data) => {
                let mut render_bundle_encoder = device.create_render_bundle_encoder(
                    &wgpu::RenderBundleEncoderDescriptor{
                        label: Some("Render bundle encoder for Rendering Block"),
                        color_formats: &[SWAPCHAIN_FORMAT],
                        depth_stencil_format: Some(DEPTH_FORMAT),
                        sample_count: 1,
                    }
                );
                render_bundle_encoder.set_pipeline(&renderer.pipeline_2d);
                render_bundle_encoder.set_vertex_buffer(0, data.vertex_buffer.slice(..));
                render_bundle_encoder.set_index_buffer(data.index_buffer.slice(..));
                render_bundle_encoder.set_bind_group(0, &renderer.camera_bind_group, &[]);
                render_bundle_encoder.set_bind_group(1, &renderer.texture_bind_group, &[]);
                render_bundle_encoder.draw_indexed(0..data.index_count, 0, 0..1);
                let render_bundle = render_bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
                    label: Some("Render bundle for Rendering Block"),
                });
                Some(render_bundle)
            },
            _ => None,
        }

    }

