#[cxx::bridge(namespace = franzplot_gui)]
pub mod ffi{

    extern "C" {
        // library initialization functions
        include!("library.h");
        fn init_imnodes();
        fn shutdown_imnodes();

        // Gui class that manages and draws everything on screen
        include!("gui.h");
        type Gui;
        fn create_gui_instance(boxed_proxy: Box<RustEventProxy>) -> UniquePtr<Gui>;
        fn test_scene_ref(ref_state: Box<RustState>);
        fn Render(self: &mut Gui, x_size: u32, y_size: u32);
        fn UpdateSceneTexture(self: &mut Gui, scene_texture_id: usize);
        fn ClearAllMarks(self: &mut Gui);
        fn MarkClean(self: &mut Gui, node_id: i32);
        fn MarkError(self: &mut Gui, node_id: i32, message: &str);
        fn MarkWarning(self: &mut Gui, node_id: i32, message: &str);
        fn test_boxed_proxy(self: &mut Gui);
    }

    extern "Rust" {
        // All rust functions that we need to interact with the loop event proxy
        type RustEventProxy;
        type RustState;
        fn process_json(proxy: &RustEventProxy, json: &CxxString);
        fn print_proxy(boxed: &RustEventProxy, message: &CxxString);
        fn update_global_vars(proxy: &RustEventProxy, names: &CxxVector<CxxString>, values: &CxxVector<f32>);
        fn update_scene_camera(proxy: &RustEventProxy, dx: f32, dy: f32);
        fn lock_mouse_cursor(proxy: &RustEventProxy, x: f32, y: f32);
        fn unlock_mouse_cursor(proxy: &RustEventProxy);
        fn print_scene(scene: Box<RustState>);
    }
}

use crate::CustomEvent;
use crate::state;
type RustEventProxy = winit::event_loop::EventLoopProxy<super::CustomEvent>;
type RustState<'a> = &'a state::State;

fn print_scene(state: Box<RustState>) {
    state.test_print();
}

fn print_proxy(proxy: &RustEventProxy, message: &cxx::CxxString) {
    let message = CustomEvent::TestMessage(message.to_str().unwrap().to_string());
    proxy.send_event(message).expect("Internal error: main application loop no longer exists");
}

fn update_global_vars(proxy: &RustEventProxy, names: &cxx::CxxVector<cxx::CxxString>, values: &cxx::CxxVector<f32>) {
    let zip_iter = names.into_iter().zip(values.into_iter());
    let mut list = Vec::<(String, f32)>::new();
    for (c_name, value) in zip_iter {
        let string = c_name.to_string();
        list.push((string, *value));
    }
    proxy.send_event(CustomEvent::UpdateGlobals(list)).expect("Internal error: main application loop no longer exists");
}

fn process_json(proxy: &RustEventProxy, json: &cxx::CxxString) {
    let rust_str = json.to_str().expect("error validating the json string as UTF8");
    proxy.send_event(CustomEvent::JsonScene(rust_str.to_string())).expect("Internal error: main application loop no longer exists");
}

fn update_scene_camera(proxy: &RustEventProxy, dx: f32, dy: f32) {
    proxy.send_event(CustomEvent::UpdateCamera(dx, dy)).expect("Internal error: main application loop no longer exists");
}

fn lock_mouse_cursor(proxy: &RustEventProxy, x: f32, y: f32) {
    proxy.send_event(CustomEvent::LockMouseCursor(x as u32, y as u32)).expect("Internal error: main application loop no longer exists");
}

fn unlock_mouse_cursor(proxy: &RustEventProxy) {
    proxy.send_event(CustomEvent::UnlockMouseCursor).expect("Internal error: main application loop no longer exists");
}

