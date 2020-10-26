#include "gui.h"

#include "franzplot-compute/src/cpp_gui/mod.rs.h"

#include <iostream>

#include "imgui.h"

namespace franzplot_gui {

Gui::Gui(rust::Box<RustEventProxy>& boxed_proxy)
    :
        boxed_proxy(std::move(boxed_proxy))
{
    new_globals_name.fill('\0');
    graph.Test(); // creates a few default nodes
}

void Gui::test_boxed_proxy() {
}

void Gui::Render() {
    using namespace ImGui;
    Begin("global variables", nullptr);
    PushItemWidth(80);
    for (size_t i = 0; i < globals_names.size(); i++) {
        DragFloat(globals_names[i].data(), &globals_values[i], 0.01);
    }
    InputText("##new_var_input", new_globals_name.data(), new_globals_name.size());
    PopItemWidth();
    SameLine();
    if (ImGui::Button("Add new variable") && new_globals_name[0] != '\0') {
        globals_names.push_back(new_globals_name);
        globals_values.push_back(0.0);
        new_globals_name[0] = '\0';
    }
    End();

    // update the globals.
    std::vector<std::string> globals_strings;
    for (auto& name : globals_names) {
        globals_strings.push_back(std::string(name.data()));
    }
    update_global_vars(*boxed_proxy, globals_strings, globals_values);
    SetNextWindowPos(ImVec2(10, 10), ImGuiCond_FirstUseEver);
    SetNextWindowSize(ImVec2(650, 500), ImGuiCond_FirstUseEver);
    ImGui::Begin("simple node editor", nullptr);

    bool test_button = ImGui::Button("gotest!");
    if (test_button) {
        // create the json representation of our context+graph
        std::string json_output;
        json_output += std::string() + "{\n"; // opens file
        json_output += std::string() + "\"global_vars\": [\n"; // opens globals
        for (size_t i = 0; i < globals_names.size(); i++) {
            std::string& name = globals_strings[i];
            json_output += std::string() + "\t\"" + name + "\",\n";
        }
        json_output += std::string() + "],\n"; // closes globals, places a comma for descriptors
        json_output += graph.ToJsonDescriptors(); // adds all the descriptors
        json_output += std::string() + "}"; // closes the file

        std::cout << "testing took place:\n" << json_output << std::endl;
        process_json(*boxed_proxy, json_output);
    }

    graph.Render();

    ImGui::End();
}

void Gui::ClearAllMarks() {
    graph.ClearAllMarks();
}

void Gui::MarkClean(int id) {
    Node* maybe_node = graph.GetNode(id);
    if (maybe_node)
        maybe_node->SetStatus(NodeStatus::Ok, "Ok");
}

void Gui::MarkError(int id, rust::Str rust_message) {
    std::string message(rust_message);
    Node* maybe_node = graph.GetNode(id);
    if (maybe_node)
        maybe_node->SetStatus(NodeStatus::Error, message);
}

void Gui::MarkWarning(int id, rust::Str rust_message) {
    std::string message(rust_message);
    Node* maybe_node = graph.GetNode(id);
    if (maybe_node)
        maybe_node->SetStatus(NodeStatus::Warning, message);
}

std::unique_ptr<Gui> create_gui_instance(rust::Box<RustEventProxy> boxed_proxy){
    return std::make_unique<Gui>(boxed_proxy);
}

}
