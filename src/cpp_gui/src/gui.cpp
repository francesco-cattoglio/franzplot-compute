#include "gui.h"

#include "franzplot-compute/src/cpp_gui/mod.rs.h"

#include "imgui.h"

namespace franzplot_gui {

Gui::Gui(rust::Box<RustEventProxy>& boxed_proxy)
    :
        boxed_proxy(std::move(boxed_proxy))
{
    graph.Test(); // creates a few default nodes
}

void Gui::test_boxed_proxy() {
    static int i = 0;
    if (i < 10)
        print_proxy(*boxed_proxy, std::string("Counting up to 10 elapsed frames: ") + std::to_string(i++));
}

void Gui::Render() {
    ImGui::SetNextWindowPos(ImVec2(10, 10), ImGuiCond_FirstUseEver);
    ImGui::SetNextWindowSize(ImVec2(650, 500), ImGuiCond_FirstUseEver);
    ImGui::Begin("simple node editor", nullptr);

    graph.Render(*boxed_proxy);

    ImGui::End();
}

std::unique_ptr<Gui> create_gui_instance(rust::Box<RustEventProxy> boxed_proxy){
    return std::make_unique<Gui>(boxed_proxy);
}

}
