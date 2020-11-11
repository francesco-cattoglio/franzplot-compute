#include "library.h"

#include "franzplot-compute/src/cpp_gui/mod.rs.h"

#include <imgui.h>
#include <imnodes.h>
#include <iostream>

#include "gui.h"

namespace franzplot_gui {

void init_imnodes() {
    imnodes::Initialize();
    auto& global_style = ImGui::GetStyle();
    global_style.WindowRounding = 0.0f; // square window borders
}

void shutdown_imnodes() {
    imnodes::Shutdown();
}

} // namespace franzplot_gui
