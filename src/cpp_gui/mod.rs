#[cxx::bridge(namespace = franzplot_gui)]
pub mod ffi{

    extern "C" {
        include!("library.h");

        type GuiInstance;
        fn init_imnodes();
        fn init_2(boxed_proxy: Box<RustEventProxy>) -> UniquePtr<GuiInstance>;
        fn shutdown_imnodes();
        //fn show_node_graph(state: SharedThing);
        //fn do_something(state: SharedThing);

        fn test_boxed_proxy(self: &mut GuiInstance);
    }

    extern "Rust" {
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

