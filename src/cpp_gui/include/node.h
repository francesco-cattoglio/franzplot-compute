#pragma once

#include <string>
#include <vector>
#include <memory>

enum class NodeType {
    Interval,
    Curve,
    Surface,
    Transform,
    Matrix
};

#include "attribute.h"
class Node {
    public:
        void Render();

    private:
        NodeType type;
        int id;
        std::string name;
        std::vector<std::shared_ptr<Attribute>> in_attributes;
        std::vector<std::shared_ptr<Attribute>> out_attributes;
        std::vector<std::shared_ptr<Attribute>> static_attributes;
};
