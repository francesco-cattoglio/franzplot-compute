#include "imnodes_shims.h"

struct ImVec2 {
    float x, y;
};

namespace imnodes {

    bool IsLinkCreated(int& started_at_attribute_id, int& ended_at_attribute_id) {
        return IsLinkCreated(&started_at_attribute_id, &ended_at_attribute_id, nullptr);
    }

    bool IsLinkHovered(int& id) {
        return IsLinkHovered(&id);
    }

    bool IsNodeHovered(int& id) {
        return IsNodeHovered(&id);
    }

    void GetNodeScreenSpacePos(const int node_id, float& x, float& y) {
        ImVec2 pos = GetNodeScreenSpacePos(node_id);
        x = pos.x;
        y = pos.y;
        return;
    }

    void SetNodeScreenSpacePos(int node_id, float x, float y) {
        ImVec2 pos = ImVec2{x, y};
        return SetNodeScreenSpacePos(node_id, pos);
    }

    rust::Vec<int> GetSelectedNodes() {
        rust::Vec<int> to_return;
        const int num_selected_nodes = NumSelectedNodes();
        if (num_selected_nodes > 0)
        {
            std::vector<int> selected_nodes;
            selected_nodes.resize(num_selected_nodes);
            imnodes::GetSelectedNodes(selected_nodes.data());
            // copy the Cpp array over the rust one
            to_return.reserve(num_selected_nodes);
            for (int node_id : selected_nodes)
                to_return.push_back(node_id);
        }

        return to_return;
    }

}
