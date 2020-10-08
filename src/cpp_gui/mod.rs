#[cxx::bridge(namespace = franzplot_gui)]
pub mod ffi{

    extern "C" {
        include!("library.h");

        fn init_imnodes();
        fn shutdown_imnodes();
        fn show_node_graph();
    }

    extern "Rust" {
        fn process_json(json: &CxxString);
    }
}

fn process_json(json: &cxx::CxxString) {
    let rust_str = json.to_str().expect("error validating the json string as UTF8");
    println!("json on rust side: {}", rust_str);
}

