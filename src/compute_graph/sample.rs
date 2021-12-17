use std::collections::BTreeMap;
use std::rc::Rc;

use super::Operation;
use super::Parameter;
use crate::computable_scene::globals::Globals;
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
    globals: &Globals,
    data_map: &BTreeMap<DataID, Data>,
    geometry_id: Option<DataID>,
    parameter_name: String,
    sample_value: String,
) -> SingleDataResult {
    let data_id = geometry_id.ok_or(ProcessingError::InputMissing(" This Sample node \n is missing its Geometry input "))?;
    let geometry_data = data_map.get(&data_id).ok_or(ProcessingError::NoInputData)?;

    match &geometry_data {
        Data::Geom0D{..}
            => Err(ProcessingError::IncorrectInput(" cannot sample from \n a point (0d geometry) ")),

        Data::Geom1D{buffer, param}
            => sample_1d_0d(device, globals, buffer, param, &parameter_name, &sample_value),

        Data::Geom2D{buffer, param1, param2}
            => sample_2d_1d(device, globals, buffer, param1, param2, &parameter_name, &sample_value),

        Data::Prefab { .. }
            => Err(ProcessingError::IncorrectInput(" cannot sample from \n a primitive ")),

        _ => Err(ProcessingError::InternalError("unhandled sample case".into()))
    }

}

fn sample_1d_0d(
    device: &wgpu::Device,
    globals: &Globals,
    geom_buffer: &wgpu::Buffer,
    geom_param: &Parameter,
    parameter_name: &str,
    sample_value: &str,
    ) -> SingleDataResult {

    // Sanitize all input expressions
    let sanitized_name = Globals::sanitize_variable_name_2(parameter_name)?;
    let sanitized_value = globals.sanitize_expression_2(&[], sample_value)?;

    let maybe_curve_param_name = geom_param.name.as_ref();
    if let Some(name) = maybe_curve_param_name {
        // if the name does not match the one from the parameter, error out
        if name != &sanitized_name {
            return Err(ProcessingError::IncorrectAttributes(" the parameter used \n is not known "));
        }
    } else {
        // if the geometry parameter does not exist, error our as well.
        // TODO: we might want to change this, so that one can sample a Bezier curve
        return Err(ProcessingError::IncorrectAttributes(" the parameter used \n is not known "));
    }

    // first, we need to check if the parameter name corresponds with the interval one.
    let wgsl_source = format!(r##"
{wgsl_header}

[[block]] struct CurveBuffer {{
    positions: array<vec4<f32>>;
}};

[[block]] struct PointBuffer {{
    position: vec4<f32>;
}};

// binding 0 used by global vars, as usual
[[group(0), binding(1)]] var<storage, read> in_curve: CurveBuffer;
[[group(0), binding(2)]] var<storage, read_write> output: PointBuffer;

[[stage(compute), workgroup_size(1)]]
fn main() {{
    // parameter space is linear, so we can figure out which index we should access
    let size = f32({array_size});
    let interval_begin: f32 = {begin};
    let interval_end: f32 = {end};
    // transform the interval so that it extends from 0 to size-1, and scale the sampling value accordingly
    let value = ({sample_value} - interval_begin) * (size - 1.0) / (interval_end - interval_begin);
    // compute the indices to use in the interpolation and interpolation weight
    let inf_value = floor(value);
    let sup_value = ceil(value);
    let alpha = fract(value);
    // clamp index acces to make sure nothing bad happens,
    // even if the provided value was outside of parameter interval
    let inf_idx = i32(clamp(inf_value, 0.0, size - 1.0));
    let sup_idx = i32(clamp(sup_value, 0.0, size - 1.0));
    output.position = (1.0 - alpha) * in_curve.positions[inf_idx] + alpha * in_curve.positions[sup_idx];
}}
"##, wgsl_header=globals.get_wgsl_header(), begin=&geom_param.begin, end=&geom_param.end,
    sample_value=sanitized_value, array_size=CHUNK_SIZE*geom_param.segments as usize);

    //println!("sample 1d->0d shader source:\n {}", &wgsl_source);

    // A point has a fixed size
    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Vec4>());
    let bind_info = vec![
        globals.get_bind_info(),
        BindInfo {
            buffer: geom_buffer,
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

fn t_1d_1d(
    device: &wgpu::Device,
    geom_buffer: &wgpu::Buffer,
    geom_param: &Parameter,
    matrix_buffer: &wgpu::Buffer,
    ) -> SingleDataResult {
    let wgsl_source = r##"
[[block]] struct CurveBuffer {
    positions: array<vec4<f32>>;
};

[[block]] struct MatrixBuffer {
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
[[block]] struct CurveBuffer {
    positions: array<vec4<f32>>;
};

[[block]] struct MatrixBuffer {
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
[[block]] struct CurveBuffer {
    positions: array<vec4<f32>>;
};

[[block]] struct MatrixBuffer {
    matrix: array<mat4x4<f32>>;
};

[[block]] struct SurfaceBuffer {
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
[[block]] struct SurfaceBuffer {
    positions: array<vec4<f32>>;
};

[[block]] struct MatrixBuffer {
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
[[block]] struct SurfaceBuffer {
    positions: array<vec4<f32>>;
};

[[block]] struct MatrixBuffer {
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

[[block]] struct PrefabBuffer {{
    vertices: array<MatcapVertex>;
}};

[[block]] struct MatrixBuffer {{
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

fn sample_2d_1d(
    device: &wgpu::Device,
    globals: &Globals,
    geom_buffer: &wgpu::Buffer,
    geom_param1: &Parameter,
    geom_param2: &Parameter,
    parameter_name: &str,
    sample_value: &str,
    ) -> SingleDataResult {

    todo!()
    }
