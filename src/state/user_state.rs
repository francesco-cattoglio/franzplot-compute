use std::path::Path;

use crate::node_graph;
use serde::{Serialize, Deserialize};
use crate::node_graph::NodeGraph;

// File versioning is a bit of a mess, unfortunately, especially when loading
// an older version. This is because TSs in UserStateV1 are automatically added by serde.
#[derive(Deserialize, Serialize)]
enum FileVersion {
    V0(UserStateV1),
    V1(UserStateV1, TSs),
    V2(UserState),
}

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
    pub fn write_to_frzp(&mut self, path: &Path) {
        let mut file = std::fs::File::create(path).unwrap();
        let ser_config = ron::ser::PrettyConfig::new()
            .depth_limit(5)
            .indentor("  ".to_owned())
            .separate_tuple_members(true)
            .enumerate_arrays(true);
        // update the time_stamp to remember the last time the file was saved
        self.tss.fs = chrono::offset::Utc::now().timestamp();
        let to_serialize = FileVersion::V2(self.clone());
        let serialized_data = ron::ser::to_string_pretty(&to_serialize, ser_config).unwrap();
        let mut contents = r##"//// FRANZPLOT DATA FILE V1.2 \\\\

//   This file should not be edited by hand,
//   as doing so might easily corrupt the data.
//   To edit this file, open it in Franzplot, version 22.10 or higher

"##.to_string();

        contents.push_str(&serialized_data);
        use std::io::Write;
        file.write_all(contents.as_bytes()).unwrap(); // TODO: handle writing failures
    }

    pub fn read_from_frzp(path: &Path) -> Result<Self, String> {
        let mut file = std::fs::File::open(path).unwrap();
        let mut contents = String::new();
        use std::io::Read;
        file.read_to_string(&mut contents)
            .map_err(|error| format!("Error opening file: {}", &error))?;
        let saved_data: FileVersion = ron::from_str(&contents)
            .map_err(|_| "Error reading file contents. Is this a franzplot file?".to_string())?;
        match saved_data {
            FileVersion::V0(user_state) => {
                // loading an older file that does NOT have timestamp infos.
                // Destructure the contents of the file
                // and assign them to a more recent version of the UserState
                let UserStateV1 {
                    node_graph,
                    globals_names,
                    globals_init_values,
                } = user_state;
                Ok(UserState {
                    node_graph,
                    globals: UserGlobals {
                        names: globals_names,
                        init_values: globals_init_values,
                    },
                    tss: TSs::new_unknown(),
                })
            }
            FileVersion::V1(user_state, time_stamps) => {
                // if we load a V1, we can just read the time stamps,
                // and put them in the UserState
                let UserStateV1 {
                    node_graph,
                    globals_names,
                    globals_init_values,
                } = user_state;
                Ok(UserState {
                    node_graph,
                    globals: UserGlobals {
                        names: globals_names,
                        init_values: globals_init_values,
                    },
                    tss: time_stamps,
                })
            }
            FileVersion::V2(user_state) => {
                // this time, everything should be there already
                Ok(user_state)
            }
        }
    }
}

