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

ThingC::ThingC(std::string appname) : appname(std::move(appname)) {}

ThingC::~ThingC() { std::cout << "done with ThingC" << std::endl; }

std::unique_ptr<ThingC> make_demo(rust::Str appname) {
  return std::make_unique<ThingC>(std::string(appname));
}

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

void show_node_graph() {
    ImGui::SetNextWindowPos(ImVec2(10, 10), ImGuiCond_FirstUseEver);
    ImGui::SetNextWindowSize(ImVec2(650, 500), ImGuiCond_FirstUseEver);
    ImGui::Begin("simple node editor", nullptr);

    globals.graph->Render();

    ImGui::End();
}

const std::string &get_name(const ThingC &thing) { return thing.appname; }

void do_thing(SharedThing state) { print_r(*state.y); }

} // namespace franzplot_gui
