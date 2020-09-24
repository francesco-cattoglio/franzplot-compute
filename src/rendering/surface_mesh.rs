use crate::device_manager::Manager;
use crate::compute_block::Dimensions;
use crate::compute_block::ComputeBlock;

const LOCAL_SIZE_X: usize = 16;
const LOCAL_SIZE_Y: usize = 16;

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
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    dimensions: Dimensions,
    compute_normals_pipeline: wgpu::ComputePipeline,
    compute_normals_bind_group: wgpu::BindGroup,
    pub num_elements: u32,
}

impl SurfaceMesh {
    pub fn new(device: &wgpu::Device, surface_block: &ComputeBlock) -> Self {
        let computed_surface = surface_block.get_buffer();
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

        // The vertex buffer will be updated every time the underlying surface gets recomputed.
        // The correct size depends on the size of the vertex description used by the renderer.
        let num_vertices = param_1.size * param_2.size;
        let vertex_required_mem = 12 * std::mem::size_of::<f32>();
        let vertex_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: None,
                mapped_at_creation: false,
                size: (num_vertices*vertex_required_mem) as wgpu::BufferAddress,
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::MAP_READ,
            });

        // TODO: we should only create one of these compute shaders instead of one for each
        // SurfaceMesh
        let shader_source = include_str!("surface_normals.cs");
        let mut bindings = Vec::<CustomBindDescriptor>::new();
        // add descriptor for input buffers
        bindings.push(CustomBindDescriptor {
            position: 0,
            buffer_slice: computed_surface.slice(..)
        });
        use crate::shader_processing::*;
        // add descriptor for output buffer
        bindings.push(CustomBindDescriptor {
            position: 1,
            buffer_slice: vertex_buffer.slice(..)
        });
        let (compute_normals_pipeline, compute_normals_bind_group) = compute_shader_no_globals(shader_source, &bindings, &device, Some("Surface Normals"));

        Self {
            compute_normals_pipeline,
            compute_normals_bind_group,
            dimensions,
            index_buffer,
            vertex_buffer,
            num_elements: index_vector.len() as u32,
        }
    }

    pub fn update(&self, manager: &Manager) {

        let mut encoder =
            manager.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder this time"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_normals_pipeline);
            compute_pass.set_bind_group(0, &self.compute_normals_bind_group, &[]);
            let (par_1, par_2) = self.dimensions.as_2d().unwrap();
            compute_pass.dispatch((par_1.size/LOCAL_SIZE_X) as u32, (par_2.size/LOCAL_SIZE_Y) as u32, 1);
        }
        let compute_queue = encoder.finish();
        manager.queue.submit(std::iter::once(compute_queue));
        //let computed_copy = crate::copy_buffer_as_f32(&self.vertex_buffer, &manager.device);
        //println!("computed copy: {:?}", computed_copy);
    }
}
