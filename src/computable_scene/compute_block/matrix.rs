use crate::computable_scene::globals::Globals;
use crate::shader_processing::*;
use super::ComputeBlock;
use super::BlockId;
use super::BlockCreationError;
use super::{ProcessedMap, ProcessingResult};
use super::Dimensions;
use crate::node_graph::Axis;

#[derive(Debug)]
pub struct MatrixBlockDescriptor {
    pub interval: Option<BlockId>,
    pub row_1: [String; 4], // matrix elements, row-major order
    pub row_2: [String; 4], // matrix elements, row-major order
    pub row_3: [String; 4], // matrix elements, row-major order
}

impl MatrixBlockDescriptor {
    pub fn make_block(self, device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap) -> ProcessingResult {
        Ok(ComputeBlock::Matrix(MatrixData::new(device, globals, processed_blocks, self)?))
    }

    pub fn new_from_rotation(axis: Axis, angle: String) -> Result<Self, BlockCreationError> {
        // we need to write down a different matrix depending on what rotation axis we have
        let (row_1, row_2, row_3);

        // Sanitize all input expressions
        let maybe_angle = Globals::sanitize_expression(&angle);
        let san_a = maybe_angle.ok_or(BlockCreationError::IncorrectAttributes(" the angle field \n contains invalid symbols "))?;
        match axis {
            Axis::X => {
                row_1 = ["1.0".into(),               "0.0".into(),                "0.0".into(), "0.0".into()];
                row_2 = ["0.0".into(), format!("cos({})", &san_a), format!("-sin({})", &san_a), "0.0".into()];
                row_3 = ["0.0".into(), format!("sin({})", &san_a),  format!("cos({})", &san_a), "0.0".into()];
            },
            Axis::Y => {
                row_1 = [ format!("cos({})", &san_a), "0.0".into(), format!("sin({})", &san_a), "0.0".into()];
                row_2 = [               "0.0".into(), "1.0".into(),               "0.0".into(), "0.0".into()];
                row_3 = [format!("-sin({})", &san_a), "0.0".into(), format!("cos({})", &san_a), "0.0".into()];
            },
            Axis::Z => {
                row_1 = [format!("cos({})", &san_a), format!("-sin({})", &san_a), "0.0".into(), "0.0".into()];
                row_2 = [format!("sin({})", &san_a),  format!("cos({})", &san_a), "0.0".into(), "0.0".into()];
                row_3 = [              "0.0".into(),                "0.0".into(), "1.0".into(), "0.0".into()];
            },
        }
        Ok(Self {
            interval: None,
            row_1,
            row_2,
            row_3,
        })
    }

    // TODO: due to the currently hacked-in mathod of translation matrix creation,
    // this will result in errors being reported twice, once on the input vector and once in the
    // translation matrix node
    pub fn new_from_translation(x: String, y: String, z: String) -> Result<Self, BlockCreationError> {
        // Sanitize all input expressions
        let maybe_x = Globals::sanitize_expression(&x);
        let sanitized_x = maybe_x.ok_or(BlockCreationError::IncorrectAttributes(" the x field \n contains invalid symbols "))?;
        let maybe_y = Globals::sanitize_expression(&y);
        let sanitized_y = maybe_y.ok_or(BlockCreationError::IncorrectAttributes(" the y field \n contains invalid symbols "))?;
        let maybe_z = Globals::sanitize_expression(&z);
        let sanitized_z = maybe_z.ok_or(BlockCreationError::IncorrectAttributes(" the z field \n contains invalid symbols "))?;

        let row_1 = ["1.0".into(), "0.0".into(), "0.0".into(), sanitized_x.into()];
        let row_2 = ["0.0".into(), "1.0".into(), "0.0".into(), sanitized_y.into()];
        let row_3 = ["0.0".into(), "0.0".into(), "1.0".into(), sanitized_z.into()];
        Ok(Self {
            interval: None,
            row_1,
            row_2,
            row_3,
        })
    }
}

