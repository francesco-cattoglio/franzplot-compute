use crate::rendering::{StandardVertexData, GLSL_STANDARD_VERTEX_STRUCT};
use crate::node_graph::AVAILABLE_SIZES;
use crate::rendering::model::{ Model, MODEL_CHUNK_VERTICES };
use super::{ComputeBlock, BlockCreationError, Dimensions, BlockId};
use super::{ProcessedMap, ProcessingResult};

const LOCAL_SIZE_X: usize = 16;
const LOCAL_SIZE_Y: usize = 16;

#[derive(Debug)]
pub struct VectorRenderingBlockDescriptor {
    pub application_point: Option<BlockId>,
    pub vector: Option<BlockId>,
    pub thickness: usize,
    pub material: usize,
}
impl VectorRenderingBlockDescriptor {
    pub fn make_block(self, device: &wgpu::Device, processed_blocks: &ProcessedMap) -> ProcessingResult {
        Ok(ComputeBlock::VectorRendering(VectorRenderingData::new(device, processed_blocks, self)?))
    }
}

pub struct VectorRenderingData {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub material_id: usize,
    //pub compute_pipeline: wgpu::ComputePipeline,
    //compute_bind_group: wgpu::BindGroup,
}

impl VectorRenderingData {
    pub fn new(device: &wgpu::Device, processed_blocks: &ProcessedMap, descriptor: VectorRenderingBlockDescriptor) -> Result<Self, BlockCreationError> {
        let radius = AVAILABLE_SIZES[descriptor.thickness];
        let n_circle_points = (descriptor.thickness + 3)*2;
        let (index_buffer, index_count, vertex_buffer) = create_arrow_buffers(device, radius, n_circle_points);
        Ok(Self {
            index_buffer,
            index_count,
            vertex_buffer,
            material_id: descriptor.material,
        })
    }
}

