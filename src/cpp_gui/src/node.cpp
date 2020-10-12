#include "node.h"

#include <imnodes.h>

#include "library.h"

namespace franzplot_gui {

Node::Node(int id, NodeType type)
    :
        id(id), type(type)
{}

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
    Node to_return = Node(next_id(), NodeType::Curve);
    to_return.name = "Curve node";
    to_return.attributes = {
        std::make_shared<SimpleInput>(next_id(), to_return.id, "interval", PinKind::Interval),
        std::make_shared<SimpleOutput>(next_id(), to_return.id, "geometry", PinKind::Geometry),
        std::make_shared<Text>(next_id(), to_return.id, "fx", 75),
        std::make_shared<Text>(next_id(), to_return.id, "fy", 75),
        std::make_shared<Text>(next_id(), to_return.id, "fz", 75),
    };

    return to_return;
}

Node Node::PrefabSurface(const std::function<int()> next_id) {
    Node to_return = Node(next_id(), NodeType::Surface);
    to_return.name = "Surface node";
    to_return.attributes = {
        std::make_shared<SimpleOutput>(next_id(), to_return.id, "geometry", PinKind::Geometry),
        std::make_shared<SimpleInput>(next_id(), to_return.id, "interval_first", PinKind::Interval),
        std::make_shared<SimpleInput>(next_id(), to_return.id, "interval_second", PinKind::Interval),
        std::make_shared<Text>(next_id(), to_return.id, "fx", 75),
        std::make_shared<Text>(next_id(), to_return.id, "fy", 75),
        std::make_shared<Text>(next_id(), to_return.id, "fz", 75),
    };

    return to_return;
}

Node Node::PrefabInterval(const std::function<int()> next_id) {
    Node to_return = Node(next_id(), NodeType::Interval);
    to_return.name = "Interval";
    auto begin = std::make_shared<Text>(next_id(), to_return.id, "begin", 35);
    auto end = std::make_shared<Text>(next_id(), to_return.id, "end", 35);
    begin->buffer = "0.0";
    end->buffer = "1.0";
    to_return.attributes = {
        std::make_shared<SimpleOutput>(next_id(), to_return.id, "interval", PinKind::Interval),
        std::make_shared<Text>(next_id(), to_return.id, "name", 35),
        begin,
        end,
        std::make_shared<IntSlider>(next_id(), to_return.id, "quality", 1, 16),
    };

    return to_return;
}

Node Node::PrefabRendering(const std::function<int()> next_id) {
    Node to_return = Node(next_id(), NodeType::Rendering);
    to_return.name = "Rendering";
    to_return.attributes = {
        std::make_shared<SimpleInput>(next_id(), to_return.id, "surface", PinKind::Geometry)
    };

    return to_return;
}

Node Node::PrefabTransform(const std::function<int()> next_id) {
    Node to_return = Node(next_id(), NodeType::Transform);
    to_return.name = "Transform";
    to_return.attributes = {
        std::make_shared<SimpleOutput>(next_id(), to_return.id, "geometry", PinKind::Geometry),
        std::make_shared<SimpleInput>(next_id(), to_return.id, "geometry", PinKind::Geometry),
        std::make_shared<SimpleInput>(next_id(), to_return.id, "matrix", PinKind::Matrix)
    };

    return to_return;
}

Node Node::PrefabMatrix(const std::function<int()> next_id) {
    Node to_return = Node(next_id(), NodeType::Matrix);
    to_return.name = "Matrix";
    auto row_1 = std::make_shared<MatrixRow>(next_id(), to_return.id, "row_1");
    auto row_2 = std::make_shared<MatrixRow>(next_id(), to_return.id, "row_2");
    auto row_3 = std::make_shared<MatrixRow>(next_id(), to_return.id, "row_3");
    row_1->buffer = { "1.0", "0.0", "0.0", "0.0" };
    row_2->buffer = { "0.0", "1.0", "0.0", "0.0" };
    row_3->buffer = { "0.0", "0.0", "1.0", "0.0" };
    to_return.attributes = {
        std::make_shared<SimpleOutput>(next_id(), to_return.id, "matrix", PinKind::Matrix),
        std::make_shared<SimpleInput>(next_id(), to_return.id, "interval", PinKind::Interval),
        row_1,
        row_2,
        row_3,
    };

    return to_return;
}

// helper function
std::string ToString(NodeType type) {
    switch (type) {
        case NodeType::Curve:
            return "Curve";

        case NodeType::Interval:
            return "Interval";

        case NodeType::Surface:
            return "Surface";

        case NodeType::Matrix:
            return "Matrix";

        case NodeType::Transform:
            return "Transform";

        case NodeType::Rendering:
            return "SurfaceRenderer";

        case NodeType::Other:
            assert(0 && "unimplemented - case not handled");
            return "Other";
    }

    assert(0 && "unreachable code reached");
}
} // namespace
