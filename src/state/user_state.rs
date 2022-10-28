use std::path::Path;
use serde::{Serialize, Deserialize};

use crate::node_graph;
use crate::node_graph::NodeGraph;

// This structure holds the timestamps that we add to the saved files
#[derive(Clone, Deserialize, Serialize)]
pub struct TSs {
    pub fc: i64,
    pub fs: i64,
    pub vn: u32,
    pub hs: u64, // currently unused
}

impl TSs {
    pub fn new_now() -> Self {
        use rand::Rng;
        let random_number = rand::thread_rng().gen::<u32>();
        Self {
            hs: 0,
            vn: random_number,
            fc: chrono::offset::Utc::now().timestamp(),
            fs: chrono::offset::Utc::now().timestamp(),
        }
    }

    pub fn new_unknown() -> Self {
        use rand::Rng;
        let random_number = rand::thread_rng().gen::<u32>();
        Self {
            hs: 0,
            vn: random_number,
            fc: 0,
            fs: 0,
        }
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct UserGlobals {
    pub names: Vec<String>,
    pub init_values: Vec<f32>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct UserStateV1 {
    #[serde(rename = "graph")]
    pub node_graph: node_graph::NodeGraph,
    pub globals_names: Vec<String>,
    pub globals_init_values: Vec<f32>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct UserState {
    #[serde(rename = "graph")]
    pub node_graph: node_graph::NodeGraph,
    pub globals: UserGlobals,
    pub tss: TSs,
}

impl Default for UserState {
    fn default() -> Self {
        UserState {
            node_graph: NodeGraph::default(),
            globals: Default::default(),
            tss: TSs::new_now(),
        }
    }
}

impl UserState {
    // TODO: proper error handling
}

