fn main() {
    let imgui_files = vec![
        "src/cpp_gui/imgui-1.75/imgui.cpp",
        "src/cpp_gui/imgui-1.75/imgui_demo.cpp",
        "src/cpp_gui/imgui-1.75/imgui_widgets.cpp",
        "src/cpp_gui/imgui-1.75/imgui_draw.cpp",
        "src/cpp_gui/imnodes-8ecdd3/imnodes.cpp",
    ];

    cxx_build::bridge("src/demo.rs")
        .file("src/demo.cpp")
        .files(imgui_files)
        .include("include")
        .include("src/cpp_gui/imgui-1.75/")
        .flag_if_supported("-std=c++14")
        .compile("cxxbridge-demo");

    println!("cargo:rerun-if-changed=src/demo.rs");
    println!("cargo:rerun-if-changed=src/demo.cpp");
    println!("cargo:rerun-if-changed=include/demo.h");
    println!("cargo:rerun-if-changed=src/cpp_gui/imnodes-8ecdd3/imnodes.cpp");
}
