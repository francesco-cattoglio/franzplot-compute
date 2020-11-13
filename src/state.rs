use crate::compute_chain::{ ComputeChain, Globals };
use crate::device_manager::Manager;
use crate::rendering::SceneRenderer;
use crate::camera::{ Camera, CameraController };
use crate::cpp_gui::ffi::GraphError;
use crate::compute_block::BlockCreationError;

// this struct encapsulates the whole application state, and doubles as an entry point
// for the C++ side of the code: the GUI will take a reference to the state, thus allowing
// the gui to have some control over the Rust side.
pub struct State {
    pub chain: ComputeChain,
    pub globals: Globals,
    pub manager: Manager,
    pub scene_renderer: SceneRenderer,
    pub camera: Camera,
    pub camera_controller: CameraController,
}

impl State {
    pub fn process_json(&mut self, json: &str) -> Vec<GraphError> {
        let json_scene: super::SceneDescriptor = serde_jsonrc::from_str(&json).unwrap();
        self.globals = Globals::new(&self.manager.device, json_scene.global_vars);
        let scene_result = self.chain.set_scene(&self.manager.device, &self.globals, json_scene.descriptors);
        self.scene_renderer.update_renderables(&self.manager.device, &self.chain);
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
