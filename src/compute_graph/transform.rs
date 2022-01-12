use std::collections::BTreeMap;
use std::rc::Rc;

use super::Operation;
use super::Parameter;
use crate::rendering::StandardVertexData;
use crate::rendering::model::MODEL_CHUNK_VERTICES;
use super::{SingleDataResult, ProcessingError};
use super::{DataID, Data};
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};

// TODO: use the chunk size instead of the magic number "16" everywhere in this file
// possibly using a const-time formatting library like https://docs.rs/const_format/
const CHUNK_SIZE: usize = super::Parameter::POINTS_PER_SEGMENT;

pub fn create(
    device: &wgpu::Device,
    data_map: &BTreeMap<DataID, Data>,
    geometry_id: Option<DataID>,
    matrix_id: Option<DataID>,
) -> SingleDataResult {
    let data_id = geometry_id
        .ok_or_else(|| ProcessingError::InputMissing(" This Transform node \n is missing its Geometry input ".into()))?;
    let geometry_data = data_map
        .get(&data_id)
        .ok_or(ProcessingError::NoInputData)?;

    let data_id = matrix_id.ok_or_else(|| ProcessingError::InputMissing(" This Transform node \n is missing its Matrix input ".into()))?;
    let matrix_data = data_map
        .get(&data_id)
        .ok_or(ProcessingError::NoInputData)?;

    match (&geometry_data, &matrix_data) {
        (Data::Geom0D{buffer: geom_buffer}, Data::Matrix0D{buffer: matrix_buffer})
            => t_0d_0d(device, geom_buffer, matrix_buffer),

        (Data::Geom0D{buffer: geom_buffer}, Data::Matrix1D{buffer, param})
            => t_0d_up_1d(device, geom_buffer, buffer, param),

        (Data::Geom1D{buffer, param}, Data::Matrix0D{buffer: matrix_buffer})
            => t_1d_1d(device, buffer, param, matrix_buffer),

        (Data::Geom1D{buffer, param}, Data::Matrix1D{buffer: matrix_buffer, param: matrix_param})
            if !param.is_equal(matrix_param)?
                => t_1d_up_2d(device, buffer, param, matrix_buffer, matrix_param),

        (Data::Geom1D{buffer, param}, Data::Matrix1D{buffer: matrix_buffer, param: matrix_param})
            if param.is_equal(matrix_param)?
                => t_1d_same_param(device, buffer, matrix_buffer, matrix_param),

        (Data::Geom2D{buffer, param1, param2}, Data::Matrix0D{buffer: matrix_buffer})
            => t_2d_2d(device, buffer, param1, param2, matrix_buffer),

        (Data::Geom2D{buffer, param1, param2}, Data::Matrix1D{buffer: matrix_buffer, param: matrix_param})
            if param1.is_equal(matrix_param)? || param2.is_equal(matrix_param)?
                => t_2d_same_param(device, buffer, param1, param2, matrix_buffer, matrix_param),

        (Data::Geom2D{..}, Data::Matrix1D{..})
            => Err(ProcessingError::IncorrectInput(" this operation would create \n an object with three parameters, \n which is not supported ".into())),

        (Data::Prefab { vertex_buffer, chunks_count, index_buffer, index_count }, Data::Matrix0D { buffer: matrix_buffer })
            => t_prefab(device, vertex_buffer, *chunks_count, index_buffer, *index_count, matrix_buffer),

        (Data::Prefab { .. }, Data::Matrix1D { .. })
            => Err(ProcessingError::IncorrectInput(" parametric transforms on primitives \n are not allowed ".into())),

        _ => Err(ProcessingError::InternalError("unhandled transform case".into()))
    }

}

fn t_0d_0d(
    device: &wgpu::Device,
    geom_buffer: &wgpu::Buffer,
    matrix_buffer: &wgpu::Buffer,
    ) -> SingleDataResult {
    let wgsl_source = r##"
struct PointBuffer {
    position: vec4<f32>;
};

struct MatrixBuffer {
    matrix: mat4x4<f32>;
};

[[group(0), binding(0)]] var<storage, read> in_point: PointBuffer;
[[group(0), binding(1)]] var<storage, read> in_matrix: MatrixBuffer;
[[group(0), binding(2)]] var<storage, read_write> output: PointBuffer;

[[stage(compute), workgroup_size(1)]]
fn main() {
    output.position = in_matrix.matrix * in_point.position;
}
"##.to_string();

    // A point has a fixed size
    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Vec4>());
    let bind_info = vec![
        BindInfo {
            buffer: geom_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: matrix_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &output_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [1, 1, 1],
    };
    let new_data = Data::Geom0D {
        buffer: output_buffer,
    };

    Ok((new_data, operation))
}

