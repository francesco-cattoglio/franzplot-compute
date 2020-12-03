fn main() {
    let cpp_files = vec![
        "src/cpp_gui/imnodes-5959729/imnodes.cpp",
        "src/cpp_gui/imnodes_shims.cpp",
    ];
    let include_files = vec![
        "src/cpp_gui/imgui-1.79/imstb_textedit.h",
        "src/cpp_gui/imgui-1.79/imgui_internal.h",
        "src/cpp_gui/imgui-1.79/imconfig.h",
        "src/cpp_gui/imgui-1.79/imgui.h",
        "src/cpp_gui/imnodes-5959729/imnodes.h",
        "src/cpp_gui/imnodes_shims.h",
        "src/cpp_gui/imgui_shims.h",
    ];

    cxx_build::bridge("src/cpp_gui/mod.rs")
        .files(&cpp_files)
        .include("src/cpp_gui/imgui-1.79/")
        .include("src/cpp_gui/imnodes-5959729/")
        .include("src/cpp_gui/include/")
        .compile("cxxbridge-gui");

    // instruct the build system to re-run cxx if any cpp file changes,
    for filename in cpp_files.iter() {
        println!("cargo:rerun-if-changed={}", filename);
    }
    for filename in include_files.iter() {
        println!("cargo:rerun-if-changed={}", filename);
    }
    // also, re-run the build system if the cpp_gui module changes!
    println!("cargo:rerun-if-changed=src/cpp_gui/mod.rs");
}
