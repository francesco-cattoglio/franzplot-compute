use std::collections::BTreeMap;
use std::rc::Rc;

use super::Operation;
use crate::computable_scene::globals::Globals;
use super::{SingleDataResult, ProcessingError};
use super::{DataID, Data};
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};

use crate::node_graph::Axis;

//#[derive(Debug)]
//pub struct MatrixBlockDescriptor {
//    pub interval: Option<BlockId>,
//    pub row_1: [String; 4], // matrix elements, row-major order
//    pub row_2: [String; 4], // matrix elements, row-major order
//    pub row_3: [String; 4], // matrix elements, row-major order
//}
//
//impl MatrixBlockDescriptor {
//    pub fn make_block(self, device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap) -> ProcessingResult {
//        Ok(ComputeBlock::Matrix(MatrixData::new(device, globals, processed_blocks, self)?))
//    }
//
//    pub fn new_from_rotation(globals: &Globals, axis: Axis, angle: String) -> Result<Self, BlockCreationError> {
//        // we need to write down a different matrix depending on what rotation axis we have
//        let (row_1, row_2, row_3);
//
//        // Sanitize all input expressions
//        let local_params = vec![];
//        let san_a = globals.sanitize_expression(&local_params, &angle)?;
//        match axis {
//            Axis::X => {
//                row_1 = ["1.0".into(),               "0.0".into(),                "0.0".into(), "0.0".into()];
//                row_2 = ["0.0".into(), format!("cos({})", &san_a), format!("-sin({})", &san_a), "0.0".into()];
//                row_3 = ["0.0".into(), format!("sin({})", &san_a),  format!("cos({})", &san_a), "0.0".into()];
//            },
//            Axis::Y => {
//                row_1 = [ format!("cos({})", &san_a), "0.0".into(), format!("sin({})", &san_a), "0.0".into()];
//                row_2 = [               "0.0".into(), "1.0".into(),               "0.0".into(), "0.0".into()];
//                row_3 = [format!("-sin({})", &san_a), "0.0".into(), format!("cos({})", &san_a), "0.0".into()];
//            },
//            Axis::Z => {
//                row_1 = [format!("cos({})", &san_a), format!("-sin({})", &san_a), "0.0".into(), "0.0".into()];
//                row_2 = [format!("sin({})", &san_a),  format!("cos({})", &san_a), "0.0".into(), "0.0".into()];
//                row_3 = [              "0.0".into(),                "0.0".into(), "1.0".into(), "0.0".into()];
//            },
//        }
//        Ok(Self {
//            interval: None,
//            row_1,
//            row_2,
//            row_3,
//        })
//    }
//
//    // TODO: due to the currently hacked-in mathod of translation matrix creation,
//    // this will result in errors being reported twice, once on the input vector and once in the
//    // translation matrix node
//    pub fn new_from_translation(globals: &Globals, x: String, y: String, z: String) -> Result<Self, BlockCreationError> {
//        // Sanitize all input expressions
//        let local_params = vec![];
//        let sanitized_x = globals.sanitize_expression(&local_params, &x)?;
//        let sanitized_y = globals.sanitize_expression(&local_params, &y)?;
//        let sanitized_z = globals.sanitize_expression(&local_params, &z)?;
//
//        let row_1 = ["1.0".into(), "0.0".into(), "0.0".into(), sanitized_x.into()];
//        let row_2 = ["0.0".into(), "1.0".into(), "0.0".into(), sanitized_y.into()];
//        let row_3 = ["0.0".into(), "0.0".into(), "1.0".into(), sanitized_z.into()];
//        Ok(Self {
//            interval: None,
//            row_1,
//            row_2,
//            row_3,
//        })
//    }
//}
//
//impl Default for MatrixBlockDescriptor {
//    fn default() -> Self {
//        Self {
//            interval: None,
//            row_1: ["1.0".into(),"0.0".into(),"0.0".into(),"0.0".into()],
//            row_2: ["0.0".into(),"1.0".into(),"0.0".into(),"0.0".into()],
//            row_3: ["0.0".into(),"0.0".into(),"1.0".into(),"0.0".into()]
//        }
//    }
//}

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

    let sanitized_m11 = globals.sanitize_expression_2(&local_params, &row_1[0])?;
    let sanitized_m12 = globals.sanitize_expression_2(&local_params, &row_1[1])?;
    let sanitized_m13 = globals.sanitize_expression_2(&local_params, &row_1[2])?;
    let sanitized_m14 = globals.sanitize_expression_2(&local_params, &row_1[3])?;
    let sanitized_m21 = globals.sanitize_expression_2(&local_params, &row_2[0])?;
    let sanitized_m22 = globals.sanitize_expression_2(&local_params, &row_2[1])?;
    let sanitized_m23 = globals.sanitize_expression_2(&local_params, &row_2[2])?;
    let sanitized_m24 = globals.sanitize_expression_2(&local_params, &row_2[3])?;
    let sanitized_m31 = globals.sanitize_expression_2(&local_params, &row_3[0])?;
    let sanitized_m32 = globals.sanitize_expression_2(&local_params, &row_3[1])?;
    let sanitized_m33 = globals.sanitize_expression_2(&local_params, &row_3[2])?;
    let sanitized_m34 = globals.sanitize_expression_2(&local_params, &row_3[3])?;

    if let Some((input_buffer, param)) = optional_interval {
        let wgsl_source = format!(r##"
{wgsl_header}

[[block]] struct InputBuffer {{
    values: array<f32>;
}};

[[block]] struct OutputBuffer {{
    matrices: array<mat4x4<f32>>;
}};

[[group(0), binding(1)]] var<storage, read> input: InputBuffer;
[[group(0), binding(2)]] var<storage, read_write> output: OutputBuffer;

[[stage(compute), workgroup_size(16)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    let index = global_id.x;
    let {par} = input.values[index];
    let matrix = mat4x4<f32>(
        vec4<f32>({_m11}, {_m21}, {_m31}, 0.0),
        vec4<f32>({_m12}, {_m22}, {_m32}, 0.0),
        vec4<f32>({_m13}, {_m23}, {_m33}, 0.0),
        vec4<f32>({_m14}, {_m24}, {_m34}, 1.0),
    );
    output.matrices[index] = matrix;
}}
        "##, wgsl_header=globals.get_wgsl_header(), par=param.name.as_ref().unwrap(),
        _m11=sanitized_m11, _m12=sanitized_m12, _m13=sanitized_m13, _m14=sanitized_m14,
        _m21=sanitized_m21, _m22=sanitized_m22, _m23=sanitized_m23, _m24=sanitized_m24,
        _m31=sanitized_m31, _m32=sanitized_m32, _m33=sanitized_m33, _m34=sanitized_m34,
        );

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
            param: param.clone(),
        };
        Ok((new_data, operation))
    } else {
        let wgsl_source = format!(r##"
{wgsl_header}

[[block]] struct OutputBuffer {{
    matrix: mat4x4<f32>;
}};

[[group(0), binding(1)]] var<storage, read_write> output: OutputBuffer;

[[stage(compute), workgroup_size(1)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
    let matrix = mat4x4<f32>(
        vec4<f32>({_m11}, {_m21}, {_m31}, 0.0),
        vec4<f32>({_m12}, {_m22}, {_m32}, 0.0),
        vec4<f32>({_m13}, {_m23}, {_m33}, 0.0),
        vec4<f32>({_m14}, {_m24}, {_m34}, 1.0),
    );
    output.matrix = matrix;
}}
        "##, wgsl_header=globals.get_wgsl_header(),
        _m11=sanitized_m11, _m12=sanitized_m12, _m13=sanitized_m13, _m14=sanitized_m14,
        _m21=sanitized_m21, _m22=sanitized_m22, _m23=sanitized_m23, _m24=sanitized_m24,
        _m31=sanitized_m31, _m32=sanitized_m32, _m33=sanitized_m33, _m34=sanitized_m34,
        );

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
//    pub fn new(device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap, descriptor: MatrixBlockDescriptor) -> Result<Self, BlockCreationError> {
//        if descriptor.interval.is_some() {
//            Self::new_with_interval(device, globals, processed_blocks, descriptor)
//        } else {
//            Self::new_without_interval(device, globals, descriptor)
//        }
//    }
//
//    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
//            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
//                label: Some("matrix compute pass"),
//            });
//            compute_pass.set_pipeline(&self.compute_pipeline);
//            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
//            compute_pass.set_bind_group(1, variables_bind_group, &[]);
//            compute_pass.dispatch(1, 1, 1);
//    }
//
//    fn new_with_interval(device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap, desc: MatrixBlockDescriptor) -> Result<Self, BlockCreationError> {
//        let input_id = desc.interval.ok_or(BlockCreationError::InternalError("Matrix new_with_interval() called with no-interval descriptor".into()))?;
//        let found_element = processed_blocks.get(&input_id).ok_or(BlockCreationError::InternalError("Matrix interval input does not exist in the block map".into()))?;
//        let input_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;
//
//        let interval_data = match input_block {
//            ComputeBlock::Interval(data) => data,
//            _ => return Err(BlockCreationError::InputInvalid("the input provided to the Matrix is not an Interval"))
//        };
//
//        let param = interval_data.out_dim.as_1d()?;
//        let param_name = param.name.clone().unwrap();
//
//        // Sanitize all input expressions
//        // TODO: maybe macros?
//        // BEWARE: named entries follow the "maths" convention: the upper left element is the
//        // element (1, 1), but `row_n` is an array and therefore starts from 0!
//        let local_params = vec![param_name.as_str()];
//        let sanitized_m11 = globals.sanitize_expression(&local_params, &desc.row_1[0])?;
//        let sanitized_m12 = globals.sanitize_expression(&local_params, &desc.row_1[1])?;
//        let sanitized_m13 = globals.sanitize_expression(&local_params, &desc.row_1[2])?;
//        let sanitized_m14 = globals.sanitize_expression(&local_params, &desc.row_1[3])?;
//        let sanitized_m21 = globals.sanitize_expression(&local_params, &desc.row_2[0])?;
//        let sanitized_m22 = globals.sanitize_expression(&local_params, &desc.row_2[1])?;
//        let sanitized_m23 = globals.sanitize_expression(&local_params, &desc.row_2[2])?;
//        let sanitized_m24 = globals.sanitize_expression(&local_params, &desc.row_2[3])?;
//        let sanitized_m31 = globals.sanitize_expression(&local_params, &desc.row_3[0])?;
//        let sanitized_m32 = globals.sanitize_expression(&local_params, &desc.row_3[1])?;
//        let sanitized_m33 = globals.sanitize_expression(&local_params, &desc.row_3[2])?;
//        let sanitized_m34 = globals.sanitize_expression(&local_params, &desc.row_3[3])?;
//
//        let shader_source = format!(r##"
//#version 450
//layout(local_size_x = {dimx}, local_size_y = 1) in;
//
//layout(set = 0, binding = 0) buffer InputBuffer {{
//    float in_buff[];
//}};
//
//layout(set = 0, binding = 1) buffer OutputBuffer {{
//    mat4 out_buff[];
//}};
//
//{header}
//
//void main() {{
//    uint index = gl_GlobalInvocationID.x;
//    float {par} = in_buff[index];
//    vec4 col_0 = vec4({_m11}, {_m21}, {_m31}, 0.0);
//    vec4 col_1 = vec4({_m12}, {_m22}, {_m32}, 0.0);
//    vec4 col_2 = vec4({_m13}, {_m23}, {_m33}, 0.0);
//    vec4 col_3 = vec4({_m14}, {_m24}, {_m34}, 1.0);
//
//    out_buff[index][0] = col_0;
//    out_buff[index][1] = col_1;
//    out_buff[index][2] = col_2;
//    out_buff[index][3] = col_3;
//}}
//"##, header=&globals.shader_header, par=&param_name, dimx=param.size,
//    _m11=sanitized_m11, _m12=sanitized_m12, _m13=sanitized_m13, _m14=sanitized_m14,
//    _m21=sanitized_m21, _m22=sanitized_m22, _m23=sanitized_m23, _m24=sanitized_m24,
//    _m31=sanitized_m31, _m32=sanitized_m32, _m33=sanitized_m33, _m34=sanitized_m34,
//);
//
//        let out_dim = Dimensions::D1(param);
//        let out_buffer = out_dim.create_storage_buffer(16 * std::mem::size_of::<f32>(), device);
//
//        let bindings = [
//            // add descriptor for input buffer
//            CustomBindDescriptor {
//                position: 0,
//                buffer: &interval_data.out_buffer
//            },
//            // add descriptor for output buffer
//            CustomBindDescriptor {
//                position: 1,
//                buffer: &out_buffer
//            }
//        ];
//
//        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, Some(&globals.bind_layout), Some("Interval"))?;
//
//        Ok(Self {
//            compute_pipeline,
//            compute_bind_group,
//            out_buffer,
//            out_dim,
//        })
//    }
//
//    fn new_without_interval(device: &wgpu::Device, globals: &Globals, desc: MatrixBlockDescriptor) -> Result<Self, BlockCreationError> {
//
//        // Sanitize all input expressions
//        // TODO: DRY, this is exactly the same code as the with_interval function
//        // BEWARE: named entries follow the "maths" convention: the upper left element is the
//        // element (1, 1), but `row_n` is an array and therefore starts from 0!
//        let local_params = vec![];
//        let sanitized_m11 = globals.sanitize_expression(&local_params, &desc.row_1[0])?;
//        let sanitized_m12 = globals.sanitize_expression(&local_params, &desc.row_1[1])?;
//        let sanitized_m13 = globals.sanitize_expression(&local_params, &desc.row_1[2])?;
//        let sanitized_m14 = globals.sanitize_expression(&local_params, &desc.row_1[3])?;
//        let sanitized_m21 = globals.sanitize_expression(&local_params, &desc.row_2[0])?;
//        let sanitized_m22 = globals.sanitize_expression(&local_params, &desc.row_2[1])?;
//        let sanitized_m23 = globals.sanitize_expression(&local_params, &desc.row_2[2])?;
//        let sanitized_m24 = globals.sanitize_expression(&local_params, &desc.row_2[3])?;
//        let sanitized_m31 = globals.sanitize_expression(&local_params, &desc.row_3[0])?;
//        let sanitized_m32 = globals.sanitize_expression(&local_params, &desc.row_3[1])?;
//        let sanitized_m33 = globals.sanitize_expression(&local_params, &desc.row_3[2])?;
//        let sanitized_m34 = globals.sanitize_expression(&local_params, &desc.row_3[3])?;
//
//        let shader_source = format!(r##"
//#version 450
//layout(local_size_x = 1, local_size_y = 1) in;
//
//layout(set = 0, binding = 0) buffer OutputBuffer {{
//    mat4 out_buff[];
//}};
//
//{header}
//
//void main() {{
//    uint index = gl_GlobalInvocationID.x;
//    vec4 col_0 = vec4({_m11}, {_m21}, {_m31}, 0.0);
//    vec4 col_1 = vec4({_m12}, {_m22}, {_m32}, 0.0);
//    vec4 col_2 = vec4({_m13}, {_m23}, {_m33}, 0.0);
//    vec4 col_3 = vec4({_m14}, {_m24}, {_m34}, 1.0);
//
//    out_buff[index][0] = col_0;
//    out_buff[index][1] = col_1;
//    out_buff[index][2] = col_2;
//    out_buff[index][3] = col_3;
//}}
//"##, header=&globals.shader_header,
//    _m11=sanitized_m11, _m12=sanitized_m12, _m13=sanitized_m13, _m14=sanitized_m14,
//    _m21=sanitized_m21, _m22=sanitized_m22, _m23=sanitized_m23, _m24=sanitized_m24,
//    _m31=sanitized_m31, _m32=sanitized_m32, _m33=sanitized_m33, _m34=sanitized_m34,
//);
//
//        let out_dim = Dimensions::D0;
//        let out_buffer = out_dim.create_storage_buffer(16 * std::mem::size_of::<f32>(), device);
//
//        let bindings = [
//            // add descriptor for output buffer
//            CustomBindDescriptor {
//                position: 0,
//                buffer: &out_buffer
//            }
//        ];
//
//        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, Some(&globals.bind_layout), Some("Interval"))?;
//
//        Ok(Self {
//            compute_pipeline,
//            compute_bind_group,
//            out_buffer,
//            out_dim,
//        })
//    }
//
//}
