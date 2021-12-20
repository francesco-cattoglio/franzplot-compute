use std::collections::BTreeMap;
use std::collections::btree_map::Iter;
use std::rc::Rc;
use indexmap::IndexMap;
use crate::rendering::model::Model;
pub use crate::node_graph::{NodeGraph, NodeID, NodeContents};
use crate::computable_scene::globals::{Globals, NameValuePair};
use crate::state::UserState;
use crate::state::Assets;

mod point;
mod vector;
mod interval;
mod curve;
mod bezier;
mod geometry_render;
mod vector_render;
mod surface;
mod matrix;
mod transform;
mod sample;
mod prefab;
mod plane;

pub type DataID = i32;
pub type PrefabId = i32;
#[derive(Clone, Debug)]
pub struct UnrecoverableError {
    node_id: NodeID,
    error: &'static str,
}
#[derive(Clone, Debug)]
pub struct RecoverableError {
    node_id: NodeID,
    error: ProcessingError,
}

pub struct Operation {
    bind_group: wgpu::BindGroup,
    pipeline: Rc<wgpu::ComputePipeline>,
    dim: [u32; 3],
}

impl Operation {
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
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
    NoInputData,
    InternalError(String),
    IncorrectAttributes(&'static str),
    IncorrectExpression(String),
    IncorrectInput(&'static str),
}
pub type SingleDataResult = Result<(Data, Operation), ProcessingError>;
pub type MatcapIter<'a> = Iter<'a, NodeID, MatcapData>;

// a parameter can be anonymous, e.g. when created by a Bezier node
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: Option<String>,
    pub segments: u32, // this is u32 because it is moslty used by the compute dispatch ops
    pub begin: String,
    pub end: String,
    pub use_interval_as_uv: bool,
}

impl Parameter {
    pub const POINTS_PER_SEGMENT: usize = 16;

    pub fn n_points(&self) -> usize {
        self.segments as usize * Self::POINTS_PER_SEGMENT
    }

