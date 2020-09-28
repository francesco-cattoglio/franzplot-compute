use crate::compute_block::*;
use crate::device_manager::Manager;

fn create_grid_buffer_index(device: &wgpu::Device, x_size: usize, y_size: usize, flag_pattern: bool) -> (wgpu::Buffer, usize) {
    // the grid has indices growing first along x, then along y
    let mut index_vector = Vec::<u32>::new();
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
                // triangulate the quad using the "flag" pattern
                index_vector.push(bot_left_idx);
                index_vector.push(bot_right_idx);
                index_vector.push(top_left_idx);

                index_vector.push(top_right_idx);
                index_vector.push(top_left_idx);
                index_vector.push(bot_right_idx);
            } else {
                // triangulate the quad using the "standard" pattern
                index_vector.push(bot_left_idx);
                index_vector.push(bot_right_idx);
                index_vector.push(top_right_idx);

                index_vector.push(top_right_idx);
                index_vector.push(top_left_idx);
                index_vector.push(bot_left_idx);
            }
        }
    }

    use wgpu::util::DeviceExt;
    let index_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&index_vector),
            usage: wgpu::BufferUsage::INDEX,
    });
    (index_buffer, index_vector.len())
}

pub fn block_to_renderable(manager: &Manager, compute_block: &ComputeBlock, camera_bind_group: &wgpu::BindGroup, pipeline: &wgpu::RenderPipeline, texture_bind_group: &wgpu::BindGroup) -> Option<wgpu::RenderBundle> {
        match compute_block {
            ComputeBlock::SurfaceRenderer(data) => {
                let (param_1, param_2) = data.out_dim.as_2d().unwrap();
                let (index_buffer, indices_count) = create_grid_buffer_index(&manager.device, param_1.size, param_2.size, true);

                let mut render_bundle_encoder = manager.device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor{
                    label: Some("render bundle encoder for surface"),
                    color_formats: &[super::SWAPCHAIN_FORMAT],
                    depth_stencil_format: Some(super::DEPTH_FORMAT),
                    sample_count: 1,
                });
                render_bundle_encoder.set_pipeline(pipeline);
                render_bundle_encoder.set_vertex_buffer(0, data.vertex_buffer.slice(..));
                render_bundle_encoder.set_index_buffer(index_buffer.slice(..));
                render_bundle_encoder.set_bind_group(0, texture_bind_group, &[]);
                render_bundle_encoder.set_bind_group(1, camera_bind_group, &[]);
                render_bundle_encoder.draw_indexed(0..indices_count as u32, 0, 0..1);
                let render_bundle = render_bundle_encoder.finish(&wgpu::RenderBundleDescriptor{
                    label: Some("render bundle for surface"),
                });

                Some(
                    render_bundle,
                )
            },
            _ => None,
        }

    }

