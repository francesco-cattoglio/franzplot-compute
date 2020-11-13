use crate::compute_chain::{ ComputeChain, Globals };
use crate::device_manager::Manager;
use crate::rendering::SceneRenderer;
use crate::camera::{ Camera, CameraController };

// this struct encapsulates the whole application state, and doubles as an "interface"
// for the C++ side of the code: the GUI will take a Box as an input and this will allow
// imgui to have (some) control over the Rust side.
pub struct State {
    pub chain: ComputeChain,
    pub globals: Globals,
    pub manager: Manager,
    pub scene_renderer: SceneRenderer,
    pub camera: Camera,
    pub camera_controller: CameraController,
}

impl State {
    pub fn test_increment(&mut self) {
    }
    pub fn test_print(&self) {
    }
}
