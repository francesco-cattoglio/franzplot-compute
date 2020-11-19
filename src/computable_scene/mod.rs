pub mod compute_block;
pub mod compute_chain;
pub mod globals;
pub mod scene_renderer;

use serde::{Deserialize, Serialize};

use globals::Globals;
use compute_chain::ComputeChain;
use scene_renderer::SceneRenderer;
use compute_block::BlockCreationError;
use crate::node_graph::NodeGraph;
use crate::node_graph::{ GraphError, Severity };

#[derive(Debug, Deserialize, Serialize)]
pub struct Descriptor {
    pub global_names: Vec<String>,
    pub global_init_values: Vec<f32>,
    pub descriptors: Vec<compute_block::BlockDescriptor>,
}

pub struct ComputableScene{
    pub globals: Globals,
    pub chain: ComputeChain,
    pub renderer: SceneRenderer,
}

impl ComputableScene {
    pub fn process_graph(&mut self, device: &wgpu::Device, graph: &mut NodeGraph, globals: Globals) -> Vec<GraphError> {
        self.globals = globals;
        let scene_result = self.chain.scene_from_graph(device, &self.globals, graph);
        self.renderer.update_renderables(device, &self.chain);

        // TODO: rewrite as a iter.map.collect?
        let mut to_return = Vec::<GraphError>::new();
        for (block_id, error) in scene_result.into_iter() {
            let id = block_id;
            match error {
                BlockCreationError::IncorrectAttributes(message) => {
                    to_return.push(GraphError {
                        severity: Severity::Error,
                        node_id: id,
                        message: imgui::ImString::new(message),
                    });
                    println!("incorrect attributes error for {}: {}", id, &message);
                },
                BlockCreationError::InputNotBuilt(message) => {
                    to_return.push(GraphError {
                        severity: Severity::Warning,
                        node_id: id,
                        message: imgui::ImString::new(message),
                    });
                    println!("input not build warning for {}: {}", id, &message);
                },
                BlockCreationError::InputMissing(message) => {
                    to_return.push(GraphError {
                        severity: Severity::Warning,
                        node_id: id,
                        message: imgui::ImString::new(message),
                    });
                    println!("missing input error for {}: {}", id, &message);
                },
                BlockCreationError::InputInvalid(message) => {
                    to_return.push(GraphError {
                        severity: Severity::Warning,
                        node_id: id,
                        message: imgui::ImString::new(message),
                    });
                    println!("invalid input error for {}: {}", id, &message);
                },
                BlockCreationError::InternalError(message) => {
                    println!("internal error: {}", &message);
                    panic!();
                },
            }
        }
        to_return
    }

}
