#[cxx::bridge(namespace = franzplot_gui)]
pub mod ffi{
    struct SharedThing{
        proxy: Box<WrappedProxy>,
    }

    extern "C" {
        include!("library.h");

        fn init_imnodes();
        fn shutdown_imnodes();
        fn show_node_graph();
        fn do_something(state: SharedThing);

    }

    extern "Rust" {
        type WrappedProxy;
        fn process_json(proxy: &WrappedProxy, json: &CxxString);
        fn print_r(r: &WrappedProxy);
    }
}

pub struct WrappedProxy(pub usize);
fn print_r(r: &WrappedProxy) {
    println!("called back with r={}", r.0);
}
//pub struct WrappedProxy(winit::event_loop::EventLoopProxy<super::CustomEvent>);

fn process_json(proxy: &WrappedProxy, json: &cxx::CxxString) {
    let rust_str = json.to_str().expect("error validating the json string as UTF8");
    println!("json on rust side: {}", rust_str);
}

