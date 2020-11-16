use crate::computable_scene::*;
use crate::device_manager::Manager;
use crate::rendering::camera::{ Camera, CameraController };

// this struct encapsulates the whole application state, and doubles as an entry point
// for the C++ side of the code: the GUI will take a reference to the state, thus allowing
// the gui to have some control over the Rust side.
pub struct State {
    pub computable_scene: ComputableScene,
    pub manager: Manager,
    pub camera: Camera,
    pub camera_controller: CameraController,
}

