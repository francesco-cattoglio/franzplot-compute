use std::io;
#[cfg(windows)] use winres::WindowsResource;

fn main() -> io::Result<()> {
    let cpp_files = vec![
        "src/cpp_gui/imnodes-ee6d407/imnodes.cpp",
        "src/cpp_gui/imnodes_shims.cpp",
    ];
    let include_files = vec![
        "src/cpp_gui/imgui-1.80/imstb_truetype.h",
        "src/cpp_gui/imgui-1.80/imstb_textedit.h",
        "src/cpp_gui/imgui-1.80/imgui_internal.h",
        "src/cpp_gui/imgui-1.80/imstb_rectpack.h",
        "src/cpp_gui/imgui-1.80/imconfig.h",
        "src/cpp_gui/imgui-1.80/imgui.h",
        "src/cpp_gui/imnodes-ee6d407/imnodes.h",
        "src/cpp_gui/imnodes_shims.h",
        "src/cpp_gui/imgui_shims.h",
    ];

    cxx_build::bridge("src/cpp_gui/mod.rs")
        .files(&cpp_files)
        .include("src/cpp_gui/imgui-1.80/")
        .include("src/cpp_gui/imnodes-ee6d407/")
        .flag_if_supported("-std=c++11")
        .compile("cxxbridge-gui");

    #[cfg(windows)] {
        WindowsResource::new()
        // This path can be absolute, or relative to your crate root.
        .set_icon("compile_resources/icon_256.ico")
        .compile()?;
    }

    // instruct the build system to re-run cxx if any cpp file changes,
    for filename in cpp_files.iter() {
        println!("cargo:rerun-if-changed={}", filename);
    }
    for filename in include_files.iter() {
        println!("cargo:rerun-if-changed={}", filename);
    }
    // also, re-run the build system if the cpp_gui module changes!
    println!("cargo:rerun-if-changed=src/cpp_gui/mod.rs");

    Ok(())
}
