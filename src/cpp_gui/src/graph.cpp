#include "graph.h"

#include "franzplot-compute/src/cpp_gui/mod.rs.h"

#include <iostream>
#include <imgui.h>
#include <imnodes.h>
#include <misc/cpp/imgui_stdlib.h>

namespace franzplot_gui {

void Graph::Render() {
    using namespace ImGui;

    imnodes::BeginNodeEditor();

    // render all links
    for (auto& entry : this->input_to_output_links) {
        int link_id = entry.first;
        int in_attribute_id = link_id;
        int out_attribute_id = entry.second;
        imnodes::Link(link_id, in_attribute_id, out_attribute_id);
    }

    for (auto& entry : this->nodes) {
        entry.second.Render();
    }

    imnodes::EndNodeEditor();

    // event processing
    ImVec2 mouse_delta = ImGui::GetMouseDragDelta(ImGuiMouseButton_Right, 4.0);
    const bool right_click_popup =
        ImGui::IsWindowFocused(ImGuiFocusedFlags_RootAndChildWindows) &&
        !ImGui::IsAnyItemHovered() &&
        ImGui::IsMouseReleased(ImGuiMouseButton_Right) &&
        mouse_delta.x == 0 && mouse_delta.y == 0; // exact comparison is fine due to GetMouseDragDelta threshold

    int hovered_id = -1;
    if (right_click_popup) {
        if (imnodes::IsNodeHovered(&hovered_id)) { // handle right-click on nodes
            last_hovered_node = hovered_id;
            ImGui::OpenPopup("Node Menu");
        } else if (imnodes::IsLinkHovered(&hovered_id)) { // handle right-click on links
            // handle right-click on nodes
            last_hovered_link = hovered_id;
            ImGui::OpenPopup("Link Menu");
        } else {
            // handle creation of new nodes
            ImGui::OpenPopup("Add node");
        }
    }
    // handle right click on nodes
    bool workaround_open_node = false;
    if (ImGui::BeginPopup("Node Menu")) {
        if (ImGui::MenuItem("Delete Node")) {
            this->RemoveNode(last_hovered_node);
            last_hovered_node = -1;
        }
        if (ImGui::MenuItem("Rename Node")) {
            workaround_open_node = true;
        }
        ImGui::EndPopup();
    }

    std::string new_name;
    if (workaround_open_node)
        OpenPopup("Edit Node Name");
    if (BeginPopup("Edit Node Name")) {
        if (InputText("new name", &new_name, ImGuiInputTextFlags_EnterReturnsTrue)) {
            this->nodes.at(last_hovered_node).name = new_name;
            CloseCurrentPopup();
        }
        EndPopup();
    }

    if (ImGui::BeginPopup("Link Menu")) {
        if (ImGui::MenuItem("Delete link")) {
            this->input_to_output_links.erase(last_hovered_link);
            last_hovered_link = -1;
        }
        ImGui::EndPopup();
    }

    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(8.f, 8.f));
    if (ImGui::BeginPopup("Add node")) {
        const ImVec2 click_pos = ImGui::GetMousePosOnOpeningCurrentPopup();

        if (ImGui::MenuItem("Interval")) {
            AddNode(Node::PrefabInterval(std::bind(&Graph::NextId, this)), click_pos);
        }

        if (ImGui::BeginMenu("Geometries")) {
            if (ImGui::MenuItem("Curve")) {
                AddNode(Node::PrefabCurve(std::bind(&Graph::NextId, this)), click_pos);
            }
            if (ImGui::MenuItem("Surface")) {
                AddNode(Node::PrefabSurface(std::bind(&Graph::NextId, this)), click_pos);
            }
            ImGui::EndMenu();
        }

        if (ImGui::BeginMenu("Transformations")) {
            if (ImGui::MenuItem("Matrix")) {
                AddNode(Node::PrefabMatrix(std::bind(&Graph::NextId, this)), click_pos);
            }
            if (ImGui::MenuItem("Transform")) {
                AddNode(Node::PrefabTransform(std::bind(&Graph::NextId, this)), click_pos);
            }
            ImGui::EndMenu();
        }

        if (ImGui::MenuItem("Rendering")) {
            AddNode(Node::PrefabRendering(std::bind(&Graph::NextId, this)), click_pos);
        }

        ImGui::EndPopup();
    }
    ImGui::PopStyleVar();


    // check if a link was destroyed;
    int link_id;
    if (imnodes::IsLinkDestroyed(&link_id)) {
        this->input_to_output_links.erase(link_id);
    }

    int start_attr, end_attr;
    if (imnodes::IsLinkCreated(&start_attr, &end_attr)) {
        // check which one of the two attributes is the input attribute and which is the output
        int in_attr, out_attr;
        if (this->attributes[start_attr]->kind == AttributeKind::Input) {
            in_attr = start_attr;
            out_attr = end_attr;
        } else {
            in_attr = end_attr;
            out_attr = start_attr;
        }
        // check if the output can be linked to this input.
        // If the two are compatible, create a link
        if (IsCompatible(static_cast<InputAttribute&>(*attributes[in_attr]), static_cast<OutputAttribute&>(*attributes[out_attr])))
            this->input_to_output_links[in_attr] = out_attr;
    }

}

