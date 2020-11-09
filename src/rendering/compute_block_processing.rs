use crate::compute_block::*;
use crate::device_manager::Manager;
use super::Renderer;

pub fn block_to_renderable(manager: &Manager, compute_block: &ComputeBlock, renderer: &Renderer) -> Option<wgpu::RenderBundle> {
        match compute_block {
            ComputeBlock::Rendering(data) => {
                let mut render_bundle_encoder = manager.device.create_render_bundle_encoder(
                    &wgpu::RenderBundleEncoderDescriptor{
                        label: Some("Render bundle encoder for Rendering Block"),
                        color_formats: &[super::SWAPCHAIN_FORMAT],
                        depth_stencil_format: Some(super::DEPTH_FORMAT),
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

