
#[derive(Debug)]
pub struct Globals {
    names: Vec<String>,
    values: Vec<f32>,
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
    fn valid_name(variable_name: &str) -> bool {
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

    pub fn get_variables_iter(&mut self) -> impl Iterator<Item = (&String, &mut f32)> {
        self.names.iter().zip(self.values.iter_mut())
    }

    pub fn new(device: &wgpu::Device, variables_names: Vec<String>, init_values: Vec<f32>) -> Self {
        // assert there are as many variables as init values
        assert!(variables_names.len() == init_values.len());

        // First: create a buffer that is big enough it can contain both the global constants and
        // the global variables.
        let buffer_size = ((GLOBAL_CONSTANTS.len() + MAX_NUM_VARIABLES) * std::mem::size_of::<f32>()) as wgpu::BufferAddress;

        // Initialize the buffer, all the constants are copied in first, then append all the variables
        let mut init_vec = Vec::<f32>::new();
        for (_constant_name, value) in GLOBAL_CONSTANTS {
            init_vec.push(*value);
        }
        init_vec.extend_from_slice(&init_values);

        // create the actual buffer, the bind layout and the bind group
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
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            min_binding_size: None,
                            has_dynamic_offset: false,
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
                    resource: buffer.as_entire_binding(),
                },
            ],
            label: Some("global variables bind group")
        });

        // write the shader header that will be used in the creation of the compute pipeline shaders
        // and store the names and the values of the globals that we will save in our struct.
        // Please note: we do this operation after the buffer init because this operation
        // consumes the input vectors.
        let mut shader_header = String::new();
        let mut names = Vec::<String>::new();
        let mut values = Vec::<f32>::new();

        shader_header += "layout(set = 1, binding = 0) uniform Uniforms {\n";
        // process all constants
        for (constant_name, _constant_value) in GLOBAL_CONSTANTS {
            shader_header += &format!("\tfloat {};\n", constant_name);
        }

        // process all variables
        let zipped_iterator = variables_names.into_iter().zip(init_values.into_iter());
        for pair in zipped_iterator {
            // if the name is not valid, just skip it!
            if !Self::valid_name(&pair.0) {
                continue;
            }
            // otherwise, print the name to the shader header and
            // add the pair to both the 'names' and the 'values' vectors
            shader_header += &format!("\tfloat {};\n", &pair.0);
            names.push(pair.0);
            values.push(pair.1);
        }
        shader_header += "};\n";

        //println!("debug info for shader header: {}", &shader_header);
        Self {
            bind_layout,
            bind_group,
            names,
            values,
            buffer,
            buffer_size,
            shader_header,
        }
    }

    pub fn update_buffer(&mut self, queue: &wgpu::Queue) {
        // quick check to make sure nobody changed the size of the values vector
        // which would spell disaster because then we would overwrite some random GPU memory
        assert!(self.names.len() == self.values.len());

        // update the mapped values in our buffer. Do not forget that this buffer
        // also contains all the global constants. Start copying from the computed offset!
        let offset = (GLOBAL_CONSTANTS.len() * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        queue.write_buffer(&self.buffer, offset, bytemuck::cast_slice(&self.values));
    }

}

