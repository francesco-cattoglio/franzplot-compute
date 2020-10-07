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

    for (auto& attribute: attributes)
        attribute->Render();

    imnodes::EndNode();
}

Node Node::PrefabCurve(const std::function<int()> next_id) {
    Node to_return = Node(next_id());
    to_return.type = NodeType::Curve;
    to_return.name = "curve node";
    to_return.attributes = {
        std::make_shared<SimpleInput>(next_id(), to_return.id, PinKind::Interval, "interval"),
        std::make_shared<SimpleOutput>(next_id(), to_return.id, PinKind::Geometry, "geometry"),
        std::make_shared<Text>(next_id(), to_return.id, "fx", 75),
        std::make_shared<Text>(next_id(), to_return.id, "fy", 75),
        std::make_shared<Text>(next_id(), to_return.id, "fz", 75),
    };

    return to_return;
}

Node Node::PrefabInterval(const std::function<int()> next_id) {
    Node to_return = Node(next_id());
    to_return.type = NodeType::Interval;
    to_return.name = "Interval";
    to_return.attributes = {
        std::make_shared<SimpleOutput>(next_id(), to_return.id, PinKind::Interval, "interval"),
        std::make_shared<Text>(next_id(), to_return.id, "name", 35),
        std::make_shared<Text>(next_id(), to_return.id, "begin", 35),
        std::make_shared<Text>(next_id(), to_return.id, "end", 35),
    };

    return to_return;
}

Node Node::PrefabRendering(const std::function<int()> next_id) {
    Node to_return = Node(next_id());
    to_return.type = NodeType::Rendering;
    to_return.name = "Rendering";
    to_return.attributes = {
        std::make_shared<SimpleInput>(next_id(), to_return.id, PinKind::Geometry, "geometry")
    };

    return to_return;
}

Node Node::PrefabMatrix(const std::function<int()> next_id) {
    Node to_return = Node(next_id());
    to_return.type = NodeType::Matrix;
    to_return.name = "Matrix";
    to_return.attributes = {
        std::make_shared<SimpleOutput>(next_id(), to_return.id, PinKind::Matrix, "matrix"),
        std::make_shared<SimpleInput>(next_id(), to_return.id, PinKind::Interval, "interval"),
        std::make_shared<QuadText>(next_id(), to_return.id, ""),
        std::make_shared<QuadText>(next_id(), to_return.id, ""),
        std::make_shared<QuadText>(next_id(), to_return.id, ""),
    };

    return to_return;
}

} // namespace
