use super::compute_block::*;
use super::globals::Globals;
use crate::node_graph::{ NodeGraph, NodeID };

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
    pub fn scene_from_graph(&mut self, device: &wgpu::Device, globals: &Globals, graph: &NodeGraph) -> Vec<(NodeID, BlockCreationError)> {
        // create a list of errors to be reported
        let mut error_list = Vec::<(BlockId, BlockCreationError)>::new();

        // process descriptors into actual compute blocks
        let process_result = Self::process_graph(device, globals, graph);

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

    // This function fails if many BlockDescriptors share the same BlockId or if
    // there is a circular dependency between all the blocks.
    fn process_graph(device: &wgpu::Device, globals: &Globals, graph: &NodeGraph) -> Result<ProcessedMap, UnrecoverableError> {
        // TODO: maybe rewrite this part, it looks like it is overcomplicated.
        // compute a map from BlockId to descriptor data and
        // a map from BlockId to all the inputs that a block has
        use std::collections::BTreeMap;
        let mut node_inputs = BTreeMap::<NodeID, Vec<NodeID>>::new();
        for (node_id, node) in graph.get_nodes() {
            let existing_inputs: Vec<NodeID> = node.get_input_nodes(graph);
            node_inputs.insert(node_id, existing_inputs);
            // TODO: we should also error out here if we find out that two block descriptors have
            // the same BlockId
        }

        // copy a list of block ids and use the following lambda to run the topological sort
        let node_ids: Vec<NodeID> = node_inputs.keys().cloned().collect();
        let successor_function = | id: &NodeID | -> Vec<NodeID> {
            node_inputs.remove(id).unwrap_or_default()
        };
        let sorting_result = pathfinding::directed::topological_sort::topological_sort(&node_ids, successor_function);

        // This function fails if a cycle in the graph is detected.
        // If it happens, return a UnrecoverableError.
        let sorted_ids = sorting_result.map_err(|node_id: NodeID| { UnrecoverableError(node_id, " cycle detected \n at this node ") })?;

        let mut processed_blocks = ProcessedMap::new();
        // Since we declared that the input of a node is the successor of the node, the ids are sorted
        // with the rendering commands first and the intervals last.
        // Therefore we process the descriptors in the reversed order
        for id in sorted_ids.into_iter().rev() {
            if let Some(node) = graph.get_node(id) {
                let block_result = ComputeBlock::from_node(device, globals, &processed_blocks, node, graph);
                processed_blocks.insert(id, block_result);
            }
        }
        Ok(processed_blocks)
    }

    pub fn run_chain(&self, device: &wgpu::Device, queue: &wgpu::Queue, globals: &Globals) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder this time"),
        });
        for (_id, block) in self.valid_blocks() {
            block.encode(&globals.bind_group, &mut encoder);
        }
        let compute_queue = encoder.finish();
        queue.submit(std::iter::once(compute_queue));
    }

    pub fn valid_blocks(&'a self) -> impl Iterator<Item = (&BlockId, &'a ComputeBlock)> {
        self.processed_blocks.iter().filter_map(|(id, elem)| Some((id, elem.as_ref().ok()?)))
    }
}


