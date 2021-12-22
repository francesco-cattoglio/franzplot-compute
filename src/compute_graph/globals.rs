use crate::compute_graph::ProcessingError;
use crate::shader_processing::BindInfo;
use crate::parser::{parse_expression, AstNode, AstError};

#[derive(Debug)]
pub struct Globals {
    names: Vec<String>,
    values: Vec<f32>,
    buffer_size: wgpu::BufferAddress,
    buffer: wgpu::Buffer,
    pub bind_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub shader_header: String, // TODO: remove all the GLSL shader headers
    wgsl_header: String,
}

pub struct NameValuePair {
    pub name: String,
    pub value: f32,
}

pub const GLOBAL_CONSTANTS: &[(&str, f32)] = &[
    ("pi", std::f32::consts::PI),
    ("zero", 0.0),
];
const MAX_NUM_VARIABLES: usize = 31;

impl Globals {
    pub fn clone_names_values(&self) -> Vec<NameValuePair> {
        self.names.iter()
            .zip(self.values.iter())
            .map(|(name, value)| NameValuePair {
                name: name.clone(),
                value: *value,
            })
            .collect()
    }

    pub fn get_wgsl_header(&self) -> &str {
        self.wgsl_header.as_str()
    }

    pub fn get_buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn sanitize_variable_name(name: &str) -> Result<String, ProcessingError> {
        let parsing_result = parse_expression(name);
        match parsing_result {
            Ok(ast_tree) => {
                // it is not enough to parse the expression correctly. We must be sure that
                // the expression is JUST a single ident, and that ident is not the same as an
                // existing constant.
                match ast_tree {
                    AstNode::Ident(ident) => {
                        for constant in GLOBAL_CONSTANTS.iter() {
                            if constant.0 == ident {
                                return Err(ProcessingError::IncorrectExpression("cannot use a mathematical constant as a variable name".into()));
                            }
                            if constant.0.starts_with('_') {
                                return Err(ProcessingError::IncorrectExpression("variable names cannot start with an underscore".into()));
                            }
                        }
                        Ok(ident)
                    }
                    _ => Err(ProcessingError::IncorrectExpression("cannot use an expression as variable name".into())),
                }
            },
            Err(_err) => Err(ProcessingError::IncorrectExpression("invalid variable name".into())),
        }
    }