void Graph::ClearAllMarks() {
    for (auto& entry : nodes) {
        entry.second.SetStatus(NodeStatus::Ok, "Ok");
    }
}

int Graph::NextId() {
    return this->next_id++;
}

void Graph::Test() {
    AddNode(Node::PrefabInterval(std::bind(&Graph::NextId, this)), {10.0, 10.0});
    AddNode(Node::PrefabRendering(std::bind(&Graph::NextId, this)), {200.0, 10.0});
}

void Graph::RecurseToJson(const Node& node, std::set<int>& visited_nodes, std::string& json) {
    // if the node has ANY input, recurse
    for (auto& attribute_ptr : node.attributes) {
        if (attribute_ptr->kind == AttributeKind::Input) {
            auto maybe_node_id = FindLinkedNode(attribute_ptr->id);
            if (maybe_node_id) {
                auto& node = nodes.at(maybe_node_id.value());
                RecurseToJson(node, visited_nodes, json);
            } else {
                std::cout << "warning: unconnected node" << std::endl;
            }
        }
    }

    // after we are done with the recursion, store myself in the json string, provided this has not been done before
    // this code is a bit monolithic because link information is stored inside the graph, not in the attributes,
    // therefore we cannot just loop over all the attributes and call one of their member functions
    if (visited_nodes.count(node.id) == 0) {
        json += std::string() + "{\n"; // opens new node entry
        json += std::string() + "\t\"id\": " + std::to_string(node.id) + ",\n";
        json += std::string() + "\t\"data\": {\n"; // opens the data section
        json += std::string() + "\t\t\"" + ToString(node.type) + "\": {\n"; // opens the attribute section
        for (auto& attribute : node.attributes) {
            std::optional<int> maybe_node_id;
            switch (attribute->kind) {
                case AttributeKind::Input:
                    maybe_node_id = FindLinkedNode(attribute->id);
                    if (maybe_node_id) {
                        json += std::string() + "\t\t\t\"" + attribute->label + "\": " + std::to_string(maybe_node_id.value()) + ",\n";
                    } else {
                        json += std::string() + "\t\t\t\"" + attribute->label + "\": null,\n";
                    }
                    break;
                case AttributeKind::Output:
                    // do nothing
                    break;
                case AttributeKind::Static:
                    json += std::string() + "\t\t\t\"" + attribute->label + "\": " + static_cast<StaticAttribute&>(*attribute).ContentsToJson() + ",\n";
                    break;
            }
        }
        json += std::string() + "\t\t}\n"; // closes attribute
        json += std::string() + "\t}\n"; // closes data
        json += std::string() + "},\n"; // closes the new node entry
        // mark as visited!
        visited_nodes.insert(node.id);
    }
}

std::optional<int> Graph::FindLinkedNode(int input_attribute_id) {
    auto find_results = input_to_output_links.find(input_attribute_id);
    if (find_results != input_to_output_links.end()) {
        int linked_attribute_id = find_results->second;
        return attributes[linked_attribute_id]->node_id;
    } else {
        return std::nullopt;
    }
}

std::string Graph::ToJsonDescriptors() {
    std::string to_return;
    std::set<int> visited_nodes;

    to_return += "\"descriptors\": [\n";
    for (auto& id_node_pair : nodes) {
        if (id_node_pair.second.type == NodeType::Rendering)
            RecurseToJson(id_node_pair.second, visited_nodes, to_return);
    }
    to_return += "]\n"; // closes descriptors array

    return to_return;
}

void Graph::AddNode(Node&& node, const ImVec2& position) {
    // add all the node attributes to our attribute map.
    // those are shared pointers so copying is OK
    for (std::shared_ptr<Attribute> attribute : node.attributes)
        this->attributes[attribute->id] = attribute;

    imnodes::SetNodeScreenSpacePos(node.id, position);
    nodes.insert(std::make_pair(node.id, node));
}

void Graph::RemoveNode(int node_id) {
    // remove all the attributes of the node from our attribute map,
    // and remember to remove all incoming AND outgoing links as well. To achieve this we currently
    // have to go over all the existing links, because we do not have a output_to_input_links,
    // though we might get one in the future
    for (std::shared_ptr<Attribute> attribute : nodes.at(node_id).attributes) {
        int attribute_id = attribute->id; // temporary store this attribute id
        this->attributes.erase(attribute_id);
        for (auto it = input_to_output_links.begin(); it != input_to_output_links.end(); ) {
            if (it->first == attribute_id || it->second == attribute_id) {
                it = input_to_output_links.erase(it);
            } else {
                ++it;
            }
        }
    }

    nodes.erase(node_id);
}

Node* Graph::GetNode(int id) {
    auto it = nodes.find(id);
    if (it == nodes.end())
        return nullptr;
    else
        return &(it->second);
}

}
