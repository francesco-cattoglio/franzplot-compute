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

    void SetNodeScreenSpacePos(int node_id, float x, float y) {
        ImVec2 pos = ImVec2{x, y};
        return SetNodeScreenSpacePos(node_id, pos);
    }
}