impl Default for MatrixBlockDescriptor {
    fn default() -> Self {
        Self {
            interval: None,
            row_1: ["1.0".into(),"0.0".into(),"0.0".into(),"0.0".into()],
            row_2: ["0.0".into(),"1.0".into(),"0.0".into(),"0.0".into()],
            row_3: ["0.0".into(),"0.0".into(),"1.0".into(),"0.0".into()]
        }
    }
}

pub struct MatrixData {
    pub out_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    pub out_dim: Dimensions,
}

impl MatrixData {
    pub fn new(device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap, descriptor: MatrixBlockDescriptor) -> Result<Self, BlockCreationError> {
        if descriptor.interval.is_some() {
            Self::new_with_interval(device, globals, processed_blocks, descriptor)
        } else {
            Self::new_without_interval(device, globals, descriptor)
        }
    }

    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("matrix compute pass"),
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            compute_pass.set_bind_group(1, variables_bind_group, &[]);
            compute_pass.dispatch(1, 1, 1);
    }

    fn new_with_interval(device: &wgpu::Device, globals: &Globals, processed_blocks: &ProcessedMap, desc: MatrixBlockDescriptor) -> Result<Self, BlockCreationError> {
        let input_id = desc.interval.ok_or(BlockCreationError::InternalError("Matrix new_with_interval() called with no-interval descriptor"))?;
        let found_element = processed_blocks.get(&input_id).ok_or(BlockCreationError::InternalError("Matrix interval input does not exist in the block map"))?;
        let input_block: &ComputeBlock = found_element.as_ref().or(Err(BlockCreationError::InputNotBuilt(" Node not computed \n due to previous errors ")))?;

        let interval_data = match input_block {
            ComputeBlock::Interval(data) => data,
            _ => return Err(BlockCreationError::InputInvalid("the input provided to the Matrix is not an Interval"))
        };

        let param = interval_data.out_dim.as_1d()?;
        let param_name = param.name.clone().unwrap();

        // Sanitize all input expressions
        // TODO: maybe macros?
        // BEWARE: named entries follow the "maths" convention: the upper left element is the
        // element (1, 1), but `row_n` is an array and therefore starts from 0!
        let maybe_m11 = Globals::sanitize_expression(&desc.row_1[0]);
        let sanitized_m11 = maybe_m11.ok_or(BlockCreationError::IncorrectAttributes(" the (1,1) entry \n contains invalid symbols "))?;
        let maybe_m12 = Globals::sanitize_expression(&desc.row_1[1]);
        let sanitized_m12 = maybe_m12.ok_or(BlockCreationError::IncorrectAttributes(" the (1,2) entry \n contains invalid symbols "))?;
        let maybe_m13 = Globals::sanitize_expression(&desc.row_1[2]);
        let sanitized_m13 = maybe_m13.ok_or(BlockCreationError::IncorrectAttributes(" the (1,3) entry \n contains invalid symbols "))?;
        let maybe_m14 = Globals::sanitize_expression(&desc.row_1[3]);
        let sanitized_m14 = maybe_m14.ok_or(BlockCreationError::IncorrectAttributes(" the (1,4) entry \n contains invalid symbols "))?;
        let maybe_m21 = Globals::sanitize_expression(&desc.row_2[0]);
        let sanitized_m21 = maybe_m21.ok_or(BlockCreationError::IncorrectAttributes(" the (2,1) entry \n contains invalid symbols "))?;
        let maybe_m22 = Globals::sanitize_expression(&desc.row_2[1]);
        let sanitized_m22 = maybe_m22.ok_or(BlockCreationError::IncorrectAttributes(" the (2,2) entry \n contains invalid symbols "))?;
        let maybe_m23 = Globals::sanitize_expression(&desc.row_2[2]);
        let sanitized_m23 = maybe_m23.ok_or(BlockCreationError::IncorrectAttributes(" the (2,3) entry \n contains invalid symbols "))?;
        let maybe_m24 = Globals::sanitize_expression(&desc.row_2[3]);
        let sanitized_m24 = maybe_m24.ok_or(BlockCreationError::IncorrectAttributes(" the (2,4) entry \n contains invalid symbols "))?;
        let maybe_m31 = Globals::sanitize_expression(&desc.row_3[0]);
        let sanitized_m31 = maybe_m31.ok_or(BlockCreationError::IncorrectAttributes(" the (3,1) entry \n contains invalid symbols "))?;
        let maybe_m32 = Globals::sanitize_expression(&desc.row_3[1]);
        let sanitized_m32 = maybe_m32.ok_or(BlockCreationError::IncorrectAttributes(" the (3,2) entry \n contains invalid symbols "))?;
        let maybe_m33 = Globals::sanitize_expression(&desc.row_3[2]);
        let sanitized_m33 = maybe_m33.ok_or(BlockCreationError::IncorrectAttributes(" the (3,3) entry \n contains invalid symbols "))?;
        let maybe_m34 = Globals::sanitize_expression(&desc.row_3[3]);
        let sanitized_m34 = maybe_m34.ok_or(BlockCreationError::IncorrectAttributes(" the (3,4) entry \n contains invalid symbols "))?;

        let shader_source = format!(r##"
#version 450
layout(local_size_x = {dimx}, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer InputBuffer {{
    float in_buff[];
}};

layout(set = 0, binding = 1) buffer OutputBuffer {{
    mat4 out_buff[];
}};

{header}

void main() {{
    uint index = gl_GlobalInvocationID.x;
    float {par} = in_buff[index];
    vec4 col_0 = vec4({_m11}, {_m21}, {_m31}, 0.0);
    vec4 col_1 = vec4({_m12}, {_m22}, {_m32}, 0.0);
    vec4 col_2 = vec4({_m13}, {_m23}, {_m33}, 0.0);
    vec4 col_3 = vec4({_m14}, {_m24}, {_m34}, 1.0);

    out_buff[index][0] = col_0;
    out_buff[index][1] = col_1;
    out_buff[index][2] = col_2;
    out_buff[index][3] = col_3;
}}
"##, header=&globals.shader_header, par=&param_name, dimx=param.size,
    _m11=sanitized_m11, _m12=sanitized_m12, _m13=sanitized_m13, _m14=sanitized_m14,
    _m21=sanitized_m21, _m22=sanitized_m22, _m23=sanitized_m23, _m24=sanitized_m24,
    _m31=sanitized_m31, _m32=sanitized_m32, _m33=sanitized_m33, _m34=sanitized_m34,
);

        let out_dim = Dimensions::D1(param);
        let out_buffer = out_dim.create_storage_buffer(16 * std::mem::size_of::<f32>(), device);

        let bindings = [
            // add descriptor for input buffer
            CustomBindDescriptor {
                position: 0,
                buffer: &interval_data.out_buffer
            },
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 1,
                buffer: &out_buffer
            }
        ];

        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, Some(&globals.bind_layout), Some("Interval"))?;

        Ok(Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
        })
    }

    fn new_without_interval(device: &wgpu::Device, globals: &Globals, desc: MatrixBlockDescriptor) -> Result<Self, BlockCreationError> {

        // Sanitize all input expressions
        // TODO: DRY, this is exactly the same code as the with_interval function
        // BEWARE: named entries follow the "maths" convention: the upper left element is the
        // element (1, 1), but `row_n` is an array and therefore starts from 0!
        let maybe_m11 = Globals::sanitize_expression(&desc.row_1[0]);
        let sanitized_m11 = maybe_m11.ok_or(BlockCreationError::IncorrectAttributes(" the (1,1) entry \n contains invalid symbols "))?;
        let maybe_m12 = Globals::sanitize_expression(&desc.row_1[1]);
        let sanitized_m12 = maybe_m12.ok_or(BlockCreationError::IncorrectAttributes(" the (1,2) entry \n contains invalid symbols "))?;
        let maybe_m13 = Globals::sanitize_expression(&desc.row_1[2]);
        let sanitized_m13 = maybe_m13.ok_or(BlockCreationError::IncorrectAttributes(" the (1,3) entry \n contains invalid symbols "))?;
        let maybe_m14 = Globals::sanitize_expression(&desc.row_1[3]);
        let sanitized_m14 = maybe_m14.ok_or(BlockCreationError::IncorrectAttributes(" the (1,4) entry \n contains invalid symbols "))?;
        let maybe_m21 = Globals::sanitize_expression(&desc.row_2[0]);
        let sanitized_m21 = maybe_m21.ok_or(BlockCreationError::IncorrectAttributes(" the (2,1) entry \n contains invalid symbols "))?;
        let maybe_m22 = Globals::sanitize_expression(&desc.row_2[1]);
        let sanitized_m22 = maybe_m22.ok_or(BlockCreationError::IncorrectAttributes(" the (2,2) entry \n contains invalid symbols "))?;
        let maybe_m23 = Globals::sanitize_expression(&desc.row_2[2]);
        let sanitized_m23 = maybe_m23.ok_or(BlockCreationError::IncorrectAttributes(" the (2,3) entry \n contains invalid symbols "))?;
        let maybe_m24 = Globals::sanitize_expression(&desc.row_2[3]);
        let sanitized_m24 = maybe_m24.ok_or(BlockCreationError::IncorrectAttributes(" the (2,4) entry \n contains invalid symbols "))?;
        let maybe_m31 = Globals::sanitize_expression(&desc.row_3[0]);
        let sanitized_m31 = maybe_m31.ok_or(BlockCreationError::IncorrectAttributes(" the (3,1) entry \n contains invalid symbols "))?;
        let maybe_m32 = Globals::sanitize_expression(&desc.row_3[1]);
        let sanitized_m32 = maybe_m32.ok_or(BlockCreationError::IncorrectAttributes(" the (3,2) entry \n contains invalid symbols "))?;
        let maybe_m33 = Globals::sanitize_expression(&desc.row_3[2]);
        let sanitized_m33 = maybe_m33.ok_or(BlockCreationError::IncorrectAttributes(" the (3,3) entry \n contains invalid symbols "))?;
        let maybe_m34 = Globals::sanitize_expression(&desc.row_3[3]);
        let sanitized_m34 = maybe_m34.ok_or(BlockCreationError::IncorrectAttributes(" the (3,4) entry \n contains invalid symbols "))?;

        let shader_source = format!(r##"
#version 450
layout(local_size_x = 1, local_size_y = 1) in;

layout(set = 0, binding = 0) buffer OutputBuffer {{
    mat4 out_buff[];
}};

{header}

void main() {{
    uint index = gl_GlobalInvocationID.x;
    vec4 col_0 = vec4({_m11}, {_m21}, {_m31}, 0.0);
    vec4 col_1 = vec4({_m12}, {_m22}, {_m32}, 0.0);
    vec4 col_2 = vec4({_m13}, {_m23}, {_m33}, 0.0);
    vec4 col_3 = vec4({_m14}, {_m24}, {_m34}, 1.0);

    out_buff[index][0] = col_0;
    out_buff[index][1] = col_1;
    out_buff[index][2] = col_2;
    out_buff[index][3] = col_3;
}}
"##, header=&globals.shader_header,
    _m11=sanitized_m11, _m12=sanitized_m12, _m13=sanitized_m13, _m14=sanitized_m14,
    _m21=sanitized_m21, _m22=sanitized_m22, _m23=sanitized_m23, _m24=sanitized_m24,
    _m31=sanitized_m31, _m32=sanitized_m32, _m33=sanitized_m33, _m34=sanitized_m34,
);

        let out_dim = Dimensions::D0;
        let out_buffer = out_dim.create_storage_buffer(16 * std::mem::size_of::<f32>(), device);

        let bindings = [
            // add descriptor for output buffer
            CustomBindDescriptor {
                position: 0,
                buffer: &out_buffer
            }
        ];

        let (compute_pipeline, compute_bind_group) = compile_compute_shader(device, shader_source.as_str(), &bindings, Some(&globals.bind_layout), Some("Interval"))?;

        Ok(Self {
            compute_pipeline,
            compute_bind_group,
            out_buffer,
            out_dim,
        })
    }

}
