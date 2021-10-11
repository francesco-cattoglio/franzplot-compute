use std::collections::BTreeMap;
use crate::rendering::model::Model;
pub use crate::node_graph::{NodeGraph, NodeID, NodeContents};
use crate::computable_scene::globals::Globals;

mod interval;
mod curve;
mod geometry_render;

pub type DataID = i32;
pub type PrefabId = i32;
#[derive(Clone, Debug)]
pub struct UnrecoverableError(NodeID, &'static str);

pub struct Operation {
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
    dim: [u32; 3],
}

impl Operation {
    pub fn encode(&self, variables_bind_group: &wgpu::BindGroup, encoder: &mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{
            label: Some("rendering compute pass")
        });
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.dispatch(self.dim[0], self.dim[1], self.dim[2]);
    }
}

#[derive(Debug, Clone)]
pub enum ProcessingError {
    InputMissing(&'static str),
    InternalError(String),
    IncorrectAttributes(&'static str),
    IncorrectExpression(String),
}
pub type ProcessingResult = Result<(BTreeMap<DataID, Data>, Operation), ProcessingError>;

// a parameter can be anonymous, e.g. when created by a Bezier node
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: Option<String>,
    pub size: usize,
    pub begin: String,
    pub end: String,
    pub use_interval_as_uv: bool,
}

impl Parameter {
    pub fn is_equal(&self, other: &Parameter) -> Result <bool, ProcessingError> {
        match (&self.name, &other.name) {
            (None, None) => Ok(false),
            (None, Some(_)) => Ok(false),
            (Some(_), None) => Ok(false),
            (Some(self_name), Some(other_name)) => {
                if self_name == other_name {
                    // having the same name but a different quality, begin or end attribute is an error.
                    if self.size != other.size {
                        Err(ProcessingError::IncorrectAttributes("The input intervals \n have the same name \n but different 'quality' "))
                    } else if self.begin != other.begin {
                        Err(ProcessingError::IncorrectAttributes("The input intervals \n have the same name \n but different 'begin' "))
                    } else if self.end != other.end {
                        Err(ProcessingError::IncorrectAttributes(" The input intervals \n have the same name \n but different 'end' "))
                    } else {
                        Ok(true)
                    }
                } else {
                    Ok(false)
                }

            }
        }
    }
}

// a data node only contains GPU buffers that are manipulated by OperationNodes
pub enum Data {
    Interval{
        buffer: wgpu::Buffer,
        param: Parameter,
    },
    Geom1D {
        buffer: wgpu::Buffer,
        param: Parameter,
    },
    Geom2D {
    },
    Matrix0D {
    },
    Matrix1D {
    },
    Prefab {
        vertex_buffer: wgpu::Buffer,
        index_buffer: wgpu::Buffer,
    },
    NotComputed,
}

pub struct MatcapData {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub mask_id: usize,
    pub material_id: usize,
    pub graph_node_id: NodeID,
}
// The compute graph contains:
// - a map of all the Data in the graph
// - a list of all the operations that are to be executed (that also implies: all the compute
// shaders that are to be run)
// - a list of all the renderables that were created as outputs.
pub struct ComputeGraph {
    renderables: Vec<MatcapData>,
    data: BTreeMap<DataID, Data>,
    operations: Vec<Operation>,
}

impl ComputeGraph {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            operations: Vec::new(),
            renderables: Vec::new(),
        }
    }

    // This function fails if many BlockDescriptors share the same BlockId or if
    // there is a circular dependency between all the blocks.
    pub fn process_graph(&mut self, device: &wgpu::Device, models: &[Model], globals: &Globals, graph: &NodeGraph) -> Result<Vec<(NodeID, ProcessingError)>, UnrecoverableError> {
        // TODO: maybe rewrite this part, it looks like it is overcomplicated.
        // compute a map from BlockId to descriptor data and
        // a map from BlockId to all the inputs that a block has
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

        // Since we declared that the input of a node is the successor of the node, the ids are sorted
        // with the rendering commands first and the intervals last.
        // Therefore we process the descriptors in the reversed order
        let mut recoverable_errors = Vec::<(NodeID, ProcessingError)>::new();
        for id in sorted_ids.into_iter().rev() {
            let node_result = self.process_single_node(device, globals, id, graph);
            if let Err(error) = node_result {
                recoverable_errors.push((id, error));
            };
        }
        Ok(recoverable_errors)
    }

    // process a single graph node, creating some data and one operation
    pub fn process_single_node(&mut self, device: &wgpu::Device, globals: &Globals, graph_node_id: NodeID, graph: &NodeGraph) ->  Result<(), ProcessingError> {
        // TODO: turn this into an if let - else construct
        let to_process = match graph.get_node(graph_node_id) {
            Some(node) => node,
            None => return Err(ProcessingError::InternalError("Node not found".into())),
        };
        match *to_process.contents() {
            NodeContents::Curve {
                interval, fx, fy, fz, output
            } => {
                let (mut new_data, operation) = curve::create(
                    device,
                    globals,
                    &self.data,
                    graph.get_attribute_as_linked_output(interval),
                    graph.get_attribute_as_string(fx).unwrap(),
                    graph.get_attribute_as_string(fy).unwrap(),
                    graph.get_attribute_as_string(fz).unwrap(),
                    output,
                )?;
                self.data.append(&mut new_data);
                self.operations.push(operation);
            },
            NodeContents::Interval {
                variable, begin, end, quality, output,
            } => {
                let (mut new_data, operation) = interval::create(
                    device,
                    globals,
                    graph.get_attribute_as_string(variable).unwrap(),
                    graph.get_attribute_as_string(begin).unwrap(),
                    graph.get_attribute_as_string(end).unwrap(),
                    graph.get_attribute_as_usize(quality).unwrap(),
                    output
                )?;
                self.data.append(&mut new_data);
                self.operations.push(operation);
            },
            NodeContents::Rendering {
                geometry, thickness, mask, material,
            } => {
                let (renderable, operation) = geometry_render::create(
                    device,
                    &self.data,
                    graph.get_attribute_as_linked_output(geometry),
                    graph.get_attribute_as_usize(thickness).unwrap(),
                    graph_node_id,
                )?;
                self.renderables.push(renderable);
                self.operations.push(operation);
            },
            _ => todo!("handle all graph node kinds!")
        }
        Ok(())
    }

    pub fn matcaps<'a>(&'a self) -> impl ExactSizeIterator<Item = &'a MatcapData> {
        self.renderables.iter()
    }

    pub fn run_compute(&self, device: &wgpu::Device, queue: &wgpu::Queue, globals: &Globals) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder this time"),
        });
        for op in self.operations.iter() {
            op.encode(&globals.bind_group, &mut encoder);
        }
        let compute_queue = encoder.finish();
        queue.submit(std::iter::once(compute_queue));

        if let Some(renderable) = self.renderables.first() {
            //let contents = crate::util::copy_buffer_as::<f32>(&renderable.vertex_buffer, device);
            let contents = crate::util::copy_buffer_as::<i32>(&renderable.index_buffer, device);
            dbg!(&contents);
        }

    }


}
