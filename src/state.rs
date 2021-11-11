use crate::computable_scene::*;
use crate::compute_graph::ComputeGraph;
use crate::device_manager::Manager;
use crate::rendering::camera;
use crate::rendering::SceneRenderer;
use crate::rendering::texture::{Texture, Masks};
use crate::rendering::model::Model;
use crate::node_graph;
use serde::{Serialize, Deserialize};

// The State struct encapsulates the whole application state,
// the GUI takes a mutable reference to the state and modifies it
// according to user input. The state contains both the data
// that the user is constantly editing (UserState) and the "rendered result"
// of that data (AppState). This distinction is very important w.r.t
// saving to file: we don't want to serialize compute shaders,
// we only want to save the graph, the variables and the scene settings.

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct UserState {
    pub graph: node_graph::NodeGraph,
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
    pub graph: ComputeGraph,
    pub renderer: SceneRenderer,
    pub computable_scene: ComputableScene,
    pub sensitivity: Sensitivity,
}

impl AppState {
    pub fn set_wireframe_axes(&mut self, length: i32, cross_size: f32) {
        self.computable_scene.renderer.set_wireframe_axes(length, cross_size, &self.manager.device);
    }

    pub fn clear_wireframe_axes(&mut self) {
        self.computable_scene.renderer.clear_wireframe_axes();
    }

    pub fn set_axes_labels(&mut self, axis_length: i32, label_size: f32) {
        self.computable_scene.renderer.set_axes_labels(axis_length as f32, label_size, &self.manager.device);
    }

    pub fn clear_axes_labels(&mut self) {
        self.computable_scene.renderer.clear_axes_labels();
    }

    pub fn update_depth_buffer(&mut self, size: wgpu::Extent3d) {
        self.computable_scene.renderer.update_depth_buffer_size(&self.manager.device, size);
    }

    pub fn update_projection_matrix(&mut self, size: wgpu::Extent3d) {
        self.camera.aspect = size.width as f32/size.height as f32;
    }


    pub fn update_camera(&mut self, camera_inputs: &camera::InputState) {
        if self.camera_enabled {
            self.camera_controller.update_camera(&mut self.camera, camera_inputs, &self.sensitivity, self.camera_lock_up);
        }
    }

    pub fn load_scene(&mut self, target_texture: &wgpu::TextureView) {
        // TODO: right now the chain is not recomputed if globals were not updated. This is
        // sub-optimal, and in the future we might want to be more fine-grained
        let global_vars_changed = self.computable_scene.globals.update_buffer(&self.manager.queue);
        if global_vars_changed {
            //self.computable_scene.chain.run_chain(&self.manager.device, &self.manager.queue, &self.computable_scene.globals);
            //self.computable_scene.graph.run_compute(&self.manager.device, &self.manager.queue, &self.computable_scene.globals);
        }
        if self.camera_ortho {
            // this is here instead of inside `update_projection_matrix` because
            // we are currently using the zoom level to build the orthographic matrix,
            // while `update_projection_matrix` gets called only on framebuffer resize
            self.computable_scene.renderer.update_proj(self.camera.build_ortho_matrix());
        } else {
            self.computable_scene.renderer.update_proj(self.camera.build_projection_matrix());
        }
        self.computable_scene.renderer.update_view(self.camera.build_view_matrix());
        //self.computable_scene.renderer.update_matcaps(&self.manager.device, &self.assets, &self.computable_scene.graph);

        // after updating everything, redraw the scene to the texture
        self.computable_scene.renderer.render(&self.manager, target_texture);
    }
    pub fn update_scene(&mut self, target_texture: &wgpu::TextureView) {
        // TODO: right now the chain is not recomputed if globals were not updated. This is
        // sub-optimal, and in the future we might want to be more fine-grained
        let global_vars_changed = self.computable_scene.globals.update_buffer(&self.manager.queue);
        if global_vars_changed {
            self.computable_scene.chain.run_chain(&self.manager.device, &self.manager.queue, &self.computable_scene.globals);
            //self.computable_scene.graph.run_compute(&self.manager.device, &self.manager.queue, &self.computable_scene.globals);
        }
        if self.camera_ortho {
            // this is here instead of inside `update_projection_matrix` because
            // we are currently using the zoom level to build the orthographic matrix,
            // while `update_projection_matrix` gets called only on framebuffer resize
            self.computable_scene.renderer.update_proj(self.camera.build_ortho_matrix());
        } else {
            self.computable_scene.renderer.update_proj(self.camera.build_projection_matrix());
        }
        self.computable_scene.renderer.update_view(self.camera.build_view_matrix());
        // after updating everything, redraw the scene to the texture
        self.computable_scene.renderer.render(&self.manager, target_texture);
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

        // construct the AppState part from the passed-in manager
        let computable_scene = ComputableScene {
            globals: globals::Globals::new(&manager.device, vec![], vec![]),
            chain: compute_chain::ComputeChain::new(),
            graph: ComputeGraph::new(),
            renderer: SceneRenderer::new_with_axes(&manager.device),
            mouse_pos: [0.0, 0.0],
        };

        let camera = camera::Camera::from_height_width(manager.config.height as f32, manager.config.width as f32);
        let camera_controller = Box::new(camera::VTKController::new());

        let app = AppState {
            computable_scene,
            assets,
            camera,
            auto_scene_on_processing: true,
            camera_enabled: false,
            camera_lock_up: true,
            camera_ortho: false,
            camera_controller,
            manager,
            sensitivity: Sensitivity::default(),
        };

        Self {
            app,
            time_stamps: TSs::new_now(),
            user,
        }
    }

