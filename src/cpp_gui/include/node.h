#pragma once

#include <string>
#include <vector>
#include <memory>

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
        Node();
        ~Node();

        void Render();

        const int id;
        std::vector<std::shared_ptr<Attribute>> in_attributes;
        std::vector<std::shared_ptr<Attribute>> out_attributes;
        std::vector<std::shared_ptr<Attribute>> static_attributes;

        static Node TemplatedCurve();

    private:
        NodeType type;
        std::string name;
};

}
