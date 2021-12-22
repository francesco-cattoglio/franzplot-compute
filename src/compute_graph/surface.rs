use std::collections::BTreeMap;
use std::rc::Rc;
use super::Operation;
use super::globals::Globals;
use super::{SingleDataResult, ProcessingError};
use super::Parameter;
use super::{DataID, Data};
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};

#[allow(clippy::too_many_arguments)]
pub fn create(
    device: &wgpu::Device,
    globals: &Globals,
    data_map: &BTreeMap<DataID, Data>,
    interval_1: Option<DataID>,
    interval_2: Option<DataID>,
    fx: String,
    fy: String,
    fz: String,
) -> SingleDataResult {
    //println!("new curve processing");
    let interval_1_id = interval_1
        .ok_or_else(|| ProcessingError::InputMissing(" This Surface node \n is missing its first input ".into()))?;
    let interval_2_id = interval_2
        .ok_or_else(|| ProcessingError::InputMissing(" This Surface node \n is missing its second input ".into()))?;
    let interval_1_data = data_map.get(&interval_1_id).ok_or(ProcessingError::NoInputData)?;
    let interval_2_data = data_map.get(&interval_2_id).ok_or(ProcessingError::NoInputData)?;

    let (buffer_1, param_1) = match interval_1_data {
        Data::Interval{
            buffer, param
        } => (buffer, param.clone()),
        _ => return Err(ProcessingError::InternalError("the first input provided to the Surface node is not an Interval".into()))
    };

    let (buffer_2, param_2) = match interval_2_data {
        Data::Interval{
            buffer, param
        } => (buffer, param.clone()),
        _ => return Err(ProcessingError::InternalError("the first input provided to the Surface node is not an Interval".into()))
    };

    let param_1_name = param_1.name.as_ref().unwrap();
    let param_2_name = param_2.name.as_ref().unwrap();

    // Sanitize all input expressions
    let local_params = vec![param_1_name.as_str(), param_2_name.as_str()];
    let sanitized_fx = globals.sanitize_expression(&local_params, &fx)?;
    let sanitized_fy = globals.sanitize_expression(&local_params, &fy)?;
    let sanitized_fz = globals.sanitize_expression(&local_params, &fz)?;

    let wgsl_source = format!(r##"
{wgsl_header}

[[block]] struct InputBuffer {{
    values: array<f32>;
}};

[[block]] struct OutputBuffer {{
    positions: array<vec4<f32>>;
}};

[[group(0), binding(1)]] var<storage, read> interval_1: InputBuffer;
[[group(0), binding(2)]] var<storage, read> interval_2: InputBuffer;
[[group(0), binding(3)]] var<storage, read_write> output: OutputBuffer;

[[stage(compute), workgroup_size({pps}, {pps})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    let par1_idx = global_id.x;
    let par2_idx = global_id.y;
    let index = par1_idx + {size_x}u * par2_idx;

    let {par1} = interval_1.values[par1_idx];
    let {par2} = interval_2.values[par2_idx];
    let fx = {fx};
    let fy = {fy};
    let fz = {fz};
    output.positions[index] = vec4<f32>(fx, fy, fz, 1.0);
}}
"##, wgsl_header=globals.get_wgsl_header(), pps=Parameter::POINTS_PER_SEGMENT,
par1=param_1_name, par2=param_2_name,
fx=sanitized_fx, fy=sanitized_fy, fz=sanitized_fz, size_x=param_1.n_points()
);

    println!("surface shader source:\n {}", &wgsl_source);

    // We are creating a curve from an interval, output vertex count is the same as interval
    // one, but buffer size is 4 times as much, because we are storing a Vec4 instead of a f32
    let output_buffer = util::create_storage_buffer(device, 4 * std::mem::size_of::<f32>() * param_1.n_points() * param_2.n_points());

    let bind_info = vec![
        globals.get_bind_info(),
        BindInfo {
            buffer: buffer_1,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: buffer_2,
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
        dim: [param_1.segments, param_2.segments, 1],
    };
    let new_data = Data::Geom2D {
        param1: param_1,
        param2: param_2,
        buffer: output_buffer,
    };

    Ok((new_data, operation))
}

