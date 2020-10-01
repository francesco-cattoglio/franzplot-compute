#include "library.h"

#include "franzplot-compute/src/cpp_gui/mod.rs.h"
#include <imgui.h>
#include <imnodes.h>
#include <iostream>

#include "attribute.h"
#include "node.h"
#include "globals.h"

namespace franzplot_gui {

ThingC::ThingC(std::string appname) : appname(std::move(appname)) {}

ThingC::~ThingC() { std::cout << "done with ThingC" << std::endl; }

std::unique_ptr<ThingC> make_demo(rust::Str appname) {
  return std::make_unique<ThingC>(std::string(appname));
}

void init_imnodes() {
    imnodes::Initialize();
    imnodes::PushAttributeFlag(imnodes::AttributeFlags_EnableLinkDetachWithDragClick);
    add_node(Node::TemplatedCurve());
    add_node(Node::TemplatedCurve());
    add_node(Node::TemplatedCurve());
}
void shutdown_imnodes() {
    imnodes::Shutdown();
    imnodes::PopAttributeFlag();
}

int new_id() {
    return globals.next_id++;
}

void add_node(Node&& node) {
    // we need to keep our attribute-to-node map up-to-date
    for (auto& attribute : node.in_attributes)
        globals.attr_node_map[attribute->id] = node.id;

    for (auto& attribute : node.out_attributes)
        globals.attr_node_map[attribute->id] = node.id;

    for (auto& attribute : node.static_attributes)
        globals.attr_node_map[attribute->id] = node.id;

    globals.nodes.insert(std::make_pair(node.id, node));
}

void show_node_graph() {
    ImGui::SetNextWindowPos(ImVec2(10, 10), ImGuiCond_FirstUseEver);
    ImGui::SetNextWindowSize(ImVec2(650, 500), ImGuiCond_FirstUseEver);
    ImGui::Begin("simple node editor", nullptr, ImGuiWindowFlags_NoTitleBar);

    imnodes::BeginNodeEditor();
    // render all links
    for (auto& entry : globals.links) {
        int link_idx = entry.first;
        auto attr_pair = entry.second;
        imnodes::Link(link_idx, attr_pair.first, attr_pair.second);
    }

    for (auto& entry : globals.nodes) {
        entry.second.Render();
    }

    imnodes::EndNodeEditor();

    // event processing

    int start_attr, end_attr;
    if (imnodes::IsLinkCreated(&start_attr, &end_attr))
    {
        auto attr_pair = std::make_pair(start_attr, end_attr);
        globals.links.insert(std::make_pair(new_id(), attr_pair));
    }
    ImGui::End();

    int link_id;
    if (imnodes::IsLinkDestroyed(&link_id)) {
        globals.links.erase(link_id);
    }
}

const std::string &get_name(const ThingC &thing) { return thing.appname; }

void do_thing(SharedThing state) { print_r(*state.y); }

} // namespace franzplot_gui