    pub fn is_equal(&self, other: &Parameter) -> Result <bool, ProcessingError> {
        match (&self.name, &other.name) {
            (None, None) => Ok(false),
            (None, Some(_)) => Ok(false),
            (Some(_), None) => Ok(false),
            (Some(self_name), Some(other_name)) => {
                if self_name == other_name {
                    // having the same name but a different quality, begin or end attribute is an error.
                    if self.segments != other.segments {
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
    Vector {
        buffer: wgpu::Buffer,
    },
    Interval {
        buffer: wgpu::Buffer,
        param: Parameter,
    },
    Geom0D {
        buffer: wgpu::Buffer,
    },
    Geom1D {
        buffer: wgpu::Buffer,
        param: Parameter,
    },
    Geom2D {
        buffer: wgpu::Buffer,
        param1: Parameter,
        param2: Parameter,
    },
    Matrix0D {
        buffer: wgpu::Buffer,
    },
    Matrix1D {
        buffer: wgpu::Buffer,
        param: Parameter,
    },
    Prefab {
        vertex_buffer: wgpu::Buffer,
        chunks_count: usize,
        index_buffer: Rc<wgpu::Buffer>,
        index_count: u32,
    },
}

pub struct MatcapData {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: Rc<wgpu::Buffer>,
    pub index_count: u32,
    pub mask_id: usize,
    pub material_id: usize,
}
// The compute graph contains:
// - a map of all the Data in the graph
// - a list of all the operations that are to be executed (that also implies: all the compute
// shaders that are to be run)
// - a list of all the renderables that were created as outputs.
pub struct ComputeGraph {
    pub globals: Globals,
    data: BTreeMap<DataID, Data>,
    renderables: BTreeMap<NodeID, MatcapData>,
    operations: IndexMap<NodeID, Operation>,
}

pub fn create_compute_graph(device: &wgpu::Device, assets: &Assets, user_state: UserState) -> Result<(ComputeGraph, Vec<RecoverableError>), UnrecoverableError> {
        // compute a map from BlockId to descriptor data and
        // a map from BlockId to all the inputs that a block has
        let mut node_inputs = BTreeMap::<NodeID, Vec<NodeID>>::new();
        let graph = &user_state.graph;
        let globals = Globals::new(device, user_state.globals_names, user_state.globals_init_values);
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
        let sorted_ids = sorting_result.map_err(
            |node_id: NodeID| {
                UnrecoverableError {
                    node_id,
                    error: " cycle detected \n at this node "
                }
            }
        )?;

        // Since we declared that the input of a node is the successor of the node, the ids are sorted
        // with the rendering commands first and the intervals last.
        // Therefore we process the descriptors in the reversed order
        let mut recoverable_errors = Vec::<RecoverableError>::new();
        let mut compute_graph = ComputeGraph {
            data: BTreeMap::new(),
            operations: IndexMap::new(),
            renderables: BTreeMap::new(),
            globals,
        };
        for id in sorted_ids.into_iter().rev() {
            let node_result = compute_graph.process_single_node(device, assets, id, graph);
            if let Err(error) = node_result {
                recoverable_errors.push(RecoverableError{
                    node_id: id,
                    error,
                });
            };
        }
        Ok((compute_graph, recoverable_errors))
}

impl ComputeGraph {
    pub fn matcaps(&self) -> MatcapIter {
        self.renderables.iter()
    }

    pub fn run_compute(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder this time"),
        });
        for op in self.operations.values() {
            op.encode(&mut encoder);
        }
        let compute_queue = encoder.finish();
        queue.submit(std::iter::once(compute_queue));
    }

    pub fn update_globals(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, pairs: Vec<NameValuePair>) {
        let values_changed = self.globals.update_buffer(queue, pairs);
        if values_changed {
            self.run_compute(device, queue);
        }
    }

    // process a single graph node.
    // If the operation is successful, then the internal state of the ComputeGraph is modified by storing
    // the newly created data and operation. If it fails, then a ProcessingError is returned and
    // the internal state is left untouched.
    fn process_single_node(&mut self, device: &wgpu::Device, assets: &Assets, graph_node_id: NodeID, graph: &NodeGraph) ->  Result<(), ProcessingError> {
        // TODO: turn this into an if let - else construct
        let to_process = match graph.get_node(graph_node_id) {
            Some(node) => node,
            None => return Err(ProcessingError::InternalError("Node not found".into())),
        };
        match *to_process.contents() {
            NodeContents::Vector {
                x, y, z, output
            } => {
                let (new_data, operation) = vector::create(
                    device,
                    &self.globals,
                    graph.get_attribute_as_string(x).unwrap(),
                    graph.get_attribute_as_string(y).unwrap(),
                    graph.get_attribute_as_string(z).unwrap(),
                )?;
                self.data.insert(output, new_data);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::Point {
                x, y, z, output
            } => {
                let (new_data, operation) = point::create(
                    device,
                    &self.globals,
                    graph.get_attribute_as_string(x).unwrap(),
                    graph.get_attribute_as_string(y).unwrap(),
                    graph.get_attribute_as_string(z).unwrap(),
                )?;
                self.data.insert(output, new_data);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::Curve {
                interval, fx, fy, fz, output
            } => {
                let (new_data, operation) = curve::create(
                    device,
                    &self.globals,
                    &self.data,
                    graph.get_attribute_as_linked_output(interval),
                    graph.get_attribute_as_string(fx).unwrap(),
                    graph.get_attribute_as_string(fy).unwrap(),
                    graph.get_attribute_as_string(fz).unwrap(),
                )?;
                self.data.insert(output, new_data);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::Surface {
                interval_1, interval_2, fx, fy, fz, output,
            } => {
                let (new_data, operation) = surface::create(
                    device,
                    &self.globals,
                    &self.data,
                    graph.get_attribute_as_linked_output(interval_1),
                    graph.get_attribute_as_linked_output(interval_2),
                    graph.get_attribute_as_string(fx).unwrap(),
                    graph.get_attribute_as_string(fy).unwrap(),
                    graph.get_attribute_as_string(fz).unwrap(),
                )?;
                self.data.insert(output, new_data);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::Bezier {
                p0, p1, p2, p3, quality, output
            } => {
                let mut points = Vec::<NodeID>::new();
                if let Some(id) = graph.get_attribute_as_linked_output(p0) {
                    points.push(id);
                }
                if let Some(id) = graph.get_attribute_as_linked_output(p1) {
                    points.push(id);
                }
                if let Some(id) = graph.get_attribute_as_linked_output(p2) {
                    points.push(id);
                }
                if let Some(id) = graph.get_attribute_as_linked_output(p3) {
                    points.push(id);
                }
                let (new_data, operation) = bezier::create(
                    device,
                    &self.data,
                    points,
                    graph.get_attribute_as_usize(quality).unwrap(),
                )?;
                self.data.insert(output, new_data);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::Interval {
                variable, begin, end, quality, output,
            } => {
                let (new_data, operation) = interval::create(
                    device,
                    &self.globals,
                    graph.get_attribute_as_string(variable).unwrap(),
                    graph.get_attribute_as_string(begin).unwrap(),
                    graph.get_attribute_as_string(end).unwrap(),
                    graph.get_attribute_as_usize(quality).unwrap(),
                )?;
                self.data.insert(output, new_data);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::TranslationMatrix {
                vector, output,
            } => {
                let (new_data, operation) = matrix::create_from_translation(
                    device,
                    &self.data,
                    graph.get_attribute_as_linked_output(vector),
                    )?;
                self.data.insert(output, new_data);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::RotationMatrix {
                axis, angle, output,
            } => {
                let (new_data, operation) = matrix::create_from_rotation(
                    device,
                    &self.globals,
                    &self.data,
                    graph.get_attribute_as_axis(axis).unwrap(),
                    graph.get_attribute_as_string(angle).unwrap(),
                    )?;
                self.data.insert(output, new_data);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::Matrix {
                interval, row_1, row_2, row_3, output,
            } => {
                let (new_data, operation) = matrix::create_from_rows(
                    device,
                    &self.globals,
                    &self.data,
                    graph.get_attribute_as_linked_output(interval),
                    graph.get_attribute_as_matrix_row(row_1).unwrap(),
                    graph.get_attribute_as_matrix_row(row_2).unwrap(),
                    graph.get_attribute_as_matrix_row(row_3).unwrap(),
                    )?;
                self.data.insert(output, new_data);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::Sample {
                geometry, parameter, value, output,
            } => {
                let (new_data, operation) = sample::create(
                    device,
                    &self.globals,
                    &self.data,
                    graph.get_attribute_as_linked_output(geometry),
                    graph.get_attribute_as_string(parameter).unwrap(),
                    graph.get_attribute_as_string(value).unwrap(),
                    )?;
                self.data.insert(output, new_data);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::Transform {
                geometry, matrix, output,
            } => {
                let (new_data, operation) = transform::create(
                    device,
                    &self.data,
                    graph.get_attribute_as_linked_output(geometry),
                    graph.get_attribute_as_linked_output(matrix),
                    )?;
                self.data.insert(output, new_data);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::Plane {
                center, normal, size, output,
            } => {
                let (new_data, operation) = plane::create(
                    device,
                    &self.data,
                    graph.get_attribute_as_linked_output(center),
                    graph.get_attribute_as_linked_output(normal),
                    graph.get_attribute_as_usize(size).unwrap(),
                    )?;
                self.data.insert(output, new_data);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::Primitive {
                primitive, size, output,
            } => {
                let (new_data, operation) = prefab::create(
                    device,
                    &assets.models,
                    &self.globals,
                    graph.get_attribute_as_usize(primitive).unwrap(),
                    graph.get_attribute_as_string(size).unwrap(),
                    )?;
                self.data.insert(output, new_data);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::Rendering {
                geometry, thickness, mask, material,
            } => {
                let (renderable, operation) = geometry_render::create(
                    device,
                    &self.data,
                    graph.get_attribute_as_linked_output(geometry),
                    graph.get_attribute_as_usize(thickness).unwrap(),
                    graph.get_attribute_as_usize(mask).unwrap(),
                    graph.get_attribute_as_usize(material).unwrap(),
                )?;
                self.renderables.insert(graph_node_id, renderable);
                self.operations.insert(graph_node_id, operation);
            },
            NodeContents::VectorRendering {
                application_point, vector, thickness, material,
            } => {
                let (renderable, operation) = vector_render::create(
                    device,
                    &self.data,
                    graph.get_attribute_as_linked_output(application_point),
                    graph.get_attribute_as_linked_output(vector),
                    graph.get_attribute_as_usize(thickness).unwrap(),
                    graph.get_attribute_as_usize(material).unwrap(),
                )?;
                self.renderables.insert(graph_node_id, renderable);
                self.operations.insert(graph_node_id, operation);
            },
            _ => todo!("handle all graph node kinds!")
        }
        Ok(())
    }

}
