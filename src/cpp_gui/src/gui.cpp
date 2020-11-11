#include "gui.h"

#include "franzplot-compute/src/cpp_gui/mod.rs.h"

#include <iostream>
#include <cstring> // for strcmp

#include "imgui.h"

namespace franzplot_gui {

Gui::Gui(rust::Box<RustEventProxy>& boxed_proxy)
    :
        boxed_proxy(std::move(boxed_proxy))
{
    new_var_name.fill('\0');
    graph.Test(); // creates a few default nodes
}

void Gui::test_boxed_proxy() {
}

void Gui::RenderGraphPage() {
    using namespace ImGui;
    bool test_button = ImGui::Button("Render!");
    SameLine(); ImGui::Button("Alongside");
    SameLine(); ImGui::Button("many");
    SameLine(); ImGui::Button("more");
    SameLine(); ImGui::Button("buttons");
    SameLine(); ImGui::Button("& tools");
    if (test_button) {
        // create the json representation of our context+graph
        std::string json_output;
        json_output += std::string() + "{\n"; // opens file
        json_output += std::string() + "\"global_vars\": [\n"; // opens globals
        for (size_t i = 0; i < globals_names.size(); i++) {
            std::string name = globals_names[i].data();
            json_output += std::string() + "\t\"" + name + "\",\n";
        }
        json_output += std::string() + "],\n"; // closes globals, places a comma for descriptors
        json_output += graph.ToJsonDescriptors(); // adds all the descriptors
        json_output += std::string() + "}"; // closes the file

        std::cout << "testing took place:\n" << json_output << std::endl;
        process_json(*boxed_proxy, json_output);
    }
    Columns(2, "graph edit layout columns", false);
    auto size = CalcTextSize("Use this text for sizing!");
    SetColumnWidth(-1, size.x);
    ImGui::Text("Global variables");
    for (size_t i = 0; i < globals_names.size(); i++) {
        PushItemWidth(80);
        ImGui::Text("%s", globals_names[i].data());
        PopItemWidth();
        SameLine();
        PushID(i);
        if (Button("X")) {
            globals_names.erase(globals_names.begin()+i);
            globals_values.erase(globals_values.begin()+i);
            i--;
        }
        PopID();
    }
    PushItemWidth(80);
    InputText("##new_var_input", new_var_name.data(), new_var_name.size());
    PopItemWidth();
    SameLine();
    if (ImGui::Button("New") && ValidVarName(new_var_name)) {
        globals_names.push_back(new_var_name);
        globals_values.push_back(0.0);
        new_var_name[0] = '\0';
    }
    NextColumn();

    graph.Render();
    ImGui::Columns(1);
}

bool Gui::ValidVarName(const VarName& name) {
    // an empty name is not valid
    if (name[0] == '\0')
        return false;

    for (auto& existing_name : globals_names) {
        // an already existing name is not valid
        if (std::strcmp(existing_name.data(), name.data()) == 0)
            return false;
    }

    return true;
}

void Gui::RenderScenePage() {
    using namespace ImGui;
    Columns(2, "scene layout columns", false);
    auto size = CalcTextSize("Use this text for sizing!");
    SetColumnWidth(-1, size.x);
    // first fill the sidebar
    ImGui::Text("Global variables");
    PushItemWidth(80);
    for (size_t i = 0; i < globals_names.size(); i++) {
        DragFloat(globals_names[i].data(), &globals_values[i], 0.01);
    }
    NextColumn();
    auto avail_space = GetContentRegionAvail();
    // we need to leave a little bit of space, otherwise a vertical scrollbar appears
    // maybe this has to do with the imagebutton borders
    ImGui::ImageButton((void*) scene_texture_id, ImVec2(avail_space.x, avail_space.y-6));
    if (ImGui::IsItemActive()){
        ImVec2 value_raw = ImGui::GetMouseDragDelta(ImGuiMouseButton_Left, 0.0f);
        ImGui::ResetMouseDragDelta(ImGuiMouseButton_Left);
        update_scene_camera(*boxed_proxy, value_raw.x, value_raw.y);
    }
    ImGui::Columns(1);
    // TODO: when cxx allows us, use arrays for pushing updated constants!
    std::vector<std::string> globals_strings;
    for (auto& name : globals_names) {
        globals_strings.push_back(std::string(name.data()));
    }
    update_global_vars(*boxed_proxy, globals_strings, globals_values);

}

void Gui::RenderSettingsPage() {
    using namespace ImGui;
    ImGui::Text("Scene settings will be in this tab");
}

void Gui::Render(std::uint32_t x_size, std::uint32_t y_size) {
    using namespace ImGui;
    // main window, that will contain everything
    ImGuiWindowFlags main_window_flags =
        ImGuiWindowFlags_NoTitleBar
        | ImGuiWindowFlags_MenuBar
        | ImGuiWindowFlags_NoResize
        | ImGuiWindowFlags_NoMove;

    ImGui::SetNextWindowSize(ImVec2(x_size, y_size));
    ImGui::SetNextWindowPos(ImVec2(0, 0));
    Begin("main window", nullptr, main_window_flags);
    // menu bar
    if (BeginMenuBar()) {
        MenuItem("File");
        MenuItem("About");
        EndMenuBar();
    }
    ImGuiTabBarFlags tab_bar_flags = ImGuiTabBarFlags_None;
    if (ImGui::BeginTabBar("MyTabBar", tab_bar_flags)) {
        if (ImGui::BeginTabItem("Node graph")) {
            RenderGraphPage();
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("Scene rendering")) {
            RenderScenePage();
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("Scene settings"))
        {
            RenderSettingsPage();
            ImGui::EndTabItem();
        }
        ImGui::EndTabBar();
    }
    End();
    //Begin("global variables", nullptr);
    //End();

    //Begin("scene view", nullptr, ImGuiWindowFlags_NoMove);
    //ImGui::ImageButton((void*) scene_texture_id, ImVec2(400, 300));
    //End();
    //// update the globals.
    //std::vector<std::string> globals_strings;
    //for (auto& name : globals_names) {
    //    globals_strings.push_back(std::string(name.data()));
    //}
    //update_global_vars(*boxed_proxy, globals_strings, globals_values);
}

void Gui::UpdateSceneTexture(std::size_t scene_texture_id){
    this->scene_texture_id = scene_texture_id;
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
