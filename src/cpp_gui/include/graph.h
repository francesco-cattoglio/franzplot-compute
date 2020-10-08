#pragma once

#include <set>
#include <map>

#include "node.h"

#include "rust/cxx.h"

namespace franzplot_gui {

struct SharedThing;
class Graph {
    public:

        void Test();
        void Render(const SharedThing& state);
        std::string ToJson();
    private:
        std::optional<int> FindLinkedNode(int input_attribute_id);
        void RecurseToJson(const Node& node, std::set<int>& visited_nodes, std::string& json);
        void AddNode(Node&& node);
        int NextId();
        int next_id = 0;
        std::map<int, Node> nodes;
        std::map<int, std::shared_ptr<Attribute>> attributes;
        // We need a data structure to store the links between our nodes,
        // but for our specific use case, one input can have one and only one link attached to it.
        // This means that we can just use the same ID for both a link and the attribute!
        // By doing this management of the links becomes way easier, because a single map stores
        // both the link and the two attribute IDs.
        std::map<int, int> input_to_output_links;
};

}
