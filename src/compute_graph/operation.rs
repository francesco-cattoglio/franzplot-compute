use std::collections::BTreeMap;
use crate::computable_scene::globals::Globals;
use super::{ProcessingResult, ProcessingError};
use super::{DataID, Data};

pub enum Operation {
    Curve {
    },
    Output1D {
    }
}

impl Operation {
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
        //let found_data = data_map.get(&data_id).ok_or(ProcessingError::InternalError("Interval used as input does not exist in the block map".into()))?;

        //let (buffer, parameter) = match found_data {
        //    Data::Interval{
        //        buffer, param
        //    } => (buffer, param),
        //    _ => return Err(ProcessingError::InternalError("the input provided to the Curve is not an Interval".into()))
        //};

        //let param_name = parameter.name.clone().unwrap();

        //// Sanitize all input expressions
        //let local_params = vec![param_name.as_str()];
        //let sanitized_fx = globals.sanitize_expression_2(&local_params, &fx)?;
        //let sanitized_fy = globals.sanitize_expression_2(&local_params, &fy)?;
        //let sanitized_fz = globals.sanitize_expression_2(&local_params, &fz)?;

        //// We are creating a curve from an interval, output vertex count is the same as interval
        //// one, but buffer size is 4 times as much, because we are storing a Vec4 instead of a f32
        //let buffer = crate::util::create_storage_buffer(device, 4 * std::mem::size_of::<f32>() * parameter.size);
        let buffer = crate::util::create_storage_buffer(device, 4 * std::mem::size_of::<f32>() * 1);

        let wgsl_source = include_str!("test_curve.wgsl");
        let wgsl_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("compute shader module"),
            source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: None,
            layout: None,
            module: &wgsl_module,
            entry_point: "main",
        });

        println!("naga deduced the layout (0): {:?}", compute_pipeline.get_bind_group_layout(0));
        println!("naga deduced the layout (1): {:?}", compute_pipeline.get_bind_group_layout(1));

        let mut new_data = BTreeMap::<DataID, Data>::new();
        new_data.insert(
            output_id,
            Data::Geom1D {
                buffer,
                //param: parameter.clone(),
            },
        );
        Err(ProcessingError::Unknown)
    }
}