fn t_0d_up_1d(
    device: &wgpu::Device,
    geom_buffer: &wgpu::Buffer,
    matrix_buffer: &wgpu::Buffer,
    matrix_param: &Parameter,
    ) -> SingleDataResult {
    let wgsl_source = r##"
struct PointBuffer {
    position: vec4<f32>;
};

struct MatrixBuffer {
    matrices: array<mat4x4<f32>>;
};

struct CurveBuffer {
    positions: array<vec4<f32>>;
};

[[group(0), binding(0)]] var<storage, read> in_point: PointBuffer;
[[group(0), binding(1)]] var<storage, read> in_matrices: MatrixBuffer;
[[group(0), binding(2)]] var<storage, read_write> output: CurveBuffer;

[[stage(compute), workgroup_size(16)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
    let index = global_id.x;
    output.positions[index] = in_matrices.matrices[index] * in_point.position;
}
"##.to_string();

    // A point has a fixed size
    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Vec4>() * matrix_param.n_points());
    let bind_info = vec![
        BindInfo {
            buffer: geom_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: matrix_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &output_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [matrix_param.segments, 1, 1],
    };
    let new_data = Data::Geom1D {
        buffer: output_buffer,
        param: matrix_param.clone(),
    };

    Ok((new_data, operation))
}

fn t_1d_1d(
    device: &wgpu::Device,
    geom_buffer: &wgpu::Buffer,
    geom_param: &Parameter,
    matrix_buffer: &wgpu::Buffer,
    ) -> SingleDataResult {
    let wgsl_source = r##"
struct CurveBuffer {
    positions: array<vec4<f32>>;
};

struct MatrixBuffer {
    matrix: mat4x4<f32>;
};

[[group(0), binding(0)]] var<storage, read> in_curve: CurveBuffer;
[[group(0), binding(1)]] var<storage, read> in_matrix: MatrixBuffer;
[[group(0), binding(2)]] var<storage, read_write> output: CurveBuffer;

[[stage(compute), workgroup_size(16)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
    let index = global_id.x;
    output.positions[index] = in_matrix.matrix * in_curve.positions[index];
}
"##.to_string();

    // A point has a fixed size
    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Vec4>() * geom_param.n_points());
    let bind_info = vec![
        BindInfo {
            buffer: geom_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: matrix_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &output_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [geom_param.segments, 1, 1],
    };
    let new_data = Data::Geom1D {
        buffer: output_buffer,
        param: geom_param.clone(),
    };

    Ok((new_data, operation))
}

fn t_1d_same_param(
    device: &wgpu::Device,
    geom_buffer: &wgpu::Buffer,
    matrix_buffer: &wgpu::Buffer,
    param: &Parameter,
    ) -> SingleDataResult {
    let wgsl_source = r##"
struct CurveBuffer {
    positions: array<vec4<f32>>;
};

struct MatrixBuffer {
    matrices: array<mat4x4<f32>>;
};

[[group(0), binding(0)]] var<storage, read> in_curve: CurveBuffer;
[[group(0), binding(1)]] var<storage, read> in_matrices: MatrixBuffer;
[[group(0), binding(2)]] var<storage, read_write> output: CurveBuffer;

[[stage(compute), workgroup_size(16)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
    let index = global_id.x;
    output.positions[index] = in_matrices.matrices[index] * in_curve.positions[index];
}
"##.to_string();

    // A point has a fixed size
    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Vec4>() * param.n_points());
    let bind_info = vec![
        BindInfo {
            buffer: geom_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: matrix_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &output_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [param.segments, 1, 1],
    };
    let new_data = Data::Geom1D {
        buffer: output_buffer,
        param: param.clone(),
    };

    Ok((new_data, operation))
}

fn t_1d_up_2d(
    device: &wgpu::Device,
    geom_buffer: &wgpu::Buffer,
    geom_param: &Parameter,
    matrix_buffer: &wgpu::Buffer,
    matrix_param: &Parameter,
    ) -> SingleDataResult {
    let wgsl_source = r##"
struct CurveBuffer {
    positions: array<vec4<f32>>;
};

struct MatrixBuffer {
    matrix: array<mat4x4<f32>>;
};

struct SurfaceBuffer {
    positions: array<vec4<f32>>;
};

[[group(0), binding(0)]] var<storage, read> in_curve: CurveBuffer;
[[group(0), binding(1)]] var<storage, read> in_matrices: MatrixBuffer;
[[group(0), binding(2)]] var<storage, read_write> output: SurfaceBuffer;

[[stage(compute), workgroup_size(16, 16)]]
fn main(
    [[builtin(global_invocation_id)]] global_id: vec3<u32>,
    [[builtin(num_workgroups)]] num_groups: vec3<u32>,
) {
    let par1_idx = global_id.x;
    let par2_idx = global_id.y;
    let index = par1_idx + num_groups.x * 16u * par2_idx;
    output.positions[index] = in_matrices.matrix[par2_idx] * in_curve.positions[par1_idx];
}
"##.to_string();

    // A point has a fixed size
    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Vec4>() * geom_param.n_points() * matrix_param.n_points());
    let bind_info = vec![
        BindInfo {
            buffer: geom_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: matrix_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &output_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [geom_param.segments, matrix_param.segments, 1],
    };
    let new_data = Data::Geom2D {
        buffer: output_buffer,
        param1: geom_param.clone(),
        param2: matrix_param.clone(),
    };

    Ok((new_data, operation))
}

fn t_2d_2d(
    device: &wgpu::Device,
    geom_buffer: &wgpu::Buffer,
    geom_param1: &Parameter,
    geom_param2: &Parameter,
    matrix_buffer: &wgpu::Buffer,
    ) -> SingleDataResult {
    let wgsl_source = r##"
struct SurfaceBuffer {
    positions: array<vec4<f32>>;
};

struct MatrixBuffer {
    matrix: mat4x4<f32>;
};

[[group(0), binding(0)]] var<storage, read> in_surface: SurfaceBuffer;
[[group(0), binding(1)]] var<storage, read> in_matrix: MatrixBuffer;
[[group(0), binding(2)]] var<storage, read_write> output: SurfaceBuffer;

[[stage(compute), workgroup_size(16, 16)]]
fn main(
    [[builtin(global_invocation_id)]] global_id: vec3<u32>,
    [[builtin(num_workgroups)]] num_groups: vec3<u32>,
) {
    let par1_idx = global_id.x;
    let par2_idx = global_id.y;
    let index = par1_idx + num_groups.x * 16u * par2_idx;
    output.positions[index] = in_matrix.matrix * in_surface.positions[index];
}
"##.to_string();

    // A point has a fixed size
    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Vec4>() * geom_param1.n_points() * geom_param2.n_points());
    let bind_info = vec![
        BindInfo {
            buffer: geom_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: matrix_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &output_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [geom_param1.segments, geom_param2.segments, 1],
    };
    let new_data = Data::Geom2D {
        buffer: output_buffer,
        param1: geom_param1.clone(),
        param2: geom_param2.clone(),
    };

    Ok((new_data, operation))
}

fn t_2d_same_param(
    device: &wgpu::Device,
    geom_buffer: &wgpu::Buffer,
    geom_param1: &Parameter,
    geom_param2: &Parameter,
    matrix_buffer: &wgpu::Buffer,
    matrix_param: &Parameter,
    ) -> SingleDataResult {

    let which_idx = if geom_param1.is_equal(matrix_param)? {
        "par1_idx"
    } else {
        "par2_idx"
    };
    let wgsl_source = r##"
struct SurfaceBuffer {
    positions: array<vec4<f32>>;
};

struct MatrixBuffer {
    matrices: array<mat4x4<f32>>;
};

[[group(0), binding(0)]] var<storage, read> in_surface: SurfaceBuffer;
[[group(0), binding(1)]] var<storage, read> in_matrices: MatrixBuffer;
[[group(0), binding(2)]] var<storage, read_write> output: SurfaceBuffer;

[[stage(compute), workgroup_size(16, 16)]]
fn main(
    [[builtin(global_invocation_id)]] global_id: vec3<u32>,
    [[builtin(num_workgroups)]] num_groups: vec3<u32>,
) {
    let par1_idx = global_id.x;
    let par2_idx = global_id.y;
    let index = par1_idx + num_groups.x * 16u * par2_idx;
    output.positions[index] = in_matrices.matrices["##.to_string() + which_idx + r##"] * in_surface.positions[index];
}
"##;

    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Vec4>() * geom_param1.n_points() * geom_param2.n_points());
    let bind_info = vec![
        BindInfo {
            buffer: geom_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: matrix_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &output_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [geom_param1.segments, geom_param2.segments, 1],
    };
    let new_data = Data::Geom2D {
        buffer: output_buffer,
        param1: geom_param1.clone(),
        param2: geom_param2.clone(),
    };

    Ok((new_data, operation))
}


fn t_prefab (device: &wgpu::Device,
    vertex_buffer: &wgpu::Buffer,
    chunks_count: usize,
    index_buffer: &Rc<wgpu::Buffer>,
    index_count: u32,
    matrix_buffer: &wgpu::Buffer,
    ) -> SingleDataResult {

    let wgsl_source = format!(r##"
struct MatcapVertex {{
    position: vec4<f32>;
    normal: vec4<f32>;
    uv_coords: vec2<f32>;
    padding: vec2<f32>;
}};

struct PrefabBuffer {{
    vertices: array<MatcapVertex>;
}};

struct MatrixBuffer {{
    matrix: mat4x4<f32>;
}};

[[group(0), binding(0)]] var<storage, read> in_prefab: PrefabBuffer;
[[group(0), binding(1)]] var<storage, read> in_matrix: MatrixBuffer;
[[group(0), binding(2)]] var<storage, read_write> output: PrefabBuffer;

[[stage(compute), workgroup_size({vertices_per_chunk})]]
fn main(
    [[builtin(global_invocation_id)]] global_id: vec3<u32>,
) {{
    let index = global_id.x;

    // in 3d->3d we operate on both the positions AND the normals.
    // positions are simply multiplied by the matrix, while
    // for normals, we extract the linear part of the transform
    // and premultiply the normals by the inverse transpose,
    // PROVIDED THE MATRIX IS NOT SINGULAR
    // TODO: possible optimization: precompute inverse transpose
    // directly in the matrix compute block (for 0D matrices only)
    let A = mat3x3<f32>(
        in_matrix.matrix[0].xyz,
        in_matrix.matrix[1].xyz,
        in_matrix.matrix[2].xyz,
    );
    let det: f32 = determinant(A);
    if (det > 1e-6) {{
        // WGSL does not provide any "inverse transpose" function, so compute it by hand
        var inv_t: mat3x3<f32>;
        let invdet = 1.0/det;
        inv_t[0][0] =  (A[1][1] * A[2][2] - A[2][1] * A[1][2]) * invdet;
        inv_t[1][0] = -(A[0][1] * A[2][2] - A[0][2] * A[2][1]) * invdet;
        inv_t[2][0] =  (A[0][1] * A[1][2] - A[0][2] * A[1][1]) * invdet;
        inv_t[0][1] = -(A[1][0] * A[2][2] - A[1][2] * A[2][0]) * invdet;
        inv_t[1][1] =  (A[0][0] * A[2][2] - A[0][2] * A[2][0]) * invdet;
        inv_t[2][1] = -(A[0][0] * A[1][2] - A[1][0] * A[0][2]) * invdet;
        inv_t[0][2] =  (A[1][0] * A[2][1] - A[2][0] * A[1][1]) * invdet;
        inv_t[1][2] = -(A[0][0] * A[2][1] - A[2][0] * A[0][1]) * invdet;
        inv_t[2][2] =  (A[0][0] * A[1][1] - A[1][0] * A[0][1]) * invdet;

        let transformed_normal: vec3<f32> = normalize(inv_t * in_prefab.vertices[index].normal.xyz);
        output.vertices[index].normal = vec4<f32>(transformed_normal, 0.0);
    }} else {{
        // this is wrong, but at least it won't produce undefined garbage results
        output.vertices[index].normal = in_prefab.vertices[index].normal;
    }}
    output.vertices[index].position = in_matrix.matrix * in_prefab.vertices[index].position;
    output.vertices[index].uv_coords = in_prefab.vertices[index].uv_coords;
    output.vertices[index].padding = in_prefab.vertices[index].padding;
}}
"##, vertices_per_chunk=MODEL_CHUNK_VERTICES,);

    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<StandardVertexData>() * chunks_count * MODEL_CHUNK_VERTICES);
    let bind_info = vec![
        BindInfo {
            buffer: vertex_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: matrix_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &output_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [chunks_count as u32, 1, 1],
    };
    let new_data = Data::Prefab {
        vertex_buffer: output_buffer,
        chunks_count,
        index_buffer: Rc::clone(index_buffer),
        index_count,
    };

    Ok((new_data, operation))
}

