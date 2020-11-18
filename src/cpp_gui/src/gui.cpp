#include "gui.h"

#include "franzplot-compute/src/cpp_gui/mod.rs.h"

#include <iostream>
#include <cstring> // for strcmp

#include "imgui.h"

namespace franzplot_gui {

Gui::Gui() {
    new_var_name.fill('\0');
    graph.Test(); // creates a few default nodes
}

void Gui::RenderGraphPage(State& rust_state) {
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
        json_output += std::string() + "\"global_names\": [\n"; // opens global names
        for (size_t i = 0; i < globals_names.size(); i++) {
            std::string name = globals_names[i].data();
            json_output += std::string() + "\t\"" + name + "\",\n";
        }
        json_output += std::string() + "],\n"; // closes global names,
        json_output += std::string() + "\"global_init_values\": [\n"; // opens global init vals
        for (size_t i = 0; i < globals_names.size(); i++) {
            json_output += std::string() + "\t" + std::to_string(0.0) + ",\n";
        }
        json_output += std::string() + "],\n"; // closes globals init vals, places a comma for descriptors
        json_output += graph.ToJsonDescriptors(); // adds all the descriptors
        json_output += std::string() + "}"; // closes the file

        std::cout << "testing took place:\n" << json_output << std::endl;
        ClearAllMarks();
        auto graph_errors = process_json(rust_state, json_output);
        for (auto& error : graph_errors) {
            if (error.is_warning) {
                MarkWarning(error.node_id, error.message);
            } else {
                MarkError(error.node_id, error.message);
            }
        }
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

GuiRequests Gui::RenderScenePage(State& rust_state) {
    using namespace ImGui;
    ImGuiMouseCursor mouse_cursor = ImGuiMouseCursor_Arrow;
    GuiRequests to_return {0, 0, false};
    Columns(2, "scene layout columns", false);
    auto size = CalcTextSize("Use this text for sizing!");
    SetColumnWidth(-1, size.x);
    // first fill the sidebar
    ImGui::Text("Global variables");
    PushItemWidth(80);
    // fetch global variables' names and values from rust state
    auto& globals_names_ref = get_globals_names(rust_state);
    auto& globals_values_ref = get_globals_values(rust_state);
    // and add the UI for updating them
    for (size_t i = 0; i < globals_names_ref.size(); i++) {
        std::string name(globals_names_ref[i]);
        float* value_ptr = globals_values_ref.data() + i;
        DragFloat(name.c_str(), value_ptr, 0.01);
        if (ImGui::IsItemHovered()) {
            mouse_cursor = ImGuiMouseCursor_ResizeEW;
        }
    }

    NextColumn();
    auto avail_space = GetContentRegionAvail();
    // we need to leave a little bit of space, otherwise a vertical scrollbar appears
    // maybe this has to do with the imagebutton borders
    ImGui::ImageButton((void*) scene_texture_id, ImVec2(avail_space.x, avail_space.y), ImVec2(0, 0), ImVec2(1,1), 0);
    // We need to communicate to Winit where we want to lock the mouse. This is because
    // we use a proxy to communicate, and that always takes a frame.
    if (ImGui::IsItemHovered()) {
        mouse_cursor = ImGuiMouseCursor_Arrow;
    }
    if (ImGui::IsItemActivated()) {
        ImVec2 mouse_position = ImGui::GetMousePos();
        to_return.freeze_mouse = true;
        to_return.frozen_mouse_x = mouse_position.x;
        to_return.frozen_mouse_y = mouse_position.y;
    } else if (ImGui::IsItemActive()){
        // Since we reset the cursor via Winit, the delta for each frame is exactly
        // the amount taht we would like the camera to move!
        ImVec2 mouse_delta = GetMouseDragDelta(0, 0.0f);
        ImVec2 mouse_position = ImGui::GetMousePos();
        to_return.freeze_mouse = true;
        to_return.frozen_mouse_x = mouse_position.x - mouse_delta.x;
        to_return.frozen_mouse_y = mouse_position.y - mouse_delta.y;

        update_scene_camera(rust_state, mouse_delta.x, mouse_delta.y);
    } else if (ImGui::IsItemDeactivated()) {
        to_return.freeze_mouse = false;
    }
    ImGui::Columns(1);
    // TODO: when cxx allows us, use arrays for pushing updated constants!
    std::vector<std::string> globals_strings;
    for (auto& name : globals_names) {
        globals_strings.push_back(std::string(name.data()));
    }
    SetMouseCursor(mouse_cursor);

    return to_return;
}

void Gui::RenderSettingsPage(State& rust_state) {
    using namespace ImGui;
    ImGui::Text("Scene settings will be in this tab");
}

GuiRequests Gui::Render(State& rust_state, std::uint32_t x_size, std::uint32_t y_size) {
    using namespace ImGui;
    GuiRequests to_return {0, 0, false};
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
            RenderGraphPage(rust_state);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("Scene rendering")) {
            to_return = RenderScenePage(rust_state);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("Scene settings"))
        {
            RenderSettingsPage(rust_state);
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
    return to_return;
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

void Gui::MarkError(int id, const rust::String& rust_message) {
    std::string message(rust_message);
    Node* maybe_node = graph.GetNode(id);
    if (maybe_node)
        maybe_node->SetStatus(NodeStatus::Error, message);
}

void Gui::MarkWarning(int id, const rust::String& rust_message) {
    std::string message(rust_message);
    Node* maybe_node = graph.GetNode(id);
    if (maybe_node)
        maybe_node->SetStatus(NodeStatus::Warning, message);
}

std::unique_ptr<Gui> create_gui_instance() {
    return std::make_unique<Gui>();
}

}
