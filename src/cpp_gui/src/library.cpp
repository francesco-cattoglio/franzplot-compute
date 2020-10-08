#include "library.h"

#include "franzplot-compute/src/cpp_gui/mod.rs.h"

#include <imgui.h>
#include <imnodes.h>
#include <iostream>

#include "attribute.h"
#include "node.h"
#include "globals.h"
#include "graph.h"

namespace franzplot_gui {

void init_imnodes() {
    imnodes::Initialize();
    imnodes::PushAttributeFlag(imnodes::AttributeFlags_EnableLinkDetachWithDragClick);
    globals.graph = new Graph();
    globals.graph->Test();
}
void shutdown_imnodes() {
    delete globals.graph;
    imnodes::PopAttributeFlag();
    imnodes::Shutdown();
}

void show_node_graph(SharedThing state) {
    ImGui::SetNextWindowPos(ImVec2(10, 10), ImGuiCond_FirstUseEver);
    ImGui::SetNextWindowSize(ImVec2(650, 500), ImGuiCond_FirstUseEver);
    ImGui::Begin("simple node editor", nullptr);

    globals.graph->Render(state);

    ImGui::End();
}

void do_something(SharedThing state) { print_r(*state.proxy); }

} // namespace franzplot_gui
