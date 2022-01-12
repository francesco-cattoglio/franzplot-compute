use std::collections::BTreeMap;
use std::rc::Rc;
use super::Operation;
use super::{SingleDataResult, ProcessingError};
use super::{DataID, Data};
use super::Parameter;
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};

const CHUNK_SIZE: usize = super::Parameter::POINTS_PER_SEGMENT;

pub fn create(
    device: &wgpu::Device,
    data_map: &BTreeMap<DataID, Data>,
    control_points_ids: Vec<DataID>,
    quality: usize,
) -> SingleDataResult {
    if !(1..=16).contains(&quality) {
        return Err(ProcessingError::IncorrectAttributes("Interval quality attribute must be an integer in the [1, 16] range".into()))
    }

    let param = super::Parameter {
        name: None,
        begin: "0.0".into(),
        end: "1.0".into(),
        segments: quality as u32,
        use_interval_as_uv: false,
    };
    match control_points_ids.len() {
        0..=1 => Err(ProcessingError::InputMissing(" A Bezier curve requires \n at least 2 points ".into())),
        2 => new_bezier_1st_degree(device, data_map, control_points_ids, param),
        3 => new_bezier_2nd_degree(device, data_map, control_points_ids, param),
        4 => new_bezier_3rd_degree(device, data_map, control_points_ids, param),
        _ => Err(ProcessingError::InternalError("Currently we only support Bézier curves up to degree 3".into())),
    }
}

fn new_bezier_1st_degree(
    device: &wgpu::Device,
    data_map: &BTreeMap<DataID, Data>,
    control_points_ids: Vec<DataID>,
    param: Parameter,
) -> SingleDataResult {

    let p0_buff = get_point_buffer(data_map, control_points_ids[0])?;
    let p1_buff = get_point_buffer(data_map, control_points_ids[1])?;

    let wgsl_source = format!(r##"
struct PointBuffer {{
    position: vec4<f32>;
}};

struct OutputBuffer {{
    positions: array<vec4<f32>>;
}};

[[group(0), binding(0)]] var<storage, read> p0: PointBuffer;
[[group(0), binding(1)]] var<storage, read> p1: PointBuffer;
[[group(0), binding(2)]] var<storage, read_write> output: OutputBuffer;

[[stage(compute), workgroup_size({CHUNK_SIZE})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    let index = global_id.x;
    let t = f32(index) / (f32({n_points}) - 1.0);
    output.positions[index] = (1.0 - t) * p0.position + t * p1.position;
}}
"##, CHUNK_SIZE=CHUNK_SIZE, n_points=param.n_points()
);

    //println!("bezier two points:\n {}", &wgsl_source);

    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Vec4>() * param.n_points());

    let bind_info = vec![
        BindInfo {
            buffer: p0_buff,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: p1_buff,
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
        param,
    };

    Ok((new_data, operation))
}

fn new_bezier_2nd_degree(
    device: &wgpu::Device,
    data_map: &BTreeMap<DataID, Data>,
    control_points_ids: Vec<DataID>,
    param: Parameter,
) -> SingleDataResult {
    let p0_buff = get_point_buffer(data_map, control_points_ids[0])?;
    let p1_buff = get_point_buffer(data_map, control_points_ids[1])?;
    let p2_buff = get_point_buffer(data_map, control_points_ids[2])?;

    let wgsl_source = format!(r##"
struct PointBuffer {{
    position: vec4<f32>;
}};

struct OutputBuffer {{
    positions: array<vec4<f32>>;
}};

[[group(0), binding(0)]] var<storage, read> p0: PointBuffer;
[[group(0), binding(1)]] var<storage, read> p1: PointBuffer;
[[group(0), binding(2)]] var<storage, read> p2: PointBuffer;
[[group(0), binding(3)]] var<storage, read_write> output: OutputBuffer;

[[stage(compute), workgroup_size({CHUNK_SIZE})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    let index = global_id.x;
    let t = f32(index) / (f32({n_points}) - 1.0);
    output.positions[index] =
                              (1.0 - t) * (1.0 - t) * p0.position
                      + 2.0 * (1.0 - t) *     t     * p1.position
                      +           t     *     t     * p2.position;
}}
"##, CHUNK_SIZE=CHUNK_SIZE, n_points=param.n_points()
);

    //println!("bezier three points:\n {}", &wgsl_source);

    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Vec4>() * param.n_points());

    let bind_info = vec![
        BindInfo {
            buffer: p0_buff,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: p1_buff,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: p2_buff,
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
        param,
    };

    Ok((new_data, operation))
}

fn new_bezier_3rd_degree(
    device: &wgpu::Device,
    data_map: &BTreeMap<DataID, Data>,
    control_points_ids: Vec<DataID>,
    param: Parameter,
) -> SingleDataResult {
    let p0_buff = get_point_buffer(data_map, control_points_ids[0])?;
    let p1_buff = get_point_buffer(data_map, control_points_ids[1])?;
    let p2_buff = get_point_buffer(data_map, control_points_ids[2])?;
    let p3_buff = get_point_buffer(data_map, control_points_ids[3])?;

    let wgsl_source = format!(r##"
struct PointBuffer {{
    position: vec4<f32>;
}};

struct OutputBuffer {{
    positions: array<vec4<f32>>;
}};

[[group(0), binding(0)]] var<storage, read> p0: PointBuffer;
[[group(0), binding(1)]] var<storage, read> p1: PointBuffer;
[[group(0), binding(2)]] var<storage, read> p2: PointBuffer;
[[group(0), binding(3)]] var<storage, read> p3: PointBuffer;
[[group(0), binding(4)]] var<storage, read_write> output: OutputBuffer;

[[stage(compute), workgroup_size({CHUNK_SIZE})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    let index = global_id.x;
    let t = f32(index) / (f32({n_points}) - 1.0);
    output.positions[index] =
                              (1.0 - t) * (1.0 - t) * (1.0 - t) * p0.position
                      + 3.0 * (1.0 - t) * (1.0 - t) *     t     * p1.position
                      + 3.0 * (1.0 - t) *     t     *     t     * p2.position
                      +           t     *     t     *     t     * p3.position;
}}
"##, CHUNK_SIZE=CHUNK_SIZE, n_points=param.n_points()
);

    //println!("bezier four points:\n {}", &wgsl_source);

    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Vec4>() * param.n_points());

    let bind_info = vec![
        BindInfo {
            buffer: p0_buff,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: p1_buff,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: p2_buff,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: p3_buff,
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
        param,
    };

    Ok((new_data, operation))
}

fn get_point_buffer(data_map: &BTreeMap<DataID, Data>, id: DataID) -> Result<&wgpu::Buffer, ProcessingError> {
    let found_element = data_map
        .get(&id)
        .ok_or(ProcessingError::NoInputData)?;
    match found_element {
        Data::Geom0D{ buffer } => Ok(buffer),
        _ => Err(ProcessingError::IncorrectInput(" the input provided to Bezier \n is not a Point ".into()))
    }
}

