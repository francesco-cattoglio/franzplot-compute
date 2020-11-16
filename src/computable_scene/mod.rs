pub mod compute_block;
pub mod compute_chain;
pub mod globals;
pub mod scene_renderer;

use serde::{Deserialize, Serialize};

use globals::Globals;
use compute_chain::ComputeChain;
use scene_renderer::SceneRenderer;

#[derive(Debug, Deserialize, Serialize)]
pub struct Descriptor {
    pub global_names: Vec<String>,
    pub global_init_values: Vec<String>,
    pub descriptors: Vec<compute_block::BlockDescriptor>,
}

pub struct ComputableScene{
    pub globals: Globals,
    pub chain: ComputeChain,
    pub renderer: SceneRenderer,
}
