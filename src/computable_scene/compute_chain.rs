use super::compute_block::*;
use super::globals::Globals;

pub struct ComputeChain {
    processed_blocks: indexmap::IndexMap<BlockId, Result<ComputeBlock, BlockCreationError>>,
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
        use std::collections::BTreeMap;
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


