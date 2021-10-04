use std::collections::BTreeMap;
use crate::computable_scene::globals::Globals;
use super::{ProcessingResult, ProcessingError};
use super::Parameter;
use super::{DataID, Data};
use crate::util;
use crate::shader_processing::{naga_compute_pipeline, BindInfo};

pub struct Operation {
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
    dim: [u32; 3],
}

impl Operation {
    pub fn new_interval(
        device: &wgpu::Device,
        globals: &Globals,
        name: String,
        begin: String,
        end: String,
        quality: usize,
        output_id: DataID,
    ) -> ProcessingResult {
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

[[group(0), binding(1)]] var<storage, write> _output: OutputBuffer;

[[stage(compute), workgroup_size({n_points})]]
fn main([[builtin(global_invocation_id)]] _global_id: vec3<u32>) {{
    let _index = _global_id.x;
    let _delta: f32 = ({interval_end} - {interval_begin}) / (f32({n_points}) - 1.0);
    _output.values[_index] = {interval_begin} + _delta * f32(_index);
}}
"##, wgsl_globals=globals.get_wgsl_header(), interval_begin=&param.begin, interval_end=&param.end, n_points=param.size
);

        println!("shader source:\n {}", &wgsl_source);

        let out_buffer = util::create_storage_buffer(device, std::mem::size_of::<f32>() * param.size);

        let bind_info = vec![
            globals.get_bind_info(),
            BindInfo {
                buffer: &out_buffer,
                ty: wgpu::BufferBindingType::Storage { read_only: false },
            },
        ];
        let (pipeline, bind_group) = naga_compute_pipeline(device, &wgsl_source, &bind_info);

        let mut new_data = BTreeMap::<DataID, Data>::new();
        new_data.insert(
            output_id,
            Data::Interval {
                buffer: out_buffer,
                param,
            },
        );
        let operation = Operation {
            bind_group,
            pipeline,
            dim: [1, 1, 1],
        };

        Ok((new_data, operation))
    }

    pub fn new_geometry_rendering(
        device: &wgpu::Device,
        data_map: &BTreeMap<DataID, Data>,
        geometry_id: Option<DataID>,
        output_id: DataID,
    ) -> ProcessingResult {
        println!("new geometry rendering processing");
        let data_id = geometry_id.ok_or(ProcessingError::InputMissing(" This Curve node \n is missing its input "))?;
        let found_data = data_map.get(&data_id).ok_or(ProcessingError::InternalError("Geometry used as input does not exist in the block map".into()))?;

        let (input_buffer, param) = match found_data {
            Data::Geom1D {
                buffer, param
            } => (buffer, param),
            Data::Geom2D {
                ..
            } => todo!(),
            Data::Prefab {
                ..
            } => todo!(),
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
[[group(0), binding(2)]] var<storage, write> _output: OutputBuffer;

[[stage(compute), workgroup_size({n_points})]]
fn main([[builtin(global_invocation_id)]] _global_id: vec3<u32>) {{
    let _index = _global_id.x;
    let {par} = _input.values[index];
    let _fx = {fx};
    let _fy = {fy};
    let _fz = {fz};
    _output.positions[index] = vec4<f32>(_fx, _fy, _fz, 1.0);
}}
"##, wgsl_header=globals.get_wgsl_header(), par=param_name, fx=sanitized_fx, fy=sanitized_fy, fz=sanitized_fz, n_points=param.size
);

        dbg!(&wgsl_source);

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
            Data::Interval {
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
}


    pub fn new_curve(
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
[[group(0), binding(2)]] var<storage, write> _output: OutputBuffer;

[[stage(compute), workgroup_size({n_points})]]
fn main([[builtin(global_invocation_id)]] _global_id: vec3<u32>) {{
    let _index = _global_id.x;
    let {par} = _input.values[index];
    let _fx = {fx};
    let _fy = {fy};
    let _fz = {fz};
    _output.positions[index] = vec4<f32>(_fx, _fy, _fz, 1.0);
}}
"##, wgsl_header=globals.get_wgsl_header(), par=param_name, fx=sanitized_fx, fy=sanitized_fy, fz=sanitized_fz, n_points=param.size
);

        dbg!(&wgsl_source);

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
            Data::Interval {
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
}

