#[cxx::bridge(namespace = franzplot_gui)]
pub mod ffi{
    struct SharedThing{
        proxy: Box<WrappedProxy>,
    }

    extern "C" {
        include!("library.h");

        fn init_imnodes();
        fn shutdown_imnodes();
        fn show_node_graph(state: SharedThing);
        fn do_something(state: SharedThing);

    }

    extern "Rust" {
        type WrappedProxy;
        fn process_json(proxy: &WrappedProxy, json: &CxxString);
        fn print_r(r: &WrappedProxy);
    }
}

//pub struct WrappedProxy(pub usize);
fn print_r(r: &WrappedProxy) {
    println!("called back with r={:?}", r.0);
}
pub struct WrappedProxy(pub std::rc::Rc<winit::event_loop::EventLoopProxy<super::CustomEvent>>);

fn process_json(proxy: &WrappedProxy, json: &cxx::CxxString) {
    let rust_str = json.to_str().expect("error validating the json string as UTF8");
    println!("json on rust side: {}", &rust_str);
    let proxy_rc = &proxy.0;
    proxy_rc.send_event(super::CustomEvent::JsonScene(rust_str.to_string()));
}

