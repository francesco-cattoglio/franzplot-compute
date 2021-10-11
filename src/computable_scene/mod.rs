pub mod compute_block;
pub mod compute_chain;
pub mod globals;

use globals::Globals;
use compute_chain::ComputeChain;
use crate::compute_graph::{ProcessingError, ComputeGraph};
use crate::state::Assets;
use crate::rendering::SceneRenderer;
use compute_block::BlockCreationError;
use crate::node_graph::NodeGraph;
use crate::node_graph::{ GraphError, Severity };

pub struct ComputableScene{
    pub globals: Globals,
    pub chain: ComputeChain,
    pub graph: ComputeGraph,
    pub renderer: SceneRenderer,
    pub mouse_pos: [f32; 2],
}

impl ComputableScene {
    pub fn process_graph(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, assets: &Assets, graph: &mut NodeGraph, globals: Globals) -> Vec<GraphError> {
        self.globals = globals;
        // TODO: this part is super confusing, there are huge side effects that
        // can easily go unnoticed. maybe refactor it?
        let scene_result = self.graph.process_graph(device, &assets.models, &self.globals, graph).unwrap();
        self.renderer.update_matcaps(device, &assets, &self.graph);

        // run the chain once, at the best of our possibilities
        self.graph.run_compute(device, queue, &self.globals);

        // Report every error to the user
        // TODO: rewrite as a iter.map.collect?
        let mut to_return = Vec::<GraphError>::new();
        for (block_id, error) in scene_result.into_iter() {
            let id = block_id;
            match error {
                ProcessingError::IncorrectAttributes(message) => {
                    to_return.push(GraphError {
                        severity: Severity::Error,
                        node_id: id,
                        message: String::from(message),
                    });
                    println!("incorrect attributes error for {}: {}", id, &message);
                },
                //ProcessingError::InputNotBuilt(message) => {
                //    to_return.push(GraphError {
                //        severity: Severity::Warning,
                //        node_id: id,
                //        message: String::from(message),
                //    });
                //    println!("input not build warning for {}: {}", id, &message);
                //},
                ProcessingError::InputMissing(message) => {
                    to_return.push(GraphError {
                        severity: Severity::Error,
                        node_id: id,
                        message: String::from(message),
                    });
                    println!("missing input error for {}: {}", id, &message);
                },
                //ProcessingError::InputInvalid(message) => {
                //    to_return.push(GraphError {
                //        severity: Severity::Error,
                //        node_id: id,
                //        message: String::from(message),
                //    });
                //    println!("invalid input error for {}: {}", id, &message);
                //},
                ProcessingError::IncorrectExpression(message) => {
                    println!("invalid input error for {}: {}", id, &message);
                    to_return.push(GraphError {
                        severity: Severity::Error,
                        node_id: id,
                        message,
                    });
                },
                ProcessingError::InternalError(message) => {
                    println!("internal error: {}", &message);
                    // A panic is a bit eccessive. Failing fast is good, but the user might be
                    // unable to report the error to the developer.
                    //
                    // panic!();
                },
            }
        }
        to_return
    }

}
