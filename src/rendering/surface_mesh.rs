use crate::compute_block::ComputeBlock;

fn create_grid_buffer_index(x_size: usize, y_size: usize, flag_pattern: bool) -> Vec<u32> {
    // the grid has indices growing first along x, then along y
    let mut index_buffer = Vec::<u32>::new();
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
                index_buffer.push(bot_left_idx);
                index_buffer.push(bot_right_idx);
                index_buffer.push(top_left_idx);

                index_buffer.push(top_right_idx);
                index_buffer.push(top_left_idx);
                index_buffer.push(bot_right_idx);
            } else {
                // triangulate the quad using the "standard" pattern
                index_buffer.push(bot_left_idx);
                index_buffer.push(bot_right_idx);
                index_buffer.push(top_right_idx);

                index_buffer.push(top_right_idx);
                index_buffer.push(top_left_idx);
                index_buffer.push(bot_left_idx);
            }
        }
    }

    index_buffer
}

pub struct SurfaceMesh {
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
}

impl SurfaceMesh {
    pub fn new(device: &wgpu::Device, surface_block: &ComputeBlock) -> Self {
        let dimensions = surface_block.get_dimensions().clone();
        let (param_1, param_2) = dimensions.as_2d().unwrap();

        // the index buffer is set in stone: once computed, we do not need to touch it ever again
        use wgpu::util::DeviceExt;
        let index_vector = create_grid_buffer_index(param_1.size, param_2.size, true);
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&index_vector),
                usage: wgpu::BufferUsage::INDEX,
            });

        Self {
            index_buffer,
            num_elements: index_vector.len() as u32,
        }
    }

}
