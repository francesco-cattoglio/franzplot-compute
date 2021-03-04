use super::StandardVertexData;

pub const MODEL_CHUNK_VERTICES: usize = 32;

#[derive(Debug)]
pub struct Model {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub vertex_count: usize,
    pub index_count: u32,
}

impl Model {
    pub fn from_obj(device: &wgpu::Device, data: &obj::ObjData) -> Self {
        let mut indices = Vec::<u32>::new();
        let mut vertices = Vec::<StandardVertexData>::new();

        for object in data.objects.iter() {
            for group in object.groups.iter() {
                for polygon in group.polys.iter() {
                    let obj::SimplePolygon(face_vertices) = &polygon;
                    // triangulate the face
                    assert!(face_vertices.len() >= 3);
                    let first_face_idx = vertices.len() as u32;
                    let last_face_idx = first_face_idx + face_vertices.len() as u32 - 1;
                    let segment_indices: Vec<u32> = (first_face_idx + 1 ..= last_face_idx).collect();
                    for segment in segment_indices.windows(2) {
                        indices.push(first_face_idx);
                        indices.push(segment[0]);
                        indices.push(segment[1]);
                    }
                    for obj::IndexTuple(pos, uv, norm) in face_vertices.iter() {
                        let p = data.position[*pos];
                        let position = [p[0], p[1], p[2], 1.0];
                        let normal = if let Some(n) = norm.map(|x| {data.normal[x]}) {
                            [n[0], n[1], n[2], 0.0]
                        } else {
                            [0.0, 0.0, 0.0, 0.0]
                        };
                        let uv_coords = if let Some(coords) = uv.map(|x| {data.texture[x]}) {
                            [coords[0], coords[1]]
                        } else {
                            [0.0, 0.0]
                        };
                        let vertex_data = StandardVertexData {
                            position,
                            normal,
                            uv_coords,
                            _padding: [2.22, 3.33],
                        };
                        vertices.push(vertex_data);
                    }
                }
            }
        }

        let vertices_remainder = vertices.len() % MODEL_CHUNK_VERTICES;
        if vertices_remainder != 0 {
            // extend the vertices to round up its size up to the next MODEL_CHUNK_VERTICES
            let new_size = vertices.len() + (MODEL_CHUNK_VERTICES - vertices_remainder);
            vertices.resize_with(new_size, StandardVertexData::default);
        }

        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("model vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::MAP_READ,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("model index buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsage::INDEX,
        });

        Self {
            vertex_buffer,
            vertex_count: vertices.len(),
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}