    pub fn write_to_frzp(&mut self, path: &std::path::PathBuf) {
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
        file.write_all(contents.as_bytes()).unwrap();
    }

    pub fn read_from_frzp(&mut self, path: &std::path::PathBuf) -> Result<(), &'static str> {
        let mut file = std::fs::File::open(path).unwrap();
        let mut contents = String::new();
        use std::io::Read;
        file.read_to_string(&mut contents)
            .or(Err("Error opening file. Content is not UTF-8."))?;
        let saved_data: FileVersion = ron::from_str(&contents)
            .or(Err("Error reading file contents. Is this a franzplot file?"))?;
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
        self.user.graph.push_positions_to_imnodes();
        Ok(())
    }

    pub fn new_file(&mut self) {
        // reset the userstate and the time stamps, clear the scene
        self.user = UserState::default();
        self.time_stamps = TSs::new_now();
        self.process_user_state();
    }

    // process the user graph, and return true if no errors were detected
    pub fn process_user_state(&mut self) -> bool {
        // try to build a new compute chain.
        // clear all errors
        self.user.graph.clear_all_errors();
        // TODO: refactor some of this perhaps? I feel like a
        // ComputableScene::process_user_state would be easier to read and reason about
        // create a new Globals from the user defined names
        let globals = globals::Globals::new(&self.app.manager.device, self.user.globals_names.clone(), self.user.globals_init_values.clone());
        let graph_errors = self.app.computable_scene.process_graph(&self.app.manager.device, &self.app.manager.queue, &self.app.assets, &mut self.user.graph, globals);
        let no_errors_detected = graph_errors.is_empty();
        for error in graph_errors.into_iter() {
            self.user.graph.mark_error(error);
        }
        return no_errors_detected;
    }
    // TODO: rename when switching to wgsl for compute is done
    // process the user graph, and return true if no errors were detected
    pub fn process_user_state_2(&mut self) -> bool {
        let globals = globals::Globals::new(&self.app.manager.device, self.user.globals_names.clone(), self.user.globals_init_values.clone());
        let graph_errors = self.app.computable_scene.process_graph_2(&self.app.manager.device, &self.app.manager.queue, &self.app.assets, &mut self.user.graph, globals);
        let no_errors_detected = graph_errors.is_empty();
        for error in graph_errors.into_iter() {
            self.user.graph.mark_error(error);
        }
        return no_errors_detected;
    }
}
