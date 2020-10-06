#include "node.h"

#include <imnodes.h>

#include "library.h"

namespace franzplot_gui {

Node::Node(int id) : id(id) {}

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

Node Node::TemplatedCurve(const std::function<int()> next_id) {
    Node to_return = Node(next_id());
    to_return.type = NodeType::Curve;
    to_return.name = "curve node";
    to_return.in_attributes.push_back(std::make_shared<IntervalAttribute>(next_id(), to_return.id, "interval"));
    to_return.static_attributes.push_back(std::make_shared<TextAttribute>(next_id(), to_return.id, "fx", 75));
    to_return.static_attributes.push_back(std::make_shared<TextAttribute>(next_id(), to_return.id, "fy", 75));
    to_return.static_attributes.push_back(std::make_shared<TextAttribute>(next_id(), to_return.id, "fz", 75));
    to_return.out_attributes.push_back(std::make_shared<OutputAttribute>(next_id(), to_return.id));

    return to_return;
}

Node Node::TemplatedInterval(const std::function<int()> next_id) {
    Node to_return = Node(next_id());
    to_return.type = NodeType::Curve;
    to_return.name = "Interval";
    to_return.static_attributes.push_back(std::make_shared<TextAttribute>(next_id(), to_return.id, "name", 35));
    to_return.static_attributes.push_back(std::make_shared<TextAttribute>(next_id(), to_return.id, "begin", 35));
    to_return.static_attributes.push_back(std::make_shared<TextAttribute>(next_id(), to_return.id, "end", 35));
    to_return.out_attributes.push_back(std::make_shared<OutputInterval>(next_id(), to_return.id));

    return to_return;
}

Node Node::TemplatedMatrix(const std::function<int()> next_id) {
    Node to_return = Node(next_id());
    to_return.type = NodeType::Curve;
    to_return.name = "Matrix";
    to_return.in_attributes.push_back(std::make_shared<IntervalAttribute>(next_id(), to_return.id, "interval"));
    to_return.static_attributes.push_back(std::make_shared<QuadTextAttribute>(next_id(), to_return.id, ""));
    to_return.static_attributes.push_back(std::make_shared<QuadTextAttribute>(next_id(), to_return.id, ""));
    to_return.static_attributes.push_back(std::make_shared<QuadTextAttribute>(next_id(), to_return.id, ""));
    to_return.out_attributes.push_back(std::make_shared<OutputAttribute>(next_id(), to_return.id));

    return to_return;
}

} // namespace
