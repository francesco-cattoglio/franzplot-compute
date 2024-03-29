use std::path::Path;

use crate::compute_graph::ComputeGraph;
use crate::device_manager::Manager;
use crate::rendering::camera;
use crate::rendering::SceneRenderer;
use crate::rendering::texture::{Texture, Masks};
use crate::rendering::model::Model;
use crate::node_graph;
use serde::{Serialize, Deserialize};

pub mod action;
pub use action::Action;

// The State struct encapsulates the whole application state,
// the GUI takes a mutable reference to the state and modifies it
// according to user input. The state contains both the data
// that the user is constantly editing (UserState) and the "rendered result"
// of that data (AppState). This distinction is very important w.r.t
// saving to file: we don't want to serialize compute shaders,
// we only want to save the graph, the variables and the scene settings.

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct UserState {
    #[serde(rename = "graph")]
    pub node_graph: node_graph::NodeGraph,
    pub globals_names: Vec<String>,
    pub globals_init_values: Vec<f32>,
}

// This structure holds the timestamps that we add to the saved files
#[derive(Clone, Default, Deserialize, Serialize)]
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

#[derive(Deserialize, Serialize)]
enum FileVersion {
    V0(UserState),
    V1(UserState, TSs),
}

pub struct Assets {
    pub materials: Vec<Texture>,
    pub masks: Masks,
    pub models: Vec<Model>,
}

pub struct Sensitivity {
    pub graph_zoom: f32,
    pub scene_zoom: f32,
    pub camera_horizontal: f32,
    pub camera_vertical: f32,
}

impl Default for Sensitivity {
    fn default() -> Self {
        Sensitivity {
            graph_zoom: 1.0,
            scene_zoom: 1.0,
            camera_horizontal: 1.0,
            camera_vertical: 1.0,
        }
    }
}

pub struct AppState {
    pub camera_controller: Box<dyn camera::Controller>,
    pub camera_enabled: bool,
    pub camera_lock_up: bool,
    pub camera_ortho: bool, // TODO: all these camera settings should NOT be here, move them somewhere else!
    pub auto_scene_on_processing: bool,
    pub camera: camera::Camera,
    pub assets: Assets,
    pub manager: Manager,
    pub comp_graph: Option<ComputeGraph>,
    pub renderer: SceneRenderer,
    pub sensitivity: Sensitivity,
}

impl AppState {
    pub fn set_wireframe_axes(&mut self, length: i32, cross_size: f32) {
        self.renderer.set_wireframe_axes(&self.manager, length, cross_size);
    }

    pub fn clear_wireframe_axes(&mut self) {
        self.renderer.clear_wireframe_axes();
    }

    pub fn set_axes_labels(&mut self, axis_length: i32, label_size: f32) {
        self.renderer.set_axes_labels(&self.manager, axis_length as f32, label_size);
    }

    pub fn clear_axes_labels(&mut self) {
        self.renderer.clear_axes_labels();
    }

    pub fn update_camera(&mut self, camera_inputs: &camera::InputState) {
        if self.camera_enabled {
            self.camera_controller.update_camera(&mut self.camera, camera_inputs, &self.sensitivity, self.camera_lock_up);
        }
    }
}

pub struct State {
    pub app: AppState,
    pub time_stamps: TSs,
    pub user: UserState,
}

impl State {
    // this function will likely be called only once, at program start
    pub fn new(manager: Manager, assets: Assets) -> Self {
        // at program start, we can just set the user data to its default value
        let user: UserState = Default::default();

        let camera = camera::Camera::default();
        let camera_controller = Box::new(camera::VTKController::new());

        let app = AppState {
            //computable_scene,
            assets,
            camera,
            auto_scene_on_processing: true,
            camera_enabled: false,
            camera_lock_up: true,
            camera_ortho: false,
            camera_controller,
            renderer: SceneRenderer::new_with_axes(&manager),
            manager,
            comp_graph: None,
            sensitivity: Sensitivity::default(),
        };

        Self {
            app,
            time_stamps: TSs::new_now(),
            user,
        }
    }

