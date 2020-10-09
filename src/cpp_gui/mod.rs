#[cxx::bridge(namespace = franzplot_gui)]
pub mod ffi{
    struct SharedThing{
        proxy: Box<WrappedProxy>,
    }

    struct OtherShared {
        proxy: Box<RustProxy>,
    }

    extern "C" {
        include!("library.h");

        type GuiInstance;
        fn init_imnodes();
        fn init_2(boxed_proxy: Box<RustProxy>) -> UniquePtr<GuiInstance>;
        fn shutdown_imnodes();
        fn show_node_graph(state: SharedThing);
        fn do_something(state: SharedThing);

    }

    extern "Rust" {
        type WrappedProxy;
        type RustProxy;
        fn process_json(proxy: &WrappedProxy, json: &CxxString);
        fn print_r(r: &WrappedProxy);
        fn print_proxy(boxed: &RustProxy, message: &CxxString);
    }
}

//pub struct WrappedProxy(pub usize);
fn print_r(r: &WrappedProxy) {
    println!("called back with r={:?}", r.0);
}
fn print_proxy(boxed: &RustProxy, message: &cxx::CxxString) {
    let message = super::CustomEvent::TestMessage(message.to_str().unwrap().to_string());
    boxed.send_event(message);
}
pub struct WrappedProxy(pub std::rc::Rc<winit::event_loop::EventLoopProxy<super::CustomEvent>>);
type RustProxy = winit::event_loop::EventLoopProxy<super::CustomEvent>;

fn process_json(proxy: &WrappedProxy, json: &cxx::CxxString) {
    let rust_str = json.to_str().expect("error validating the json string as UTF8");
    println!("json on rust side: {}", &rust_str);
    let proxy_rc = &proxy.0;
    proxy_rc.send_event(super::CustomEvent::JsonScene(rust_str.to_string()));
}

