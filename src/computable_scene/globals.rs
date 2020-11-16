use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Globals {
    // TODO: do not give public access to these fields, get a const ref accessors
    pub names: Vec<String>,
    pub values: Vec<f32>,
    // TODO: remove variables, only use names and values
    variables: BTreeMap<String, f32>,
    buffer_size: wgpu::BufferAddress,
    buffer: wgpu::Buffer,
    pub bind_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub shader_header: String,
}

const GLOBAL_CONSTANTS: &[(&str, f32)] = &[
    ("pi", std::f32::consts::PI)
];
const MAX_NUM_VARIABLES: usize = 31;

impl Globals {
    // In this case this function in used inside a Vec::<String>::retain() call,
    // we cannot freely choose its signature! Disable the related clippy lint
    // TODO: reconsider this way of dealing with invalid variable names.
    #[allow(clippy::ptr_arg)]
    fn valid_name(variable_name: &String) -> bool {
        for (constant_name, _value) in GLOBAL_CONSTANTS {
            if variable_name == *constant_name {
                // TODO: this should be logged in as warning!
                println!("Warning, invalid variable name used: {}", variable_name);
                return false;
            }
        }
        println!("Valid global var name: {}", variable_name);
        true
    }

    pub fn new(device: &wgpu::Device, mut variables_names: Vec<String>) -> Self {
        let buffer_size = ((GLOBAL_CONSTANTS.len() + MAX_NUM_VARIABLES) * std::mem::size_of::<f32>()) as wgpu::BufferAddress;

        let mut init_vec = Vec::<f32>::new();
        for (_constant_name, value) in GLOBAL_CONSTANTS {
            init_vec.push(*value);
        }
        init_vec.append(&mut vec![0.0f32; MAX_NUM_VARIABLES]);

        use wgpu::util::DeviceExt;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("globals buffer"),
            contents: bytemuck::cast_slice(&init_vec),
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM
        });
        let bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        count: None,
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::UniformBuffer {
                            min_binding_size: None,
                            dynamic: false,
                        }
                    },
                ],
                label: Some("Globals uniform layout")
            });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer (
                        buffer.slice(..),
                    ),
                },
            ],
            label: Some("variables bind group")
        });

        // write the shader header that will be used in the creation of the compute pipeline shaders
        // and fill up the map that will be used to store the associated values
        let mut shader_header = String::new();
        let mut variables = BTreeMap::<String, f32>::new();
        shader_header += "layout(set = 1, binding = 0) uniform Uniforms {\n";
        for (constant_name, _constant_value) in GLOBAL_CONSTANTS {
            shader_header += &format!("\tfloat {};\n", constant_name);
        }
        // purge input variables for invalid names
        variables_names.retain(Self::valid_name);
        for variable_name in variables_names.into_iter() {
            shader_header += &format!("\tfloat {};\n", variable_name);
            variables.insert(variable_name, 0.0);
        }
        shader_header += "};\n";
        println!("debug info for shader header: {}", &shader_header);

        Self {
            bind_layout,
            bind_group,
            names: vec!{"var_1".into(), "var2".into()},
            values: vec!{0.0, 0.0},
            buffer,
            buffer_size,
            variables,
            shader_header,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, list: &[(String, f32)]) {
        // Update our global variables with the ones found in the passed in list.
        // The passed-in list might contain some variables that do not actually exist;
        // we just do nothing in that case.
        for (name, new_value) in list.iter() {
            if let Some(value) = self.variables.get_mut(name) {
                *value = *new_value;
            }
        }
        // update the mapped values in our buffer. Do not forget that this buffer
        // also contains all the global constants. Start copying from the computed offset!
        let offset = (GLOBAL_CONSTANTS.len() * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let values: Vec<f32> = self.variables.values().copied().collect();
        queue.write_buffer(&self.buffer, offset, bytemuck::cast_slice(&values));
    }

}

