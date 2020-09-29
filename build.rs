fn main() {
    cxx_build::bridge("src/demo.rs")
        .file("src/demo.cpp")
        .include("include")
        .flag_if_supported("-std=c++14")
        .compile("cxxbridge-demo");

    println!("cargo:rerun-if-changed=src/demo.rs");
    println!("cargo:rerun-if-changed=src/demo.cpp");
    println!("cargo:rerun-if-changed=include/demo.h");
}
