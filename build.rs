fn main() {
    let imgui_files = vec![
        "src/cpp_gui/imgui-1.75/imgui.cpp",
        "src/cpp_gui/imgui-1.75/imgui_demo.cpp",
        "src/cpp_gui/imgui-1.75/imgui_widgets.cpp",
        "src/cpp_gui/imgui-1.75/imgui_draw.cpp",
        "src/cpp_gui/imnodes-8ecdd3/imnodes.cpp",
    ];
    let cpp_files = vec![
        "src/cpp_gui/src/attribute.cpp",
        "src/cpp_gui/src/library.cpp",
        "src/cpp_gui/src/node.cpp",
    ];

    cxx_build::bridge("src/cpp_gui/mod.rs")
        .files(imgui_files)
        .files(&cpp_files)
        .include("src/cpp_gui/imgui-1.75/")
        .include("src/cpp_gui/imnodes-8ecdd3/")
        .include("src/cpp_gui/include/")
        .flag_if_supported("-std=c++14")
        .compile("cxxbridge-gui");

    println!("cargo:rerun-if-changed=src/demo.rs");
    println!("cargo:rerun-if-changed=src/demo.cpp");
    println!("cargo:rerun-if-changed=include/demo.h");
    println!("cargo:rerun-if-changed=src/cpp_gui/imnodes-8ecdd3/imnodes.cpp");
    println!("cargo:rerun-if-changed=src/cpp_gui/imnodes-8ecdd3/imnodes.h");
    for filename in cpp_files.iter() {
        println!("cargo:rerun-if-changed={}", filename);
    }
}