// TODO: DRY!!! This is copied from rendering.rs, there will be some parts in common with it, no
// doubts about it.
fn create_arrow_segment(segment: (usize, usize), circle_points: usize) -> Vec::<u32> {
    let mut indices = Vec::<u32>::new();
    // the variable names are a bit misleading, so here is an explanation:
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

fn create_arrow_cap(segment: usize, cap_point_index: u32, circle_points: usize) -> Vec::<u32> {
    let mut indices = Vec::<u32>::new();
    // the variable names are a bit misleading, so here is an explanation:
    // segment_start is the index of the first vertex in the first circle of the segment
    // segment_end is the index of the first vertex in the second circle.
    let segment_start = (segment * circle_points) as u32;

    // first go through all the sides except for the very last one
    for i in 0 .. (circle_points - 1) as u32 {
        // two triangles per each face
        indices.extend_from_slice(&[segment_start + i, segment_start + i + 1, cap_point_index]);
    }
    // then add in the last one. We could have used a % to make sure the output would be correct
    // but it is not worth it, KISS principle!
    indices.extend_from_slice(&[segment_start + circle_points as u32 - 1, segment_start, cap_point_index]);

    indices
}

fn create_arrow_buffers(device: &wgpu::Device, radius: f32, circle_points: usize) -> (wgpu::Buffer, u32, wgpu::Buffer) {
    assert!(circle_points > 3);
    let head_z_start: f32 = 1.0 - 1.5*radius;
    let mut index_vector = Vec::<u32>::new();
    let mut vertex_vector = Vec::<StandardVertexData>::new();

    // first segment: the cylinder part of the arrow
    {
        let segment = (0, 1);
        let mut segment_indices = create_arrow_segment(segment, circle_points);
        index_vector.append(&mut segment_indices);
        // we also compute the corresponding vertices.
        // start with the first circle
        for i in 0 .. circle_points {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / (circle_points - 1) as f32;
            let position = [0.5*radius*theta.cos(), 0.5*radius*theta.sin(), 0.0, 1.0];
            let normal = [theta.cos(), theta.sin(), 0.0, 0.0];
            vertex_vector.push(StandardVertexData {
                position,
                normal,
                uv_coords: [0.0, 0.0],
                _padding: [0.123, 0.456],
            });
        }
        // add the second circle: same normals, different z for the points
        for i in 0 .. circle_points {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / (circle_points - 1) as f32;
            let position = [0.5*radius*theta.cos(), 0.5*radius*theta.sin(), head_z_start, 1.0];
            let normal = [theta.cos(), theta.sin(), 0.0, 0.0];
            vertex_vector.push(StandardVertexData {
                position,
                normal,
                uv_coords: [0.0, 0.0],
                _padding: [0.123, 0.456],
            });
        }
    }
    // second segment: the expanding part of the arrow
    {
        let segment = (2, 3);
        let mut segment_indices = create_arrow_segment(segment, circle_points);
        index_vector.append(&mut segment_indices);
        // we need to add the points that correspond to the circle number 2. The ones for the
        // add the circle number 2: normals look downwards
        for i in 0 .. circle_points {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / (circle_points - 1) as f32;
            let position = [0.5*radius*theta.cos(), 0.5*radius*theta.sin(), head_z_start, 1.0];
            let normal = [0.0, 0.0, -1.0, 0.0];
            vertex_vector.push(StandardVertexData {
                position,
                normal,
                uv_coords: [0.0, 0.0],
                _padding: [0.123, 0.456],
            });
        }
        // This time we change and use the entire radius, instead of only half,
        // and set the normals to look downwards
        for i in 0 .. circle_points {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / (circle_points - 1) as f32;
            let position = [radius*theta.cos(), radius*theta.sin(), head_z_start, 1.0];
            let normal = [0.0, 0.0, -1.0, 0.0];
            vertex_vector.push(StandardVertexData {
                position,
                normal,
                uv_coords: [0.0, 0.0],
                _padding: [0.123, 0.456],
            });
        }
    }
    // last part: the hat of the arrow
    // before proceding, we want to create a new circle, because we need to duplicate the vertices
    // (to show a proper edge we need to split the normals!)
    // we don't point the normals correctly, we just point them outwards, which should
    // look good enough.
    {
        for i in 0 .. circle_points {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / (circle_points - 1) as f32;
            let position = [radius*theta.cos(), radius*theta.sin(), head_z_start, 1.0];
            let normal = [theta.cos(), theta.sin(), 0.0, 0.0];
            vertex_vector.push(StandardVertexData {
                position,
                normal,
                uv_coords: [0.0, 0.0],
                _padding: [0.123, 0.456],
            });
        }
        // now, add the final point for the cap.
        vertex_vector.push(StandardVertexData {
            position: [0.0, 0.0, 1.0, 1.0],
            normal: [0.0, 0.0, 1.0, 0.0],
            uv_coords: [0.0, 0.0],
            _padding: [0.123, 0.456],
        });
        // and create the indices for it
        let circle = 4;
        let last_vertex_index = (vertex_vector.len() - 1) as u32;
        let mut cap_indices = create_arrow_cap(circle, last_vertex_index, circle_points);
        index_vector.append(&mut cap_indices);
    }
    // one-pass-last part: close off the bottom of the the arrow
    {
        vertex_vector.push(StandardVertexData {
            position: [0.0, 0.0, 0.0, 1.0],
            normal: [0.0, 0.0, -1.0, 0.0],
            uv_coords: [0.0, 0.0],
            _padding: [0.123, 0.456],
        });
        // and create the indices for it
        let circle = 0;
        let last_vertex_index = (vertex_vector.len() - 1) as u32;
        // BEWARE: this will have the triangle to come out reversed, i.e. with CW ordering
        // instead of CCW
        let mut cap_indices = create_arrow_cap(circle, last_vertex_index, circle_points);
        index_vector.append(&mut cap_indices);
    }

    use wgpu::util::DeviceExt;
    let index_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&index_vector),
            usage: wgpu::BufferUsage::INDEX,
    });
    let vertex_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertex_vector),
            usage: wgpu::BufferUsage::VERTEX,
    });
    (index_buffer, index_vector.len() as u32, vertex_buffer)
}



