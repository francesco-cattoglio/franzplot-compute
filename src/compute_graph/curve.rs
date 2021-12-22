use std::collections::BTreeMap;
use std::rc::Rc;
use super::Operation;
use super::globals::Globals;
use super::{SingleDataResult, ProcessingError};
use super::{DataID, Data};
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};

pub fn create(
    device: &wgpu::Device,
    globals: &Globals,
    data_map: &BTreeMap<DataID, Data>,
    interval_id: Option<DataID>,
    fx: String,
    fy: String,
    fz: String,
) -> SingleDataResult {
    //println!("new curve processing");
    let data_id = interval_id
        .ok_or_else(|| ProcessingError::InputMissing(" This Curve node \n is missing its input ".into()))?;
    let found_data = data_map
        .get(&data_id)
        .ok_or(ProcessingError::NoInputData)?;

    let (input_buffer, param) = match found_data {
        Data::Interval {
            buffer, param
        } => (buffer, param.clone()),
        _ => return Err(ProcessingError::InternalError("the input provided to the Curve is not an Interval".into()))
    };

    let param_name = param.name.as_ref().unwrap();

    // Sanitize all input expressions
    let local_params = vec![param_name.as_str()];
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

[[group(0), binding(1)]] var<storage, read> input: InputBuffer;
[[group(0), binding(2)]] var<storage, read_write> output: OutputBuffer;

[[stage(compute), workgroup_size({n_points})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    let index = global_id.x;
    let {par} = input.values[index];
    let fx = {fx};
    let fy = {fy};
    let fz = {fz};
    output.positions[index] = vec4<f32>(fx, fy, fz, 1.0);
}}
"##, wgsl_header=globals.get_wgsl_header(), par=param_name, fx=sanitized_fx, fy=sanitized_fy, fz=sanitized_fz, n_points=param.n_points()
);

    //println!("shader source:\n {}", &wgsl_source);

    // We are creating a curve from an interval, output vertex count is the same as interval
    // one, but buffer size is 4 times as much, because we are storing a Vec4 instead of a f32
    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Vec4>() * param.n_points());

    let bind_info = vec![
        globals.get_bind_info(),
        BindInfo {
            buffer: input_buffer,
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
    let new_data = Data::Geom1D {
        buffer: output_buffer,
        param: param.clone(),
    };

    Ok((new_data, operation))
}
