#pragma once

#include <string>
#include <vector>
#include <memory>
#include <functional>

#include "attribute.h"

namespace franzplot_gui {

enum class NodeType {
    Interval,
    Curve,
    Surface,
    Transform,
    Matrix,
    Rendering,
    Other
};

std::string ToString(NodeType type);

class Node {
    public:
        Node(int id, NodeType type);

        void Render();

        const int id;
        const NodeType type;
        std::vector<std::shared_ptr<Attribute>> attributes;

        static Node PrefabCurve(const std::function<int()> next_id);
        static Node PrefabSurface(const std::function<int()> next_id);
        static Node PrefabInterval(const std::function<int()> next_id);
        static Node PrefabMatrix(const std::function<int()> next_id);
        static Node PrefabRendering(const std::function<int()> next_id);
        static Node PrefabTransform(const std::function<int()> next_id);

        std::string name;
    private:
};

}