    // TODO: rename this and remove the other one once the conversion to the new compute_graph is done
    pub fn sanitize_expression(&self, local_params: &[&str], expression: &str) -> Result<String, ProcessingError> {
        let parsing_result = parse_expression(expression);
        match parsing_result {
            Ok(ast_tree) => {
                // the expression parsed correctly, but now we need to check if all the identifiers it
                // contains actually exist.
                let all_idents = ast_tree.find_all_idents();
                'validate: for ident in all_idents.into_iter() {
                    // if the ident is inside the variable names, we are good.
                    if self.names.contains(&ident) {
                        continue 'validate;
                    }
                    // if the ident is inside the global constants, we are good
                    for constant in GLOBAL_CONSTANTS.iter() {
                        if constant.0 == ident {
                            continue 'validate;
                        }
                    }
                    // if the ident is one of the parameters taken as input by the node, we are also good.
                    for param in local_params.iter() {
                        if param == &ident {
                            continue 'validate;
                        }
                    }

                    // OTHERWISE, write down an error!
                    let err = format!("Unknown variable or parameter used: '{}'", ident);
                    return Err(ProcessingError::IncorrectExpression(err));
                }
                Ok(ast_tree.to_string(&self.names))
            },
            Err(ast_error) => Err(Self::ast_to_block_error(ast_error)),
        }
    }

    fn ast_to_block_error(error: AstError) -> ProcessingError {
        match error {
            AstError::UnreachableMatch(e) => ProcessingError::InternalError(e),
            AstError::InternalError(e) => ProcessingError::InternalError(e),
            AstError::InvalidCharacter(e) => ProcessingError::IncorrectExpression(e),
            AstError::PowAmbiguity(e) => ProcessingError::IncorrectExpression(e),
            AstError::ImplicitProduct(e) => ProcessingError::IncorrectExpression(e),
            AstError::MultipleSigns(e) => ProcessingError::IncorrectExpression(e),
            AstError::MultipleOps(e) => ProcessingError::IncorrectExpression(e),
            AstError::MultipleExpressions(e) => ProcessingError::IncorrectExpression(e),
            AstError::FailedParse(e) => ProcessingError::IncorrectExpression(e),
            AstError::MissingParenthesis(e) => ProcessingError::IncorrectExpression(e),
            AstError::EmptyExpression(e) => ProcessingError::IncorrectExpression(e),
            AstError::InvalidName(e) => ProcessingError::IncorrectExpression(e),
        }
    }


    pub fn new(device: &wgpu::Device, variables_names: &[String], init_values: &[f32]) -> Self {
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
        init_vec.extend_from_slice(init_values);

        // create the actual buffer, the bind layout and the bind group
        use wgpu::util::DeviceExt;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("globals buffer"),
            contents: bytemuck::cast_slice(&init_vec),
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM
        });
        let bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        count: None,
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
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
        let mut wgsl_header = String::new();
        let mut names = Vec::<String>::new();
        let mut values = Vec::<f32>::new();

        //// in wgsl, the constants go outside of all functions
        //for (constant_name, constant_value) in GLOBAL_CONSTANTS {
        //    wgsl_header += &format!("let {}: f32 = {:?};\n", constant_name, constant_value);
        //}
        wgsl_header += "[[block]] struct Globals {\n";

        // is glsl, we put those constants inside the global variables
        shader_header += "layout(set = 1, binding = 0) uniform Uniforms {\n";
        for (constant_name, _constant_value) in GLOBAL_CONSTANTS {
            shader_header += &format!("\tfloat {};\n", constant_name);
            wgsl_header += &format!("\t{}: f32;\n", constant_name);
        }

        // process all variables
        let zipped_iterator = variables_names.iter().zip(init_values.iter());
        for pair in zipped_iterator {
            // print the name to the shader header and
            // add the pair to both the 'names' and the 'values' vectors
            shader_header += &format!("\tfloat {};\n", &pair.0);
            wgsl_header += &format!("\t{}: f32;\n", &pair.0);
            names.push(pair.0.clone());
            values.push(*pair.1);
        }
        shader_header += "};\n";
        // when we close the wgsl struct, we also need to write the binding to the group 1
        wgsl_header += "};\n";
        wgsl_header += "[[group(0), binding(0)]] var<uniform> globals: Globals;\n";


        //println!("debug info for shader header: {}", &shader_header);
        Self {
            bind_layout,
            bind_group,
            names,
            values,
            buffer,
            buffer_size,
            shader_header,
            wgsl_header,
        }
    }

    /// Updates the buffer containing all global variables.
    /// If none of the globals changed, then this function does nothing and returns false.
    /// Otherwise, if at least one global var changed, then the wgpu buffers is updated
    /// and the function returns true
    pub fn update_buffer(&mut self, queue: &wgpu::Queue, pairs: Vec<NameValuePair>) -> bool {
        // quick check to make sure nobody changed the size of the values vector
        // which would spell disaster because then we would overwrite some random GPU memory
        assert!(self.names.len() == self.values.len());

        let mut values_changed = false;
        let zipped = self.names.iter().zip(self.values.iter_mut());
        for (name, old_value) in zipped {
            // search for the pair that has the same name of the value that we want to update
            if let Some(pair) = pairs.iter().find(|e| &e.name == name) {
                // direct comparison of floats is ok in this case: those are not results of
                // random computations, those are values taken from imgui interface.
                #[allow(clippy::float_cmp)]
                if *old_value != pair.value {
                    *old_value = pair.value;
                    values_changed = true;
                }
            }
        }

        if values_changed {
            // values did in fact change. Update the buffer and overwrite the "old_values"
            // When updating the mapped values in our buffer, do not forget that this buffer
            // also contains all the global constants. Start copying from the computed offset!
            let offset = (GLOBAL_CONSTANTS.len() * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
            queue.write_buffer(&self.buffer, offset, bytemuck::cast_slice(&self.values));
        }

        values_changed
    }

    pub fn get_bind_info(&self) -> crate::shader_processing::BindInfo {
        BindInfo {
            buffer: &self.buffer,
            ty: wgpu::BufferBindingType::Uniform,
        }
    }
}