    pub fn process(&mut self, action: Action) -> Result<(), String>{
        match action {
            Action::WriteToFile(path) => {
                self.write_to_frzp(&path);
                Ok(())
            } ,
            Action::OpenFile(path) => {
                self.read_from_frzp(&path)
            },
            Action::NewFile() => {
                // reset the user state: this will zero out the node graph and its global vars
                self.user = UserState::default();
                // clear all the created renderables and the entire compute graph
                self.app.renderer.clear_matcaps();
                self.app.comp_graph = None;
                // new timestamp for the new file
                self.time_stamps = TSs::new_now();
                Ok(())
            },
            Action::RenderScene(extent, view) => {
                // create aliases
                let renderer = &mut self.app.renderer;
                let camera = &mut self.app.camera;
                renderer.resize_if_needed(&self.app.manager, extent);
                let aspect_ratio = extent.width as f32/extent.height as f32;
                let projection_matrix = if self.app.camera_ortho {
                    camera.build_ortho_matrix(aspect_ratio)
                } else {
                    camera.build_projection_matrix(aspect_ratio)
                };
                renderer.update_proj(projection_matrix);
                renderer.update_view(camera.build_view_matrix());
                // after updating everything, redraw the scene to the texture
                renderer.render(&self.app.manager, view);
                Ok(())
            },
            Action::ProcessUserState() => {
                // - clear previous node graph errors
                // - try to create a new compute graph
                // - if successful, update the scene rendering and report recoverable errors
                // - if unsuccessful, report the unrecoverable error to the user
                self.user.node_graph.clear_all_errors();
                let process_result = crate::compute_graph::create_compute_graph(&self.app.manager.device, &self.app.assets, &self.user);
                match process_result {
                    Ok((compute_graph, recoverable_errors)) => {
                        // run the first compute, and create the matcaps in the SceneRenderer
                        compute_graph.run_compute(&self.app.manager.device, &self.app.manager.queue);
                        self.app.renderer.recreate_matcaps(&self.app.manager, &self.app.assets, compute_graph.matcaps());
                        self.app.comp_graph = Some(compute_graph);
                        if recoverable_errors.is_empty() {
                            Ok(())
                        } else {
                            for error in recoverable_errors.into_iter() {
                                self.user.node_graph.mark_error(error.into());
                            }
                            Err("Recoverable errors detected".into())
                        }
                    },
                    Err(unrecoverable_error) => {
                        let formatted_error = format!("Unrecoverable error: {:?}", &unrecoverable_error);
                        self.user.node_graph.mark_error(unrecoverable_error.into());
                        Err(formatted_error) // TODO: better handling
                    }
                }
            }
            Action::UpdateGlobals(pairs) => {
                // if the compute graph exists, tell it to update the globals
                if let Some(graph) = &mut self.app.comp_graph {
                    graph.update_globals(&self.app.manager.device, &self.app.manager.queue, pairs);
                    Ok(())
                } else {
                    dbg!("tried to update globals, but there is no graph!"); // TODO: better handling
                    Ok(())
                }
            }
        }
    }

    fn write_to_frzp(&mut self, path: &Path) {
        let mut file = std::fs::File::create(path).unwrap();
        let ser_config = ron::ser::PrettyConfig::new()
            .with_depth_limit(5)
            .with_indentor("  ".to_owned())
            .with_separate_tuple_members(true)
            .with_enumerate_arrays(true);
        // update the time_stamp to remember the last time the file was saved
        self.time_stamps.fs = chrono::offset::Utc::now().timestamp();
        let to_serialize = FileVersion::V1(self.user.clone(), self.time_stamps.clone());
        let serialized_data = ron::ser::to_string_pretty(&to_serialize, ser_config).unwrap();
        let mut contents = r##"//// FRANZPLOT DATA FILE V1.1 \\\\

//   This file should not be edited by hand,
//   as doing so might easily corrupt the data.
//   To edit this file, open it in Franzplot, version 21.04 or higher

"##.to_string();

        contents.push_str(&serialized_data);
        use std::io::Write;
        file.write_all(contents.as_bytes()).unwrap(); // TODO: handle writing failures
    }

    fn read_from_frzp(&mut self, path: &Path) -> Result<(), String> {
        let mut file = std::fs::File::open(path).unwrap();
        let mut contents = String::new();
        use std::io::Read;
        file.read_to_string(&mut contents)
            .map_err(|error| format!("Error opening file: {}", &error))?;
        let saved_data: FileVersion = ron::from_str(&contents)
            .map_err(|_| "Error reading file contents. Is this a franzplot file?".to_string())?;
        match saved_data {
            FileVersion::V0(user_state) => {
                // loading an older file that does NOT have timestamp infos
                self.user = user_state;
                self.time_stamps = TSs::new_unknown();
            }
            FileVersion::V1(user_state, time_stamps) => {
                self.user = user_state;
                self.time_stamps = time_stamps;
            }
        }
        self.user.node_graph.push_positions_to_imnodes();
        Ok(())
    }
}
