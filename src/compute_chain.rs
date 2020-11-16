use crate::compute_block::*;

use std::collections::BTreeMap;

use wgpu::util::DeviceExt;

pub struct ComputeChain {
    processed_blocks: indexmap::IndexMap<BlockId, Result<ComputeBlock, BlockCreationError>>,
}

#[derive(Debug)]
pub struct Globals {
    pub names: Vec<String>,
    pub values: Vec<f32>,
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

// TODO: maybe this could just be a field in the BlockCreationError enum,
// instead of its own type
struct UnrecoverableError(BlockId, &'static str);

impl<'a> ComputeChain {
    pub fn new() -> Self {
        Self {
            processed_blocks: indexmap::IndexMap::new(),
        }
    }

    // This function currently modifies the internal state of the compute chain and reports any
    // error that happened at ComputeBlock creation time
    pub fn set_scene(&mut self, device: &wgpu::Device, globals: &Globals, descriptors: Vec<BlockDescriptor>) -> Vec<(BlockId, BlockCreationError)> {
        // create a list of errors to be reported
        let mut error_list = Vec::<(BlockId, BlockCreationError)>::new();

        // process descriptors into actual compute blocks
        let process_result = Self::process_descriptors(device, globals, descriptors);

        // TODO: we might decide _not_ to replace the current compute chain if processed_map
        // contained errors, or by some other logic.
        match process_result {
            Ok(processed_map) => {
                self.processed_blocks = processed_map;

                // TODO: maybe we could use the "ComputeChain::invalid_blocks" function?
                for (block_id, result) in self.processed_blocks.iter() {
                    if let Err(error) = result {
                        error_list.push((*block_id, error.clone()));
                    }
                }
            }
            Err(error) => {
                error_list.push((error.0, BlockCreationError::InputInvalid(error.1)));
            }
        }

        error_list
    }

    // consumes the input Vec<BlockDescriptor> and processes each one of them, turning it into a
    // ComputeBlock. This function fails if many BlockDescriptors share the same BlockId or if
    // there is a circular dependency between all the blocks.
    fn process_descriptors(device: &wgpu::Device, globals: &Globals, descriptors: Vec<BlockDescriptor>) -> Result<ProcessedMap, UnrecoverableError> {
        // TODO: maybe rewrite this part, it looks like it is overcomplicated.
        // compute a map from BlockId to descriptor data and
        // a map from BlockId to all the inputs that a block has
        let mut descriptor_inputs = BTreeMap::<BlockId, Vec<BlockId>>::new();
        let mut descriptor_data = BTreeMap::<BlockId, DescriptorData>::new();
        for descriptor in descriptors.into_iter() {
            descriptor_inputs.insert(descriptor.id, descriptor.data.get_input_ids());
            descriptor_data.insert(descriptor.id, descriptor.data);
            // TODO: we should also error out here if we find out that two block descriptors have
            // the same BlockId
        }

        // copy a list of block ids and use the following lambda to run the topological sort
        let descriptor_ids: Vec<BlockId> = descriptor_inputs.keys().cloned().collect();
        let successor_function = | id: &BlockId | -> Vec<BlockId> {
            descriptor_inputs.remove(id).unwrap_or_default()
        };
        let sorting_result = pathfinding::directed::topological_sort::topological_sort(&descriptor_ids, successor_function);

        // This function fails if a cycle in the graph is detected.
        // If it happens, return a UnrecoverableError.
        let sorted_ids = sorting_result.map_err(|block_id: BlockId| { UnrecoverableError(block_id, " cycle detected \n at this node ") })?;

        let mut processed_blocks = ProcessedMap::new();
        // Since we declared that the input of a node is the successor of the node, the ids are sorted
        // having the rendering commands first and the intervals last.
        // Therefore we process the descriptors in the reversed order
        for id in sorted_ids.into_iter().rev() {
            if let Some(descriptor) = descriptor_data.remove(&id) {
                let new_block = descriptor.to_block(device, globals, &processed_blocks);
                processed_blocks.insert(id, new_block);
            }
        }
        Ok(processed_blocks)
    }

    pub fn run_chain(&self, device: &wgpu::Device, queue: &wgpu::Queue, globals: &Globals) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder this time"),
        });
        for block in self.valid_blocks() {
            block.encode(&globals.bind_group, &mut encoder);
        }
        let compute_queue = encoder.finish();
        queue.submit(std::iter::once(compute_queue));
    }

    pub fn valid_blocks(&'a self) -> impl Iterator<Item = &'a ComputeBlock> {
        self.processed_blocks.values().filter_map(|elem| elem.as_ref().ok())
    }

    pub fn invalid_blocks(&'a self) -> impl Iterator<Item = &'a BlockCreationError> {
        self.processed_blocks.values().filter_map(|elem| elem.as_ref().err())
    }

}


