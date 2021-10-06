use std::collections::BTreeMap;
use super::Operation;
use crate::computable_scene::globals::Globals;
use super::{ProcessingResult, ProcessingError};
use super::Parameter;
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
    output_id: DataID,
) -> ProcessingResult {
    println!("new curve processing");
    let data_id = interval_id.ok_or(ProcessingError::InputMissing(" This Curve node \n is missing its input "))?;
    let found_data = data_map.get(&data_id).ok_or(ProcessingError::InternalError("Interval used as input does not exist in the block map".into()))?;

    let (input_buffer, param) = match found_data {
        Data::Interval{
            buffer, param
        } => (buffer, param),
        _ => return Err(ProcessingError::InternalError("the input provided to the Curve is not an Interval".into()))
    };

    let param_name = param.name.clone().unwrap();

    // Sanitize all input expressions
    let local_params = vec![param_name.as_str()];
    let sanitized_fx = globals.sanitize_expression_2(&local_params, &fx)?;
    let sanitized_fy = globals.sanitize_expression_2(&local_params, &fy)?;
    let sanitized_fz = globals.sanitize_expression_2(&local_params, &fz)?;

    let wgsl_source = format!(r##"
{wgsl_header}

[[block]] struct InputBuffer {{
values: array<f32>;
}};

[[block]] struct OutputBuffer {{
positions: array<vec4<f32>>;
}};

[[group(0), binding(1)]] var<storage, read> _input: InputBuffer;
[[group(0), binding(2)]] var<storage, read_write> _output: OutputBuffer;

[[stage(compute), workgroup_size({n_points})]]
fn main([[builtin(global_invocation_id)]] _global_id: vec3<u32>) {{
let _index = _global_id.x;
let {par} = _input.values[_index];
let _fx = {fx};
let _fy = {fy};
let _fz = {fz};
_output.positions[_index] = vec4<f32>(_fx, _fy, _fz, 1.0);
}}
"##, wgsl_header=globals.get_wgsl_header(), par=param_name, fx=sanitized_fx, fy=sanitized_fy, fz=sanitized_fz, n_points=param.size
);

    println!("shader source:\n {}", &wgsl_source);

    // We are creating a curve from an interval, output vertex count is the same as interval
    // one, but buffer size is 4 times as much, because we are storing a Vec4 instead of a f32
    let output_buffer = util::create_storage_buffer(device, 4 * std::mem::size_of::<f32>() * param.size);

    let bind_info = vec![
        globals.get_bind_info(),
        BindInfo {
            buffer: &input_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: true },
        },
        BindInfo {
            buffer: &output_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let mut new_data = BTreeMap::<DataID, Data>::new();
    new_data.insert(
        output_id,
        Data::Geom1D {
            buffer: output_buffer,
            param: param.clone(),
        },
    );
    let operation = Operation {
        bind_group,
        pipeline,
        dim: [1, 1, 1],
    };

    Ok((new_data, operation))
}
