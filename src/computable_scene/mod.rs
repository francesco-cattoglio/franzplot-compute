pub mod compute_block;
pub mod compute_chain;
pub mod globals;

use globals::Globals;
use compute_chain::ComputeChain;
use crate::rendering::SceneRenderer;
use compute_block::BlockCreationError;
use crate::node_graph::NodeGraph;
use crate::node_graph::{ GraphError, Severity };

pub struct ComputableScene{
    pub globals: Globals,
    pub chain: ComputeChain,
    pub renderer: SceneRenderer,
    pub mouse_pos: [f32; 2],
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
                        message: String::from(message),
                    });
                    println!("incorrect attributes error for {}: {}", id, &message);
                },
                BlockCreationError::InputNotBuilt(message) => {
                    to_return.push(GraphError {
                        severity: Severity::Warning,
                        node_id: id,
                        message: String::from(message),
                    });
                    println!("input not build warning for {}: {}", id, &message);
                },
                BlockCreationError::InputMissing(message) => {
                    to_return.push(GraphError {
                        severity: Severity::Warning,
                        node_id: id,
                        message: String::from(message),
                    });
                    println!("missing input error for {}: {}", id, &message);
                },
                BlockCreationError::InputInvalid(message) => {
                    to_return.push(GraphError {
                        severity: Severity::Warning,
                        node_id: id,
                        message: String::from(message),
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
