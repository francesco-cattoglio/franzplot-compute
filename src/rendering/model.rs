use super::StandardVertexData;

#[derive(Debug)]
pub struct Model {
    vertices: Vec<StandardVertexData>,
    indices: Vec<u32>,
}

impl Model {
    pub fn from_obj(data: &obj::ObjData) -> Self {
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
                            [0.0, 0.0, 0.0, 1.0]
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

        Self {
            vertices,
            indices,
        }
    }
}
