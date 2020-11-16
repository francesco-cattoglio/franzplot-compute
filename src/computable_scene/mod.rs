pub mod compute_block;
pub mod compute_chain;
pub mod globals;
pub mod scene_renderer;

use crate::cpp_gui::ffi::GraphError;
use serde::{Deserialize, Serialize};

use globals::Globals;
use compute_chain::ComputeChain;
use scene_renderer::SceneRenderer;
use compute_block::BlockCreationError;

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
    pub fn process_json(&mut self, device: &wgpu::Device, json: &str) -> Vec<GraphError> {
        let json_scene: Descriptor = serde_jsonrc::from_str(&json).unwrap();
        // TODO: make globals use both the names and the init values!
        self.globals = globals::Globals::new(device, json_scene.global_names);
        let scene_result = self.chain.set_scene(device, &self.globals, json_scene.descriptors);
        self.renderer.update_renderables(device, &self.chain);
        let mut to_return = Vec::<GraphError>::new();
        // TODO: rewrite as a iter.map.collect
        for (block_id, error) in scene_result.iter() {
            let id = *block_id;
            match error {
                BlockCreationError::IncorrectAttributes(message) => {
                    to_return.push(GraphError {
                        is_warning: false,
                        node_id: id,
                        message: message.to_string(),
                    });
                    println!("incorrect attributes error for {}: {}", id, &message);
                },
                BlockCreationError::InputNotBuilt(message) => {
                    to_return.push(GraphError {
                        is_warning: true,
                        node_id: id,
                        message: message.to_string(),
                    });
                    println!("input not build warning for {}: {}", id, &message);
                },
                BlockCreationError::InputMissing(message) => {
                    to_return.push(GraphError {
                        is_warning: false,
                        node_id: id,
                        message: message.to_string(),
                    });
                    println!("missing input error for {}: {}", id, &message);
                },
                BlockCreationError::InputInvalid(message) => {
                    to_return.push(GraphError {
                        is_warning: false,
                        node_id: id,
                        message: message.to_string(),
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
