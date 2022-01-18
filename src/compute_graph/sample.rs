use std::collections::BTreeMap;
use std::rc::Rc;

use super::Operation;
use super::Parameter;
use super::globals::Globals;
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
    let data_id = geometry_id
        .ok_or_else(|| ProcessingError::InputMissing(" This Sample node \n is missing its Geometry input ".into()))?;
    let geometry_data = data_map
        .get(&data_id)
        .ok_or(ProcessingError::NoInputData)?;

    match &geometry_data {
        Data::Geom0D{..}
            => Err(ProcessingError::IncorrectInput(" cannot sample from \n a point (0d geometry) ".into())),

        Data::Geom1D{buffer, param}
            => sample_1d_0d(device, globals, buffer, param, &parameter_name, &sample_value),

        Data::Geom2D{buffer, param1, param2}
            => sample_2d_1d(device, globals, buffer, param1, param2, &parameter_name, &sample_value),

        Data::Prefab { .. }
            => Err(ProcessingError::IncorrectInput(" cannot sample from \n a primitive ".into())),

        _ => Err(ProcessingError::InternalError(" input provided to sample \n is not a geometry ".into()))
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
    let sanitized_name = Globals::sanitize_variable_name(parameter_name)?;
    let sanitized_value = globals.sanitize_expression(&[], sample_value)?;

    // first, we need to check if the parameter name corresponds with the interval one.
    let maybe_curve_param_name = geom_param.name.as_ref();
    if let Some(name) = maybe_curve_param_name {
        // if the name does not match the one from the parameter, error out
        if name != &sanitized_name {
            return Err(ProcessingError::IncorrectAttributes(" the parameter used \n is not known ".into()));
        }
    } else {
        // if the geometry parameter does not exist, error our as well.
        // TODO: we might want to change this, so that one can sample a Bezier curve
        return Err(ProcessingError::IncorrectAttributes(" the parameter used \n is not known ".into()));
    }

    let wgsl_source = format!(r##"
{wgsl_header}

struct CurveBuffer {{
    positions: array<vec4<f32>>;
}};

struct PointBuffer {{
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
    sample_value=sanitized_value, array_size=geom_param.n_points());

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

fn sample_2d_1d(
    device: &wgpu::Device,
    globals: &Globals,
    geom_buffer: &wgpu::Buffer,
    geom_param1: &Parameter,
    geom_param2: &Parameter,
    parameter_name: &str,
    sample_value: &str,
    ) -> SingleDataResult {

    // Sanitize all input expressions
    let sanitized_name = Globals::sanitize_variable_name(parameter_name)?;
    let sanitized_value = globals.sanitize_expression(&[], sample_value)?;

    let maybe_curve_param1_name = geom_param1.name.as_ref();
    let maybe_curve_param2_name = geom_param2.name.as_ref();
    let (which_param, _which_name) = match (maybe_curve_param1_name, maybe_curve_param2_name) {
        (Some(name), _) if name == &sanitized_name
            => { (1, name) },
        (_, Some(name)) if name == &sanitized_name
            => { (2, name) },
        _ => return Err(ProcessingError::IncorrectAttributes(" the parameter used \n is not known ".into())),
    };

    // the shader will be slightly different depending on which param is the one being sampled
    let (sampled_param, nonsampled_param) = if which_param == 1 {
        (geom_param1.clone(), geom_param2.clone())
    } else {
        (geom_param2.clone(), geom_param1.clone())
    };

    let wgsl_source = format!(r##"
{wgsl_header}

struct SurfaceBuffer {{
    positions: array<vec4<f32>>;
}};

struct CurveBuffer {{
    positions: array<vec4<f32>>;
}};

// binding 0 used by global vars, as usual
[[group(0), binding(1)]] var<storage, read> in_surface: SurfaceBuffer;
[[group(0), binding(2)]] var<storage, read_write> output: CurveBuffer;

[[stage(compute), workgroup_size({CHUNK_SIZE})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    // parameter space is linear, so we can figure out which index we should access
    let size = f32({sampled_array_size});
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
    let inf_idx = u32(clamp(inf_value, 0.0, size - 1.0));
    let sup_idx = u32(clamp(sup_value, 0.0, size - 1.0));

    // we now have to compute the two indices differently, depending on the parameter
    var inf_index: u32;
    var sup_index: u32;
    if ({sampling_first_param}) {{
        // if the param being sampled is the first one
        inf_index = inf_idx + {first_array_size}u * global_id.x;
        sup_index = sup_idx + {first_array_size}u * global_id.x;
    }} else {{
        // if the param being sampled is the second one
        inf_index = global_id.x + {first_array_size}u * inf_idx;
        sup_index = global_id.x + {first_array_size}u * sup_idx;
    }}
    output.positions[global_id.x] = (1.0 - alpha) * in_surface.positions[inf_index] + alpha * in_surface.positions[sup_index];
}}
"##, wgsl_header=&globals.get_wgsl_header(), sampled_array_size=sampled_param.n_points(),
first_array_size=geom_param1.n_points(),
sampling_first_param= which_param==1, CHUNK_SIZE=CHUNK_SIZE,
begin=&sampled_param.begin, end=&sampled_param.end, sample_value=sanitized_value);

    //println!("sample 2d->1d shader source:\n {}", &wgsl_source);
    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Vec4>() * nonsampled_param.n_points());
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
        dim: [nonsampled_param.segments, 1, 1],
    };
    let new_data = Data::Geom1D {
        buffer: output_buffer,
        param: nonsampled_param,
    };

    Ok((new_data, operation))
}
