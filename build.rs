fn main() {
    let imgui_files = vec![
        "src/cpp_gui/imgui-1.78/imgui.cpp",
        "src/cpp_gui/imgui-1.78/imgui_widgets.cpp",
        "src/cpp_gui/imgui-1.78/imgui_draw.cpp",
        "src/cpp_gui/imgui-1.78/misc/cpp/imgui_stdlib.cpp",
    ];
    let cpp_files = vec![
        "src/cpp_gui/imnodes-8ecdd3/imnodes.cpp",
        "src/cpp_gui/src/attribute.cpp",
        "src/cpp_gui/src/library.cpp",
        "src/cpp_gui/src/graph.cpp",
        "src/cpp_gui/src/node.cpp",
        "src/cpp_gui/src/gui.cpp",
    ];

    let include_files = vec![
        "src/cpp_gui/imnodes-8ecdd3/imnodes.h",
        "src/cpp_gui/include/library.h",
        "src/cpp_gui/include/attribute.h",
        "src/cpp_gui/include/graph.h",
        "src/cpp_gui/include/node.h",
        "src/cpp_gui/include/gui.h",
    ];

    cxx_build::bridge("src/cpp_gui/mod.rs")
        .files(&imgui_files)
        .files(&cpp_files)
        .include("src/cpp_gui/imgui-1.78/")
        .include("src/cpp_gui/imnodes-8ecdd3/")
        .include("src/cpp_gui/include/")
        .flag("-std=c++17")
        .compile("cxxbridge-gui");

    // instruct the build system to re-run cxx if any cpp file changes,
    for filename in imgui_files.iter() {
        println!("cargo:rerun-if-changed={}", filename);
    }
    for filename in cpp_files.iter() {
        println!("cargo:rerun-if-changed={}", filename);
    }
    for filename in include_files.iter() {
        println!("cargo:rerun-if-changed={}", filename);
    }
}
