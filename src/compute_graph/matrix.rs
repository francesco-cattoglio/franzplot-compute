use std::collections::BTreeMap;
use std::rc::Rc;

use super::Operation;
use super::globals::Globals;
use super::{SingleDataResult, ProcessingError};
use super::{DataID, Data};
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};

use crate::node_graph::Axis;

pub fn create_from_rotation(
    device: &wgpu::Device,
    globals: &Globals,
    data_map: &BTreeMap<DataID, Data>,
    axis: Axis,
    angle: String) -> SingleDataResult {
    // we need to write down a different matrix depending on what rotation axis we have
    let (row_1, row_2, row_3);

    dbg!(&angle);
    // Sanitize all input expressions to get any error, but do not save the result
    // (otherwise we would be renaming all global vars, e.g: `pi`->`globals.pi`)
    let _ = globals.sanitize_expression(&[], &angle)?;
    match axis {
        Axis::X => {
            row_1 = ["1.0".into(),               "0.0".into(),                "0.0".into(), "0.0".into()];
            row_2 = ["0.0".into(), format!("cos({})", &angle), format!("-sin({})", &angle), "0.0".into()];
            row_3 = ["0.0".into(), format!("sin({})", &angle),  format!("cos({})", &angle), "0.0".into()];
        },
        Axis::Y => {
            row_1 = [ format!("cos({})", &angle), "0.0".into(), format!("sin({})", &angle), "0.0".into()];
            row_2 = [               "0.0".into(), "1.0".into(),               "0.0".into(), "0.0".into()];
            row_3 = [format!("-sin({})", &angle), "0.0".into(), format!("cos({})", &angle), "0.0".into()];
        },
        Axis::Z => {
            row_1 = [format!("cos({})", &angle), format!("-sin({})", &angle), "0.0".into(), "0.0".into()];
            row_2 = [format!("sin({})", &angle),  format!("cos({})", &angle), "0.0".into(), "0.0".into()];
            row_3 = [              "0.0".into(),                "0.0".into(), "1.0".into(), "0.0".into()];
        },
    }

    create_from_rows(
        device,
        globals,
        data_map,
        None,
        row_1,
        row_2,
        row_3,
    )
}

