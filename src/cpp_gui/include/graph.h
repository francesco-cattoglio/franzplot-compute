#pragma once

#include "node.h"

namespace franzplot_gui {

class Graph {
    public:

        void Test();
        void Render();
    private:
        int NextId();
        int next_id = 0;
        std::map<int, Node> nodes;
        std::map<int, int> attr_node_map;
        std::map<int, std::pair<int, int>> links;
};
//void add_node(Node&& node) {
//    // we need to keep our attribute-to-node map up-to-date
//    for (auto& attribute : node.in_attributes)
//        globals.attr_node_map[attribute->id] = node.id;
//
//    for (auto& attribute : node.out_attributes)
//        globals.attr_node_map[attribute->id] = node.id;
//
//    for (auto& attribute : node.static_attributes)
//        globals.attr_node_map[attribute->id] = node.id;
//
//    globals.nodes.insert(std::make_pair(node.id, node));
//}


}
