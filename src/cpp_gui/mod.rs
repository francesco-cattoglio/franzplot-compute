use crate::state::State;

#[derive(Clone, Copy)]
#[repr(i32)]
pub enum PinShape
{
    Circle,
    CircleFilled,
    Triangle,
    TriangleFilled,
    Quad,
    QuadFilled
}
unsafe impl cxx::ExternType for PinShape {
    type Id = cxx::type_id!("imnodes::PinShape");
    type Kind = cxx::kind::Trivial;
}
#[cxx::bridge(namespace = "imnodes")]
pub mod imnodes {
    unsafe extern "C++" {
        include!("franzplot-compute/src/cpp_gui/imnodes-8ecdd3/imnodes.h");
        include!("franzplot-compute/src/cpp_gui/pointer_shims.h");
        type PinShape = super::PinShape;
        fn Initialize();
        fn Shutdown();
        fn BeginNodeEditor();
        fn EndNodeEditor();
        fn BeginNode(id: i32);
        fn EndNode();
        fn BeginNodeTitleBar();
        fn EndNodeTitleBar();
        fn BeginInputAttribute(id: i32, shape: PinShape);
        fn EndInputAttribute();
        fn BeginStaticAttribute(id: i32);
        fn EndStaticAttribute();
        fn BeginOutputAttribute(id: i32, shape: PinShape);
        fn EndOutputAttribute();
        fn Link(link_id: i32, first_id: i32, second_id: i32);
        fn IsLinkCreated(first_id: &mut i32, second_id: &mut i32) -> bool;
    }
}

#[cxx::bridge]
pub mod ffi{
    // some common structures used as return types for various functions
    #[namespace = "franzplot_gui"]
    struct GraphError {
        message: String,
        node_id: i32,
        is_warning: bool,
    }

    // this struct is the easies way I found to give imgui some control over
    // the winit event loop without using the event proxy. The event proxy
    // sometimes is not good enough because there is one frame delay between
    // the request and the execution.
    #[namespace = "franzplot_gui"]
    struct GuiRequests {
        frozen_mouse_x: u32,
        frozen_mouse_y: u32,
        freeze_mouse: bool,
    }
}

//#[cxx::bridge]
//pub mod ffi{
//    // some common structures used as return types for various functions
//    #[namespace = "franzplot_gui"]
//    struct GraphError {
//        message: String,
//        node_id: i32,
//        is_warning: bool,
//    }
//
//    // this struct is the easies way I found to give imgui some control over
//    // the winit event loop without using the event proxy. The event proxy
//    // sometimes is not good enough because there is one frame delay between
//    // the request and the execution.
//    #[namespace = "franzplot_gui"]
//    struct GuiRequests {
//        frozen_mouse_x: u32,
//        frozen_mouse_y: u32,
//        freeze_mouse: bool,
//    }
//
//    #[namespace = "franzplot_gui"]
//    unsafe extern "C++" {
//        // library initialization functions
//        include!("library.h");
//        fn init_imnodes();
//        fn shutdown_imnodes();
//
//        // Gui class that manages and draws everything on screen
//        // A quick note on how the two codes manage to interact:
//        // The main.rs holds two objects: a unique_ptr to a GUI instance and a State
//        // struct. Depending on what we need to achieve we can either pass the GUI
//        // to a State's function, or we can pass a mut reference of the State to the GUI function.
//        include!("gui.h");
//        type Gui;
//        fn create_gui_instance() -> UniquePtr<Gui>;
//        fn Render(self: Pin<&mut Gui>, state: &mut State, x_size: u32, y_size: u32) -> GuiRequests;
//        fn UpdateSceneTexture(self: Pin<&mut Gui>, scene_texture_id: usize);
//    }
//
//    #[namespace = "franzplot_gui"]
//    extern "Rust" {
//        // All rust functions that we need to interact with the rest of the code.
//        // Most of them are just shims/translation layers for the C++ types
//        type State;
//        fn process_json(state: &mut State, json: &CxxString) -> Vec<GraphError>;
//        fn update_scene_camera(state: &mut State, dx: f32, dy: f32);
//        fn get_globals_names(state: &State) -> &Vec<String>;
//        fn get_globals_values(state: &mut State) -> &mut Vec<f32>;
//    }
//}
//
//fn process_json(state: &mut State, json: &cxx::CxxString) -> Vec<ffi::GraphError> {
//    let rust_str = json.to_str().expect("error validating the json string as UTF8");
//    state.computable_scene.process_json(&state.manager.device, rust_str)
//}
//
//fn update_scene_camera(state: &mut State, dx: f32, dy: f32) {
//    state.camera_controller.process_mouse(dx, dy);
//}
//
//fn get_globals_names(state: &State) -> &Vec<String> {
//    state.computable_scene.globals.get_names()
//}
//
//fn get_globals_values(state: &mut State) -> &mut Vec<f32> {
//    state.computable_scene.globals.get_values_mut()
//}

