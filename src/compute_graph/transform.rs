use std::collections::BTreeMap;
use std::rc::Rc;

use super::Operation;
use crate::computable_scene::globals::Globals;
use super::{SingleDataResult, ProcessingError};
use super::{DataID, Data};
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};


pub fn create(
    device: &wgpu::Device,
    data_map: &BTreeMap<DataID, Data>,
    geometry_id: Option<DataID>,
    matrix_id: Option<DataID>,
) -> SingleDataResult {
    let data_id = geometry_id.ok_or(ProcessingError::InputMissing(" This Transform node \n is missing its Geometry input "))?;
    let geometry_data = data_map.get(&data_id).ok_or(ProcessingError::NoInputData)?;

    let data_id = matrix_id.ok_or(ProcessingError::InputMissing(" This Transform node \n is missing its Matrix input "))?;
    let matrix_data = data_map.get(&data_id).ok_or(ProcessingError::NoInputData)?;

    match (&geometry_data, &matrix_data) {
        (Data::Geom0D{buffer: geom_buffer}, Data::Matrix0D{buffer: matrix_buffer}) => t_0d_0d(device, geom_buffer, matrix_buffer),
        _ => Err(ProcessingError::InternalError("unhandled transform case".into()))
    }

}

// we are transforming a point.
fn t_0d_0d(
    device: &wgpu::Device,
    geom_buffer: &wgpu::Buffer,
    matrix_buffer: &wgpu::Buffer,
    ) -> SingleDataResult {
    let wgsl_source = r##"
[[block]] struct PointBuffer {
    position: vec4<f32>;
};

[[block]] struct MatrixBuffer {
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