pub fn create_from_translation(
    device: &wgpu::Device,
    data_map: &BTreeMap<DataID, Data>,
    vector_id: Option<DataID>,
) -> SingleDataResult {
    let data_id = vector_id
        .ok_or_else(|| ProcessingError::InputMissing(" This Translation Matrix node \n is missing its input ".into()))?;
    let found_data = data_map
        .get(&data_id)
        .ok_or(ProcessingError::NoInputData)?;
    let vector_buffer = match found_data {
        Data::Vector { buffer } => buffer,
        _ => return Err(ProcessingError::IncorrectInput(" Translation Matrix first input \n is not a vector ".into()))
    };

    let wgsl_source = format!(r##"
@group(0) @binding(0) var<storage, read> in_translation: vec4<f32>;
@group(0) @binding(1) var<storage, read_write> out_matrix: mat4x4<f32>;

@compute @workgroup_size(1)
fn main() {{
    output.matrix = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(in_translation.xyz, 1.0),
    );
}}
"##,);

    //println!("translation matrix wgsl shader: {}", wgsl_source);
    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Mat4>());
    let bind_info = vec![
        BindInfo {
            buffer: vector_buffer,
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
    let new_data = Data::Matrix0D {
        buffer: output_buffer,
    };
    Ok((new_data, operation))
}

pub fn create_from_rows(
    device: &wgpu::Device,
    globals: &Globals,
    data_map: &BTreeMap<DataID, Data>,
    interval_id: Option<DataID>,
    row_1: [String; 4],
    row_2: [String; 4],
    row_3: [String; 4],
) -> SingleDataResult {
    // take the optional interval_id and turn it into an optional (Buffer, Param) tuple.
    // If the interval was empty, then we will have a None.
    // If there was some error during processing, we move the error outside the lambda
    // thanks to the transpose()? line: we have an Option<Result<...>>, we turn it into a
    // Result<Option<...>>, we unwrap via ?
    let optional_interval = interval_id
        .map(|id| {
            let found_data = data_map.get(&id).ok_or(ProcessingError::NoInputData)?;
            match found_data {
                Data::Interval {
                    buffer, param
                } => Ok((buffer, param.clone())),
                _ => Err(ProcessingError::InternalError("the input provided to the Matrix is not an Interval".into()))
            }
        }).transpose()?;

    let mut local_params = Vec::<&str>::new();
    if let Some((_buffer, ref param)) = optional_interval {
        local_params.push(param.name.as_ref().unwrap().as_str())
    }

    let sanitized_m11 = globals.sanitize_expression(&local_params, &row_1[0])?;
    let sanitized_m12 = globals.sanitize_expression(&local_params, &row_1[1])?;
    let sanitized_m13 = globals.sanitize_expression(&local_params, &row_1[2])?;
    let sanitized_m14 = globals.sanitize_expression(&local_params, &row_1[3])?;
    let sanitized_m21 = globals.sanitize_expression(&local_params, &row_2[0])?;
    let sanitized_m22 = globals.sanitize_expression(&local_params, &row_2[1])?;
    let sanitized_m23 = globals.sanitize_expression(&local_params, &row_2[2])?;
    let sanitized_m24 = globals.sanitize_expression(&local_params, &row_2[3])?;
    let sanitized_m31 = globals.sanitize_expression(&local_params, &row_3[0])?;
    let sanitized_m32 = globals.sanitize_expression(&local_params, &row_3[1])?;
    let sanitized_m33 = globals.sanitize_expression(&local_params, &row_3[2])?;
    let sanitized_m34 = globals.sanitize_expression(&local_params, &row_3[3])?;

    if let Some((input_buffer, param)) = optional_interval {
        let wgsl_source = format!(r##"
{wgsl_header}

@group(0) @binding(1) var<storage, read> in_values: array<f32>;
@group(0) @binding(2) var<storage, read_write> out_matrices: array<mat4x4<f32>>;

@compute @workgroup_size(16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let index = global_id.x;
    let {par} = in_values[index];
    out_matrices[index] = mat4x4<f32>(
        vec4<f32>({_m11}, {_m21}, {_m31}, 0.0),
        vec4<f32>({_m12}, {_m22}, {_m32}, 0.0),
        vec4<f32>({_m13}, {_m23}, {_m33}, 0.0),
        vec4<f32>({_m14}, {_m24}, {_m34}, 1.0),
    );
}}
        "##, wgsl_header=globals.get_wgsl_header(), par=param.name.as_ref().unwrap(),
        _m11=sanitized_m11, _m12=sanitized_m12, _m13=sanitized_m13, _m14=sanitized_m14,
        _m21=sanitized_m21, _m22=sanitized_m22, _m23=sanitized_m23, _m24=sanitized_m24,
        _m31=sanitized_m31, _m32=sanitized_m32, _m33=sanitized_m33, _m34=sanitized_m34,
        );

        //println!("parametrix matrix wgsl shader: {}", wgsl_source);
        let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Mat4>() * param.n_points());
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
            dim: [param.segments, 1, 1],
        };
        let new_data = Data::Matrix1D {
            buffer: output_buffer,
            param,
        };
        Ok((new_data, operation))
    } else {
        let wgsl_source = format!(r##"
{wgsl_header}

@group(0) @binding(1) var<storage, read_write> out_matrix: mat4x4<f32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    out_matrix = mat4x4<f32>(
        vec4<f32>({_m11}, {_m21}, {_m31}, 0.0),
        vec4<f32>({_m12}, {_m22}, {_m32}, 0.0),
        vec4<f32>({_m13}, {_m23}, {_m33}, 0.0),
        vec4<f32>({_m14}, {_m24}, {_m34}, 1.0),
    );
}}
        "##, wgsl_header=globals.get_wgsl_header(),
        _m11=sanitized_m11, _m12=sanitized_m12, _m13=sanitized_m13, _m14=sanitized_m14,
        _m21=sanitized_m21, _m22=sanitized_m22, _m23=sanitized_m23, _m24=sanitized_m24,
        _m31=sanitized_m31, _m32=sanitized_m32, _m33=sanitized_m33, _m34=sanitized_m34,
        );

        //println!("nonparametric matrix wgsl shader: {}", wgsl_source);
        let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<glam::Mat4>());
        let bind_info = vec![
            globals.get_bind_info(),
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
        let new_data = Data::Matrix0D {
            buffer: output_buffer,
        };
        Ok((new_data, operation))
    }
}
