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
    Other
};

class Node {
    public:
        Node(int id);
        ~Node();

        void Render();

        const int id;
        std::vector<std::shared_ptr<Attribute>> in_attributes;
        std::vector<std::shared_ptr<Attribute>> out_attributes;
        std::vector<std::shared_ptr<Attribute>> static_attributes;

        static Node TemplatedCurve(const std::function<int()> next_id);
        static Node TemplatedInterval(const std::function<int()> next_id);
        static Node TemplatedMatrix(const std::function<int()> next_id);

    private:
        NodeType type;
        std::string name;
};

}
