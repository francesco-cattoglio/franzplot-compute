use std::rc::Rc;
use glam::Vec4;

use super::Operation;
use super::globals::Globals;
use super::SingleDataResult;
use super::Data;
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};

pub fn create(
    device: &wgpu::Device,
    globals: &Globals,
    fx: String,
    fy: String,
    fz: String,
) -> SingleDataResult {
    let sanitized_fx = globals.sanitize_expression(&[], &fx)?;
    let sanitized_fy = globals.sanitize_expression(&[], &fy)?;
    let sanitized_fz = globals.sanitize_expression(&[], &fz)?;

    let wgsl_source = format!(r##"
{wgsl_globals}

[[block]] struct OutputBuffer {{
positions: vec4<f32>;
}};

[[group(0), binding(1)]] var<storage, read_write> out: OutputBuffer;

[[stage(compute), workgroup_size(1)]]
fn main() {{
    out.positions.x = {fx};
    out.positions.y = {fy};
    out.positions.z = {fz};
    out.positions.w = 0.0;
}}
"##, wgsl_globals=globals.get_wgsl_header(), fx=sanitized_fx, fy=sanitized_fy, fz=sanitized_fz,
);

    let output_buffer = util::create_storage_buffer(device, std::mem::size_of::<Vec4>());
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
    let new_data = Data::Vector {
        buffer: output_buffer,
    };

    Ok((new_data, operation))
}


