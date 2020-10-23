use maplit::btreemap;
use crate::compute_block::*;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use wgpu::util::DeviceExt;

pub struct ComputeChain {
    processed_blocks: indexmap::IndexMap<BlockId, Result<ComputeBlock, BlockCreationError>>,
    globals_buffer_size: wgpu::BufferAddress,
    globals_buffer: wgpu::Buffer,
    pub globals_bind_layout: wgpu::BindGroupLayout,
    pub globals_bind_group: wgpu::BindGroup,
    pub shader_header: String,
    pub global_vars: BTreeMap<String, f32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Context {
    pub globals: BTreeMap<String, f32>,
}

impl std::default::Default for Context {
    fn default() -> Self {
        Self {
            globals: btreemap!{
//                "t".to_string() => 0.0,
                "pi".to_string() => std::f32::consts::PI,
            },
        }
    }
}

const MAX_NUM_GLOBALS: usize = 32;

impl<'a> ComputeChain {
    pub fn new(device: &wgpu::Device) -> Self {

        let globals_buffer_size = (MAX_NUM_GLOBALS * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor{
            label: Some("globals buffer"),
            mapped_at_creation: false,
            size: globals_buffer_size,
            usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM
        });
        let globals_bind_layout =
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
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &globals_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer (
                        globals_buffer.slice(..),
                    ),
                },
            ],
            label: Some("variables bind group")
        });

        Self {
            globals_bind_layout,
            globals_bind_group,
            globals_buffer,
            globals_buffer_size,
            processed_blocks: indexmap::IndexMap::new(),
            global_vars: BTreeMap::new(),
            shader_header: String::new(),
        }
    }

    // TODO: set_globals and update_globals feel a bit off, they are similar yet different, maybe
    // think about changing the two functions
    pub fn set_globals(&mut self, queue: &wgpu::Queue, globals: &BTreeMap<String, f32>) {
        assert!(globals.len() <= MAX_NUM_GLOBALS);
        // update the stored names of the globals
        self.global_vars = globals.clone();
        // add some constants if they are missing
        self.global_vars.insert("pi".to_string(), std::f32::consts::PI);

        // store a copy of the mapped values in our buffer
        let values: Vec<f32> = globals.values().copied().collect();
        queue.write_buffer(&self.globals_buffer, 0, bytemuck::cast_slice(&values));
        // write the shader header that will be used in the creation of the compute pipeline shaders
        self.shader_header.push_str("layout(set = 1, binding = 0) uniform Uniforms {\n");
        for (var_name, var_value) in self.global_vars.iter() {
            self.shader_header.push_str(format!("\tfloat {};\n", var_name).as_str());
        }
        self.shader_header.push_str("};\n");
        //println!("debug info for shader header: {}", &shader_header);
    }

    // TODO: I would really, REALLY like for this function to consume the descriptor array. Would
    // simplify the process_descriptors function greatly
    // TODO: this still requires some work, the way this returns an Ok(()) or a list of error is
    // weird, maybe just a list of errors will do. After all, even if errors are returned, internal
    // state is still modified by the process_descriptors call.
    pub fn set_scene(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, context: &Context, descriptors: &Vec<BlockDescriptor>) -> Result<(), Vec<(BlockId, BlockCreationError)>> {
        assert!(context.globals.len() <= MAX_NUM_GLOBALS);
        // cleanup previously stored context, shader_header and blocks
        self.processed_blocks.clear();
        self.shader_header.clear();
        self.global_vars.clear();

        // now re-process the context
        self.set_globals(queue, &context.globals);

        // and process descriptors into actual compute blocks
        self.process_descriptors(device, descriptors);

        // TODO: also, this code is ugly
        let mut error_list = Vec::<(BlockId, BlockCreationError)>::new();
        for processed in self.processed_blocks.iter() {
            if let Err(error) = processed.1 {
                error_list.push((*processed.0, error.clone()));
            }
        }

        if error_list.is_empty() {
            Ok(())
        } else {
            Err(error_list)
        }
    }

    // TODO: evaluate the following:
    // it might be worth it to rewrite this function so that it does not take a &self and it does
    // not modify the compute_chain, instead returning the indexmap containing all the compute
    // blocks and all the errors. In order to do however you need to pass in _a_lot_ of arguements,
    // including the indexmap being modified, which is needed for lookup purposes inside
    // ComputeBlock "constructors"
    pub fn process_descriptors(&mut self, device: &wgpu::Device, descriptors: &Vec<BlockDescriptor>) {
        // TODO: maybe rewrite this part, it looks like it is overcomplicated.
        // compute a map from BlockId to references to all the descriptor data (this is necessary
        // because DescriptorData is borrowed and non-copiable)
        // and compute a map from BlockId to all the inputs that block has
        let mut descriptor_data = BTreeMap::<BlockId, &DescriptorData>::new();
        let mut descriptor_inputs = BTreeMap::<BlockId, Vec<BlockId>>::new();
        for descriptor in descriptors.iter() {
            descriptor_data.insert(descriptor.id, &descriptor.data);
            descriptor_inputs.insert(descriptor.id, descriptor.data.get_input_ids());
        }

        // compute a list of blocks and use the following lambda to run the topological sort
        let descriptor_ids: Vec<BlockId> = descriptor_inputs.keys().cloned().collect();
        let successor_function = | id: &BlockId | -> Vec<BlockId> {
            descriptor_inputs.remove(id).unwrap_or(Vec::<BlockId>::new())
        };
        let sorting_result = pathfinding::directed::topological_sort::topological_sort(&descriptor_ids, successor_function);

        // here we unwrap, but this function would fail if a cycle in the graph is detected.
        // It would be nice to return that as an error.
        let sorted_ids = sorting_result.unwrap();

        // Since we declared that the input of a node is the successor of the node, the ids are sorted
        // having the rendering commands first and the intervals last.
        // Therefore we process the descriptors in the reversed order
        for id in sorted_ids.into_iter().rev() {
            if let Some(descriptor) = descriptor_data.remove(&id) {
                let new_block = descriptor.to_block(&self, device);
                self.processed_blocks.insert(id, new_block);
            }
        }

    }

    pub fn run_chain(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder this time"),
        });
        for maybe_block in self.processed_blocks.values() {
            if let Ok(block) = maybe_block {
                block.encode(&self.globals_bind_group, &mut encoder);
            }
        }
        let compute_queue = encoder.finish();
        queue.submit(std::iter::once(compute_queue));
    }

    pub fn update_globals(&mut self, queue: &wgpu::Queue, list: &Vec<(String, f32)>) {
        // Update our global variables with the ones found in the passed in list.
        // The passed-in list might contain some variables that do not actually exist;
        // we just do nothing in that case.
        for (name, new_value) in list.iter() {
            if let Some(value) = self.global_vars.get_mut(name) {
                *value = *new_value;
            }
        }
        // update the mapped values in our buffer
        let values: Vec<f32> = self.global_vars.values().copied().collect();
        queue.write_buffer(&self.globals_buffer, 0, bytemuck::cast_slice(&values));
    }

    pub fn create_from_descriptors(device: &wgpu::Device, queue: &wgpu::Queue, context: &Context, descriptors: &Vec<BlockDescriptor>) -> Result<(), Vec<(BlockId, BlockCreationError)>> {
        let mut chain = Self::new(device);
        chain.set_scene(device, queue, context, descriptors)
    }

    pub fn get_block(&'a self, id: &BlockId) -> Option<&'a Result<ComputeBlock, BlockCreationError>> {
        self.processed_blocks.get(id)
    }

    pub fn blocks_iterator(&'a self) -> impl Iterator<Item = &'a ComputeBlock> {
        self.processed_blocks.values().filter_map(|elem| elem.as_ref().ok())
    }

}


