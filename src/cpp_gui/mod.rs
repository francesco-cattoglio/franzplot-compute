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
        fn Render(self: &mut Gui);
        fn test_boxed_proxy(self: &mut Gui);
    }

    extern "Rust" {
        // All rust functions that we need to interact with the loop event proxy
        type RustEventProxy;
        fn process_json(proxy: &RustEventProxy, json: &CxxString);
        fn print_proxy(boxed: &RustEventProxy, message: &CxxString);
    }

}

type RustEventProxy = winit::event_loop::EventLoopProxy<super::CustomEvent>;

fn print_proxy(proxy: &RustEventProxy, message: &cxx::CxxString) {
    let message = super::CustomEvent::TestMessage(message.to_str().unwrap().to_string());
    proxy.send_event(message);
}

fn process_json(proxy: &RustEventProxy, json: &cxx::CxxString) {
    let rust_str = json.to_str().expect("error validating the json string as UTF8");
    proxy.send_event(super::CustomEvent::JsonScene(rust_str.to_string()));
}

