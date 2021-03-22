
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
const KEYWORDS: &[&str] = &[
    "sin", "cos", "tan",
    "asin", "acos", "atan",
    "int", "float", "double",
];
const MAX_NUM_VARIABLES: usize = 31;

impl Globals {
    /// This function sanitizes a variable name. If the variable name could be sanitized,
    /// e.g. by removing extra spaces, then it is returned as Some(name), otherwise if the
    /// name was invalid (e.g: was a keyword) None is returned
    // TODO: we might want to turn this into a result, so that we can display something
    // back to the user instead of just println!() it
    pub fn sanitize_variable_name(name: &str) -> Option<&str> {
        // Make sure that the name does not contain any internal whitespace
        if name.split_whitespace().count() == 0 {
            println!("Variables name cannot be empty");
            return None;
        }
        if name.split_whitespace().count() > 1 {
            println!("Variables cannot contain spaces; {} is not a valid name", name);
            return None;
        }
        // and then strip leading and trailing spaces, leaving only the actual name
        let name = name.trim_start().trim_end();

        // Check if the first character exists, and return null if it is not alphabetic.
        let mut chars_iter = name.chars();
        if !chars_iter.next()?.is_ascii_alphabetic() {
            println!("The first character in a variable must be a letter; {} is not a valid name", name);
            return None;
        }

        // Also check all the other characters, there shall only be letters, numbers, underscores
        while let Some(character) = chars_iter.next() {
            if !character.is_ascii_alphanumeric() && character != '_' {
                println!("Only letters, numbers and underscores are allowed in variables; {} is not a valid name", name);
                return None;
            }
        }

        // the name should now be compared to global constants and keywords, and be rejected if any
        // one matches
        for (constant_name, _value) in GLOBAL_CONSTANTS {
            if name == *constant_name {
                // TODO: this should be logged in as warning!
                println!("Warning, invalid variable name used: {} is reserved", name);
                return None;
            }
        }
        for keyword in KEYWORDS {
            if name == *keyword {
                // TODO: this should be logged in as warning!
                println!("Warning, invalid variable name used: {} is reserved", name);
                return None;
            }
        }

        Some(name)
    }

    pub fn sanitize_expression(expression: &str) -> Option<&str> {
        // strip leading and trailing spaces.
        let expression = expression.trim_start().trim_end();

        // Check all the characters. Only ascii characters are allowed, and
        // anything that contains a semicolon or an equal should be rejected.
        // Otherwise the user could badly mess up the shader code.
        // We also disable the '^' symbol because it was used on older versions of franzplot
        // but it has a different meaning in GLSL
        for character in expression.chars() {
            if !character.is_ascii()
                || character == '^'
                || character == ';'
                || character == '=' {
                println!("Warning, invalid variable name used: {}", expression);
                return None;
            }
        }

        Some(expression)
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
            // assert that all variable names have been sanitizied already.
            assert!(Self::sanitize_variable_name(&pair.0).is_some());

            // print the name to the shader header and
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

