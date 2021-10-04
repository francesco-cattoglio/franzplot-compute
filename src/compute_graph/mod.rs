use std::collections::BTreeMap;
use crate::node_graph::{NodeGraph, NodeID, NodeContents};
use crate::computable_scene::globals::Globals;

mod operation;
use operation::Operation;

pub type DataID = i32;
pub type PrefabId = i32;

pub enum ProcessingError {
    InputMissing(&'static str),
    InternalError(String),
    IncorrectAttributes(&'static str),
    IncorrectExpression(String),
    Unknown,
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
    },
    NotComputed,
}

// The compute graph contains:
// - a map of all the Data in the graph
// - a list of all the operations that are to be executed (that also implies: all the compute
// shaders that are to be run)
// - a list of all the renderables that were created as outputs.
pub struct ComputeGraph {
    data: BTreeMap<DataID, Data>,
    operations: Vec<Operation>,
}

impl ComputeGraph {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            operations: Vec::new(),
        }
    }
    // process a single graph node, creating some data and one operation
    pub fn process_graph_node(&self, device: &wgpu::Device, globals: &Globals, graph_node_id: NodeID, graph: &NodeGraph) -> ProcessingResult {
        let to_process = graph.get_node(graph_node_id).unwrap();
        match *to_process.contents() {
            NodeContents::Curve {
                interval, fx, fy, fz, output
            } => {
                Operation::new_curve(
                    device,
                    globals,
                    &self.data,
                    graph.get_attribute_as_linked_node(interval),
                    graph.get_attribute_as_string(fx).unwrap(),
                    graph.get_attribute_as_string(fy).unwrap(),
                    graph.get_attribute_as_string(fz).unwrap(),
                    output,
                )
            },
            NodeContents::Interval {
                variable, begin, end, quality, output,
            } => {
                Operation::new_interval(
                    device,
                    globals,
                    graph.get_attribute_as_string(variable).unwrap(),
                    graph.get_attribute_as_string(begin).unwrap(),
                    graph.get_attribute_as_string(end).unwrap(),
                    graph.get_attribute_as_usize(quality).unwrap(),
                    output
                    )
            },
            _ => todo!("handle all graph node kinds!")
        }

}



}
