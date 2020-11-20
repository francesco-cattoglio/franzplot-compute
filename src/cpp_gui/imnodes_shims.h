#pragma once

struct ImVec2;

namespace imnodes {

    // forward declarations of the actual imnodes functions
    bool IsLinkCreated(int* started_at_attribute_id, int* ended_at_attribute_id, bool* created_from_snap);
    bool IsLinkHovered(int* id);
    bool IsNodeHovered(int* id);
    void SetNodeScreenSpacePos(int node_id, const ImVec2& screen_space_pos);

    // declaration of our shims
    bool IsLinkCreated(int& started_at_attribute_id, int& ended_at_attribute_id);
    bool IsLinkHovered(int& id);
    bool IsNodeHovered(int& id);
    void SetNodeScreenSpacePos(int node_id, float x, float y);

}
