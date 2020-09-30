#[cxx::bridge(namespace = org::example)]
pub mod ffi{
    struct SharedThing {
        z: i32,
        y: Box<ThingR>,
        x: UniquePtr<ThingC>,
    }

    extern "C" {
        include!("demo.h");

        type ThingC;
        fn make_demo(appname: &str) -> UniquePtr<ThingC>;
        fn get_name(thing: &ThingC) -> &CxxString;
        fn do_thing(state: SharedThing);
        fn init_imnodes();
        fn shutdown_imnodes();
        fn show_node_graph();
    }

    extern "Rust" {
        type ThingR;
        fn print_r(r: &ThingR);
    }
}
pub struct ThingR(usize);

pub fn print_r(r: &ThingR) {
    println!("called back with r={}", r.0);
}

