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
        fn MarkError(self: &mut Gui, node_id: i32, message: &str);
        fn test_boxed_proxy(self: &mut Gui);
    }

    extern "Rust" {
        // All rust functions that we need to interact with the loop event proxy
        type RustEventProxy;
        fn process_json(proxy: &RustEventProxy, json: &CxxString);
        fn print_proxy(boxed: &RustEventProxy, message: &CxxString);
        fn update_global_vars(proxy: &RustEventProxy, names: &CxxVector<CxxString>, values: &CxxVector<f32>);
    }

}

type RustEventProxy = winit::event_loop::EventLoopProxy<super::CustomEvent>;

fn print_proxy(proxy: &RustEventProxy, message: &cxx::CxxString) {
    let message = super::CustomEvent::TestMessage(message.to_str().unwrap().to_string());
    proxy.send_event(message);
}

fn update_global_vars(proxy: &RustEventProxy, names: &cxx::CxxVector<cxx::CxxString>, values: &cxx::CxxVector<f32>) {
    let zip_iter = names.into_iter().zip(values.into_iter());
    let mut list = Vec::<(String, f32)>::new();
    for (c_name, value) in zip_iter {
        let string = c_name.to_string();
        list.push((string, *value));
    }
    proxy.send_event(super::CustomEvent::UpdateGlobals(list));
}

fn set_global_vars(proxy: &RustEventProxy, names: &cxx::CxxVector<cxx::CxxString>, values: &cxx::CxxVector<f32>) {
    let zip_iter = names.into_iter().zip(values.into_iter());
    use std::collections::BTreeMap;
    let mut map = BTreeMap::<String, f32>::new();
    for (c_name, value) in zip_iter {
        let string = c_name.to_string();
        map.insert(string, *value);
    }
    proxy.send_event(super::CustomEvent::SetGlobals(map));
}

fn process_json(proxy: &RustEventProxy, json: &cxx::CxxString) {
    let rust_str = json.to_str().expect("error validating the json string as UTF8");
    proxy.send_event(super::CustomEvent::JsonScene(rust_str.to_string()));
}

