#include "node.h"

#include <imnodes.h>

#include "library.h"

namespace franzplot_gui {

Node::Node() : id(new_id()) {}

Node::~Node() {}

void Node::Render() {
    imnodes::BeginNode(this->id);

    imnodes::BeginNodeTitleBar();
    ImGui::TextUnformatted(this->name.c_str());
    imnodes::EndNodeTitleBar();

    for (auto& attribute: this->out_attributes) {
        attribute->Render();
    }
    for (auto& attribute: this->in_attributes) {
        attribute->Render();
    }
    for (auto& attribute: this->static_attributes) {
        attribute->Render();
    }

    imnodes::EndNode();
}

Node Node::TemplatedCurve() {
    Node to_return = Node();
    to_return.type = NodeType::Curve;
    to_return.name = "curve node";
    to_return.in_attributes.push_back(std::make_shared<TextAttribute>(to_return.id, "fx"));
    to_return.in_attributes.push_back(std::make_shared<TextAttribute>(to_return.id, "fy"));
    to_return.in_attributes.push_back(std::make_shared<TextAttribute>(to_return.id, "fz"));
    to_return.out_attributes.push_back(std::make_shared<OutputAttribute>(to_return.id));

    return to_return;
}

} // namespace
