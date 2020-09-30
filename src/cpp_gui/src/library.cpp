#include "library.h"
#include "franzplot-compute/src/cpp_gui/mod.rs.h"
#include <imgui.h>
#include <imnodes.h>
#include <iostream>

namespace org {
namespace example {

ThingC::ThingC(std::string appname) : appname(std::move(appname)) {}

ThingC::~ThingC() { std::cout << "done with ThingC" << std::endl; }

std::unique_ptr<ThingC> make_demo(rust::Str appname) {
  return std::make_unique<ThingC>(std::string(appname));
}

static bool show_another_window = false;
static ImVec4 clear_color = ImVec4(0.45f, 0.55f, 0.60f, 1.00f);

void init_imnodes() {
    imnodes::Initialize();
}
void shutdown_imnodes() {
    imnodes::Shutdown();
}

void show_node_graph() {
        ImGui::SetNextWindowPos(ImVec2(10, 10), ImGuiCond_FirstUseEver);
        ImGui::SetNextWindowSize(ImVec2(650, 500), ImGuiCond_FirstUseEver);
        ImGui::Begin("simple node editor", nullptr, ImGuiWindowFlags_NoTitleBar);

        imnodes::BeginNodeEditor();
        {
            imnodes::BeginNode(1);

            imnodes::BeginNodeTitleBar();
            ImGui::TextUnformatted("simple node :)");
            imnodes::EndNodeTitleBar();

            imnodes::BeginInputAttribute(2);
            ImGui::Text("input");
            imnodes::EndInputAttribute();

            imnodes::BeginOutputAttribute(3);
            ImGui::Indent(40);
            ImGui::Text("output");
            imnodes::EndOutputAttribute();

            imnodes::EndNode();
        }
        auto test = new TextAttribute(15);
        test->Render();
        delete test;
        {
            imnodes::BeginNode(2);

            imnodes::BeginNodeTitleBar();
            ImGui::TextUnformatted("second node");
            imnodes::EndNodeTitleBar();

            imnodes::BeginInputAttribute(4);
            ImGui::Text("input");
            imnodes::EndInputAttribute();

            imnodes::BeginOutputAttribute(0);
            ImGui::Indent(40);
            ImGui::Text("output");
            imnodes::EndOutputAttribute();

            imnodes::EndNode();
        }
        imnodes::Link(14, 0, 2);

        imnodes::EndNodeEditor();

      ImGui::End();
}

const std::string &get_name(const ThingC &thing) { return thing.appname; }

void do_thing(SharedThing state) { print_r(*state.y); }

} // namespace example
} // namespace org
