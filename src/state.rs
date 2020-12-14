use crate::computable_scene::*;
use crate::device_manager::Manager;
use crate::rendering::camera;
use crate::rendering::SceneRenderer;
use crate::rendering::texture::{Texture, Masks};
use crate::node_graph;
use serde::{Serialize, Deserialize};

// The State struct encapsulates the whole application state,
// the GUI takes a mutable reference to the state and modifies it
// according to user input. The state contains both the data
// that the user is constantly editing (UserState) and the "rendered result"
// of that data (AppState). This distinction is very important w.r.t
// saving to file: we don't want to serialize compute shaders,
// we only want to save the graph, the variables and the scene settings.

#[derive(Default, Deserialize, Serialize)]
pub struct UserState {
    pub graph: node_graph::NodeGraph,
    pub globals_names: Vec<String>,
    pub globals_init_values: Vec<f32>,
}

impl UserState {
    pub fn write_to_file(&self, path: &std::path::PathBuf) {
        let file = std::fs::File::create(path).unwrap();
        serde_json::to_writer_pretty(file, &self).unwrap();
    }

    pub fn read_from_file(&mut self, path: &std::path::PathBuf) {
        let file = std::fs::File::open(path).unwrap();
        let maybe_user_state = serde_json::from_reader(file);
        *self = maybe_user_state.unwrap();
        self.graph.push_positions_to_imnodes();
    }
}

pub struct AppState {
    pub camera_controller: Box<dyn camera::Controller>,
    pub camera_enabled: bool,
    pub camera: camera::Camera, // we might want to store camera position in user state
    pub manager: Manager,
    pub textures: Vec<Texture>,
    pub masks: Masks,
    pub computable_scene: ComputableScene,
}

impl AppState {
    pub fn update_depth_buffer(&mut self, size: wgpu::Extent3d) {
        self.computable_scene.renderer.update_depth_buffer_size(&self.manager.device, size);
    }

    pub fn update_projection_matrix(&mut self, size: wgpu::Extent3d) {
        self.camera.aspect = size.width as f32/size.height as f32;
        self.computable_scene.renderer.update_proj(self.camera.build_projection_matrix());
    }

    pub fn update_scene(&mut self, target_texture: &wgpu::TextureView, camera_inputs: &camera::InputState) {
        // TODO: make sure this is done only when it is really needed!
        self.computable_scene.globals.update_buffer(&self.manager.queue);
        self.computable_scene.chain.run_chain(&self.manager.device, &self.manager.queue, &self.computable_scene.globals);
        if self.camera_enabled {
            self.camera_controller.update_camera(&mut self.camera, camera_inputs);
        }
        self.computable_scene.renderer.update_view(self.camera.build_view_matrix());
        // after updating everything, redraw the scene to the texture
        self.computable_scene.renderer.render(&self.manager, target_texture);
    }
}

pub struct State {
    pub app: AppState,
    pub user: UserState,
}

impl State {
    // this function will likely be called only once, at program start
    pub fn new(manager: Manager) -> Self {
        // at program start, we can just set the user data to its default value
        let user: UserState = Default::default();

        // construct the AppState part from the passed-in manager
        let computable_scene = ComputableScene {
            globals: globals::Globals::new(&manager.device, vec![], vec![]),
            chain: compute_chain::ComputeChain::new(),
            renderer: SceneRenderer::new(&manager.device),
            mouse_pos: [0.0, 0.0],
        };

        let checkerboard = Texture::load(&manager.device, &manager.queue, "./resources/checkerboard.png", "checkers").unwrap();
        let test_matcap = Texture::load(&manager.device, &manager.queue, "./resources/matcap_test.png", "matcaptest").unwrap();
        let test_matcap_2 = Texture::load(&manager.device, &manager.queue, "./resources/matcap_test_2.png", "matcaptest").unwrap();
        let test_matcap_3 = Texture::load(&manager.device, &manager.queue, "./resources/matcap_test_3.png", "matcaptest").unwrap();

        let camera = camera::Camera::from_height_width(manager.sc_desc.height as f32, manager.sc_desc.width as f32);
        let camera_controller = Box::new(camera::VTKController::new(0.015, 0.015, 0.03));
        use std::convert::TryInto;
        let app = AppState {
            computable_scene,
            masks: vec![checkerboard].try_into().expect("wrong length"),
            textures: vec![test_matcap, test_matcap_2, test_matcap_3],
            camera,
            camera_enabled: false,
            camera_controller,
            manager
        };

        Self {
            app,
            user,
        }
    }

    pub fn process_user_state(&mut self) {
        // try to build a new compute chain.
        // clear all errors
        self.user.graph.clear_all_errors();
        // TODO: refactor some of this perhaps? I feel like a
        // ComputableScene::process_user_state would be easier to read and reason about
        // create a new Globals from the user defined names
        let globals = globals::Globals::new(&self.app.manager.device, self.user.globals_names.clone(), self.user.globals_init_values.clone());
        let graph_errors = self.app.computable_scene.process_graph(&self.app.manager.device, &self.app.masks, &self.app.textures, &mut self.user.graph, globals);
        for error in graph_errors.into_iter() {
            self.user.graph.mark_error(error);
        }
    }
}
