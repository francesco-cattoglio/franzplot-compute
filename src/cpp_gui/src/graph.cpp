#include "graph.h"

#include "franzplot-compute/src/cpp_gui/mod.rs.h"

#include <iostream>
#include <imgui.h>
#include <imnodes.h>

namespace franzplot_gui {

void Graph::Render() {
    bool test_button = ImGui::Button("gotest!");
    if (test_button) {
        std::string json_output = this->ToJson();
        std::cout << "testing took place: " << json_output << std::endl;
//        process_json(*(state.proxy), json_output);
    }

    bool open_file_button = ImGui::Button("load from file");
    if (open_file_button) {
        std::cout << "loading a scene from file" << std::endl;
    }

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

int Graph::NextId() {
    return this->next_id++;
}

void Graph::Test() {
    AddNode(Node::PrefabInterval(std::bind(&Graph::NextId, this)));
    AddNode(Node::PrefabInterval(std::bind(&Graph::NextId, this)));
    AddNode(Node::PrefabSurface(std::bind(&Graph::NextId, this)));
    AddNode(Node::PrefabRendering(std::bind(&Graph::NextId, this)));
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
        json += std::string() + "\t\"id\": \"" + std::to_string(node.id) + "\",\n";
        json += std::string() + "\t\"data\": {\n"; // opens the data section
        json += std::string() + "\t\t\"" + ToString(node.type) + "\": {\n"; // opens the attribute section
        for (auto& attribute : node.attributes) {
            std::optional<int> maybe_node_id;
            switch (attribute->kind) {
                case AttributeKind::Input:
                    maybe_node_id = FindLinkedNode(attribute->id);
                    if (maybe_node_id) {
                        json += std::string() + "\t\t\t\"" + attribute->label + "\": \"" + std::to_string(maybe_node_id.value()) + "\",\n";
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

std::string Graph::ToJson() {
    std::string to_return;
    std::set<int> visited_nodes;

    to_return += "{ \"context\": { \"globals\": { \"pi\": 3.1415927 } }, \"descriptors\": [\n";
    for (auto& id_node_pair : nodes) {
        if (id_node_pair.second.type == NodeType::Rendering)
            RecurseToJson(id_node_pair.second, visited_nodes, to_return);
    }
    to_return += "]}"; // closes descriptors array

    return to_return;
}

void Graph::AddNode(Node&& node) {
    // we need to keep our attribute-to-node map up-to-date
    for (std::shared_ptr<Attribute> attribute : node.attributes)
        attributes[attribute->id] = attribute;

    nodes.insert(std::make_pair(node.id, node));
}

}
