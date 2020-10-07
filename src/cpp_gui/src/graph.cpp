#include "graph.h"

#include <iostream>
#include <imgui.h>
#include <imnodes.h>

namespace franzplot_gui {

void Graph::Render() {
    bool test_button = ImGui::Button("gotest!");
    if (test_button)
        std::cout << "testing took place: " << this->ToJson() << std::endl;

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
    AddNode(Node::PrefabCurve(std::bind(&Graph::NextId, this)));
    AddNode(Node::PrefabMatrix(std::bind(&Graph::NextId, this)));
    AddNode(Node::PrefabRendering(std::bind(&Graph::NextId, this)));
    AddNode(Node::PrefabTransform(std::bind(&Graph::NextId, this)));
}

void Graph::RecurseToJson(const Node& node, std::set<int>& visited_nodes, std::string& json) {
    // if the node has ANY input, recurse
    for (auto& attribute_ptr : node.attributes) {
        if (attribute_ptr->kind == AttributeKind::Input) {
            auto find_results = input_to_output_links.find(attribute_ptr->id);
            if (find_results != input_to_output_links.end()) {
                int linked_attribute_id = find_results->second;
                int linked_node_id = attributes[linked_attribute_id]->node_id;
                auto& node = nodes.at(linked_node_id);
                RecurseToJson(node, visited_nodes, json);
            } else {
                std::cout << "warning: unconnected node" << std::endl;
            }
        }
    }

    // after we are done with the recursion, store myself in the json string, provided this has not been done before
    if (visited_nodes.count(node.id) == 0) {
        json += ", " + node.name;
        // mark as visited!
        visited_nodes.insert(node.id);
    }
}

std::string Graph::ToJson() {
    std::string to_return;
    std::set<int> visited_nodes;
    to_return += "{ descriptors: [\n";
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
