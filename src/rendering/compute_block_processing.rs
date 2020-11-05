use crate::compute_block::*;
use crate::device_manager::Manager;
use super::Renderer;

// a tube is a sequence of circles connected to each other, with 2 flat caps at the beginning and
// at the end.
fn create_tube_segment(segment: (usize, usize), circle_points: usize) -> Vec::<u32> {
    let mut indices = Vec::<u32>::new();
    // the names are not very good.
    // segment_start is the index of the first vertex in the first circle of the segment
    // segment_end is the index of the first vertex in the second circle.
    let segment_start = (segment.0 * circle_points) as u32;
    let segment_end = (segment.1 * circle_points) as u32;

    // first go through all the sides except for the very last one
    for i in 0 .. (circle_points - 1) as u32 {
        // two triangles per each face
        indices.extend_from_slice(&[segment_start + i, segment_start + i + 1, segment_end + i + 1]);
        indices.extend_from_slice(&[segment_start + i, segment_end + i + 1, segment_end + i]);
    }
    // then add in the last one. We could have used a % to make sure the output would be correct
    // but it is not worth it, KISS principle!
    indices.extend_from_slice(&[segment_end - 1, segment_start, segment_end]);
    indices.extend_from_slice(&[segment_end + (circle_points - 1) as u32, segment_end - 1, segment_end]);

    indices
}

fn create_tube_buffer_index(device: &wgpu::Device, x_size: usize, circle_points: usize) -> (wgpu::Buffer, usize) {
    assert!(circle_points > 2);
    let mut index_vector = Vec::<u32>::new();

    for i in 0 .. x_size - 1 {
        let segment = (i, i+1);
        let mut segment_indices = create_tube_segment(segment, circle_points);
        index_vector.append(&mut segment_indices);
    }

    // TODO: add caps

    use wgpu::util::DeviceExt;
    let index_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&index_vector),
            usage: wgpu::BufferUsage::INDEX,
    });
    (index_buffer, index_vector.len())
}

fn create_tube_vertex_index(device: &wgpu::Device, x_size: usize, n_circle_points: usize) -> wgpu::Buffer {
    let mut circle_points = Vec::<f32>::with_capacity(4 * n_circle_points);
    for i in 0 .. n_circle_points {
        let angle = 2.0 * std::f32::consts::PI * i as f32 / n_circle_points as f32;
        // I want my circle to have the first vertex in (0, 0, 1)
        circle_points.push(f32::sin(angle));
        circle_points.push(0.0);
        circle_points.push(f32::cos(angle));
        circle_points.push(1.0);
    }

    let mut tube_vertices = Vec::<f32>::with_capacity(4 * n_circle_points * x_size);
    // make up a vertex array by as many copies as needed
    for _i in 0 .. x_size {
        tube_vertices.extend_from_slice(&circle_points);
    }

    use wgpu::util::DeviceExt;
    let vertex_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&tube_vertices),
            usage: wgpu::BufferUsage::VERTEX,
    });
    vertex_buffer

}

fn create_lines_buffer_index(device: &wgpu::Device, x_size: usize) -> (wgpu::Buffer, usize) {
    let mut index_vector = Vec::<u32>::new();
    for i in 0 .. x_size {
        index_vector.push(i as u32);
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

pub fn block_to_renderable(manager: &Manager, compute_block: &ComputeBlock, renderer: &Renderer) -> Option<wgpu::RenderBundle> {
        match compute_block {
            ComputeBlock::SurfaceRenderer(data) => {
                let mut render_bundle_encoder = manager.device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor{
                    label: Some("render bundle encoder for surface"),
                    color_formats: &[super::SWAPCHAIN_FORMAT],
                    depth_stencil_format: Some(super::DEPTH_FORMAT),
                    sample_count: 1,
                });
                let render_bundle = match &data.out_dim {
                    Dimensions::D0 =>  {
                        unimplemented!()
                    }
                    Dimensions::D1(param_1) => {
                    use crate::util::copy_buffer_as_f32;
                    let input_buffer = copy_buffer_as_f32(&data.out_buffer, &manager.device);
                    dbg!(&input_buffer);
                        let curvedata_bind_group = manager.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            layout: &renderer.curvedata_bind_layout,
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::Buffer(data.out_buffer.slice(..)),
                                    },
                                ],
                                label: Some("curvedata binding group")
                            });

                        render_bundle_encoder.set_pipeline(&renderer.pipeline_1d);
                        let n_circle_points: usize = 3;
                        let (index_buffer, indices_count) = create_tube_buffer_index(&manager.device, param_1.size, n_circle_points);
                        let vertex_buffer = create_tube_vertex_index(&manager.device, param_1.size, n_circle_points);
                        render_bundle_encoder.set_vertex_buffer(0, vertex_buffer.slice(..));
                        render_bundle_encoder.set_index_buffer(index_buffer.slice(..));
                        render_bundle_encoder.set_bind_group(0, &curvedata_bind_group, &[]);
                        render_bundle_encoder.set_bind_group(1, &renderer.camera_bind_group, &[]);
                        render_bundle_encoder.set_bind_group(2, &renderer.texture_bind_group, &[]);
                        render_bundle_encoder.draw_indexed(0..indices_count as u32, 0, 0..1);
                        render_bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
                            label: Some("render bundle for curve"),
                        })
                    }
                    Dimensions::D2(param_1, param_2) => {
                        render_bundle_encoder.set_pipeline(&renderer.pipeline_2d);
                        let (index_buffer, indices_count) = create_grid_buffer_index(&manager.device, param_1.size, param_2.size, true);
                        render_bundle_encoder.set_vertex_buffer(0, data.out_buffer.slice(..));
                        render_bundle_encoder.set_index_buffer(index_buffer.slice(..));
                        render_bundle_encoder.set_bind_group(0, &renderer.camera_bind_group, &[]);
                        render_bundle_encoder.set_bind_group(1, &renderer.texture_bind_group, &[]);
                        render_bundle_encoder.draw_indexed(0..indices_count as u32, 0, 0..1);
                        render_bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
                            label: Some("render bundle for surface"),
                        })
                    }
                };
                Some(render_bundle)
            },
            _ => None,
        }

    }

