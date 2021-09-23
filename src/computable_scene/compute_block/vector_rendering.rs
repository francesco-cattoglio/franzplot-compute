use crate::rendering::{StandardVertexData, GLSL_STANDARD_VERTEX_STRUCT};
use crate::node_graph::AVAILABLE_SIZES;
use super::{ComputeBlock, BlockCreationError, BlockId};
use super::{ProcessedMap, ProcessingResult};

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
    pub vertex_count: usize,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub out_buffer: wgpu::Buffer,
    pub material_id: usize,
    pub compute_pipeline: wgpu::ComputePipeline,
    pub compute_bind_group: wgpu::BindGroup,
}

impl VectorRenderingData {
    pub fn new(device: &wgpu::Device, processed_blocks: &ProcessedMap, descriptor: VectorRenderingBlockDescriptor) -> Result<Self, BlockCreationError> {
        // we fetch the application_points and vector inputs
        let appl_point_id = descriptor.application_point.ok_or(BlockCreationError::InputMissing(" This Vector Renderer node \n is missing an input "))?;
        let found_element = processed_blocks.get(&appl_point_id).ok_or(BlockCreationError::InternalError("Vector renderer application point input does not exist in the block map".into()))?;
        let appl_point_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;
        let appl_point_data = if let ComputeBlock::Point(data) = appl_point_block {
            data
        } else {
            return Err(BlockCreationError::InputInvalid(" the first input provided \n is not a point "));
        };

        let vector_id = descriptor.vector.ok_or(BlockCreationError::InputMissing(" This Vector Renderer node \n is missing an input "))?;
        let found_element = processed_blocks.get(&vector_id).ok_or(BlockCreationError::InternalError("Vector renderer vector input does not exist in the block map".into()))?;
        let vector_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;
        let vector_data = if let ComputeBlock::Vector(data) = vector_block {
            data
        } else {
            return Err(BlockCreationError::InputInvalid(" the second input provided \n is not a vector "));
        };

        // then we create the basic shape of the arrow. It points upwards and has length 1.0
        // we will transform the vertex buffer in our compute shader
        let radius = AVAILABLE_SIZES[descriptor.thickness];
        let n_circle_points = (descriptor.thickness + 3)*2;
        let (index_buffer, index_count, vertex_buffer, vertex_count) = create_arrow_buffers(device, radius, n_circle_points);

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

{vertex_struct}

layout(set = 0, binding = 0) buffer ArrowVertices {{
    Vertex arrow_vertices[];
}};

layout(set = 0, binding = 1) buffer ApplPoint {{
    vec4 appl_point;
}};

layout(set = 0, binding = 2) buffer Vector {{
    vec4 vector;
}};

layout(set = 0, binding = 3) buffer OutputData {{
    Vertex out_buff[];
}};

void main() {{
    uint idx = gl_GlobalInvocationID.x;

    Vertex in_vertex = arrow_vertices[idx];
    float s_z = length(vector); // to make the vector of the correct length, scale along z
    vec4 direction = normalize(vector);
    float angle_z;
    // workaround MacOS bug: atan2 seems to give the wrong result
    if (direction.y == 0.0) {{
        if (direction.x > 0) {{
            angle_z = -0.5*3.141527;
        }} else {{
            angle_z =  0.5*3.141527;
        }}
    }} else {{
        angle_z = -1.0 * atan(direction.x, direction.y);
    }}
    float angle_x = -1.0 * acos(direction.z);
    float cos_t = cos(angle_z);
    float sin_t = sin(angle_z);
    float cos_p = cos(angle_x);
    float sin_p = sin(angle_x);

    mat4 matrix;
    matrix[0] = vec4(cos_t, sin_t, 0.0, 0.0); // first column
    matrix[1] = vec4(-cos_p*sin_t, cos_p*cos_t, sin_p, 0.0); // 2nd column
    matrix[2] = vec4(sin_p*sin_t, -sin_p*cos_t, cos_p, 0.0); // 3rd column
    matrix[3] = appl_point; // 4th column

    // since the matrix is orthogonal, we can apply it to normals as well.
    // BEWARE: this is not correct because the vertex normal should be transformed when we scale
    // the arrow. But the normals were wrong to begin with! (see create_arrow_buffers function)

    out_buff[idx].position = matrix * vec4(in_vertex.position.xy, s_z * in_vertex.position.z, 1.0);
    out_buff[idx].normal = matrix * in_vertex.normal;
    out_buff[idx].uv_coords = in_vertex.uv_coords;
    out_buff[idx]._padding = in_vertex._padding;
}}
"##, vertex_struct=GLSL_STANDARD_VERTEX_STRUCT, dimx=vertex_count);

        let out_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            mapped_at_creation: false,
            size: (vertex_count * std::mem::size_of::<StandardVertexData>()) as wgpu::BufferAddress,
            // Beware:copy and map are only needed when debugging/inspecting
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::MAP_READ,
        });

        let bindings = [
            // add descriptor for input buffers
            CustomBindDescriptor {
                position: 0,
                buffer: &vertex_buffer
            },
            CustomBindDescriptor {
                position: 1,
                buffer: &appl_point_data.out_buffer
            },
            CustomBindDescriptor {
                position: 2,
                buffer: &vector_data.out_buffer
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 3,
                buffer: &out_buffer
            }
        ];

        use crate::shader_processing::*;
        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, &shader_source, &bindings, None, Some("Curve Normals"))?;


        Ok(Self {
            index_buffer,
            index_count,
            vertex_buffer,
            vertex_count,
            out_buffer,
            material_id: descriptor.material,
            compute_pipeline,
            compute_bind_group,
        })
    }

    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{
            label: Some("vector rendering compute pass")
        });
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
        // BEWARE: just like we did with other shaders, we wrote the size of the buffer inside the local shader
        // dimensions, therefore the whole compute will always take just 1 dispatch
        compute_pass.dispatch(1, 1, 1);
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

fn create_arrow_buffers(device: &wgpu::Device, radius: f32, circle_points: usize) -> (wgpu::Buffer, u32, wgpu::Buffer, usize) {
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
            // we add a very tiny amount of bias to z to prevent z-fighting when showing the vector of a plane
            let position = [0.5*radius*theta.cos(), 0.5*radius*theta.sin(), 0.001, 1.0];
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
            usage: wgpu::BufferUsages::INDEX,
    });
    let vertex_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertex_vector),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::MAP_READ,
    });
    dbg!(vertex_vector.len());
    (index_buffer, index_vector.len() as u32, vertex_buffer, vertex_vector.len())
}



