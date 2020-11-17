use crate::state::State;

#[cxx::bridge(namespace = franzplot_gui)]
pub mod ffi{
    // some common structures used as return types for various functions
    struct GraphError {
        message: String,
        node_id: i32,
        is_warning: bool,
    }

    // this struct is the easies way I found to give imgui some control over
    // the winit event loop without using the event proxy. The event proxy
    // sometimes is not good enough because there is one frame delay between
    // the request and the execution.
    struct GuiRequests {
        frozen_mouse_x: u32,
        frozen_mouse_y: u32,
        freeze_mouse: bool,
    }

    extern "C++" {
        // library initialization functions
        include!("library.h");
        fn init_imnodes();
        fn shutdown_imnodes();

        // Gui class that manages and draws everything on screen
        // A quick note on how the two codes manage to interact:
        // The main.rs holds two objects: a unique_ptr to a GUI instance and a State
        // struct. Depending on what we need to achieve we can either pass the GUI
        // to a State's function, or we can pass a mut reference of the State to the GUI function.
        include!("gui.h");
        type Gui;
        fn create_gui_instance(boxed_proxy: Box<RustEventProxy>) -> UniquePtr<Gui>;
        fn Render(self: &mut Gui, state: &mut State, x_size: u32, y_size: u32) -> GuiRequests;
        fn UpdateSceneTexture(self: &mut Gui, scene_texture_id: usize);
    }

    extern "Rust" {
        // All rust functions that we need to interact with the rest of the code.
        // Most of them are just shims/translation layers for the C++ types
        type RustEventProxy;
        type State;
        fn process_json(state: &mut State, json: &CxxString) -> Vec<GraphError>;
        fn update_scene_camera(state: &mut State, dx: f32, dy: f32);
        fn get_globals_names(state: &State) -> &Vec<String>;
        fn get_globals_values(state: &mut State) -> &mut Vec<f32>;
    }
}

// TODO: maybe remove this. There is no use for it right now, but perhaps it will be needed in the future
use crate::CustomEvent;
type RustEventProxy = winit::event_loop::EventLoopProxy<CustomEvent>;

fn process_json(state: &mut State, json: &cxx::CxxString) -> Vec<ffi::GraphError> {
    let rust_str = json.to_str().expect("error validating the json string as UTF8");
    state.computable_scene.process_json(&state.manager.device, rust_str)
}

fn update_scene_camera(state: &mut State, dx: f32, dy: f32) {
    state.camera_controller.process_mouse(dx, dy);
}

fn get_globals_names(state: &State) -> &Vec<String> {
    state.computable_scene.globals.get_names()
}

fn get_globals_values(state: &mut State) -> &mut Vec<f32> {
    state.computable_scene.globals.get_values_mut()
}


