use std::collections::BTreeMap;
use std::rc::Rc;
use super::Operation;
use crate::computable_scene::globals::Globals;
use super::{SingleDataResult, ProcessingError};
use super::Parameter;
use super::{DataID, Data};
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};

pub fn create(
    device: &wgpu::Device,
    globals: &Globals,
    name: String,
    begin: String,
    end: String,
    quality: usize,
) -> SingleDataResult {
    if quality < 1 || quality > 16 {
        return Err(ProcessingError::IncorrectAttributes("Interval quality attribute must be an integer in the [1, 16] range"))
    }
    if name.is_empty() {
        return Err(ProcessingError::IncorrectAttributes(" please provide a name \n for the interval's variable "));
    }
    if begin.is_empty() {
        return Err(ProcessingError::IncorrectAttributes(" please provide an expression \n for the interval's begin "));
    }
    if end.is_empty() {
        return Err(ProcessingError::IncorrectAttributes(" please provide an expression \n for the interval's end "));
    }

    let n_evals = 16 * quality;
    // Make sure that the name does not contain any internal whitespace
    let sanitized_name = Globals::sanitize_variable_name_2(&name)?;

    // Note that sanitizing also removes leading and trailing whitespaces in the begin and end fields.
    // This is done here because Parameters can be compared, and if we strip all
    // whitespaces here we are sure that the comparison will be succesful if the user
    // inputs the same thing in two different nodes but adds an extra whitespace.
    // TODO: if the user enters the same number but writes it differently, the comparison can
    // fail nonetheless, we need to make sure we sanitize the values as well
    let local_params = vec![];
    let sanitized_begin = globals.sanitize_expression_2(&local_params, &begin)?;
    let sanitized_end = globals.sanitize_expression_2(&local_params, &end)?;
    let param = Parameter {
        name: Some(sanitized_name),
        begin: sanitized_begin,
        end: sanitized_end,
        size: n_evals,
        use_interval_as_uv: false,
    };

    let wgsl_source = format!(r##"
{wgsl_globals}

[[block]] struct OutputBuffer {{
values: array<f32>;
}};

[[group(0), binding(1)]] var<storage, read_write> output: OutputBuffer;

[[stage(compute), workgroup_size({n_points})]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {{
let index = global_id.x;
let delta: f32 = ({interval_end} - {interval_begin}) / (f32({n_points}) - 1.0);
output.values[index] = {interval_begin} + delta * f32(index);
}}
"##, wgsl_globals=globals.get_wgsl_header(), interval_begin=&param.begin, interval_end=&param.end, n_points=param.size
);

    //println!("shader source:\n {}", &wgsl_source);

    let out_buffer = util::create_storage_buffer(device, std::mem::size_of::<f32>() * param.size);

    let bind_info = vec![
        globals.get_bind_info(),
        BindInfo {
            buffer: &out_buffer,
            ty: wgpu::BufferBindingType::Storage { read_only: false },
        },
    ];
    let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

    let new_data = Data::Interval {
        buffer: out_buffer,
        param,
    };
    let operation = Operation {
        bind_group,
        pipeline: Rc::new(pipeline),
        dim: [1, 1, 1],
    };

    Ok((new_data, operation))
}

