#pragma once

#include "rust/cxx.h"

struct ImVec2;

namespace imnodes {

    // forward declarations of the actual imnodes functions
    bool IsLinkCreated(int* started_at_attribute_id, int* ended_at_attribute_id, bool* created_from_snap);
    bool IsLinkHovered(int* id);
    bool IsNodeHovered(int* id);
    void SetNodeScreenSpacePos(int node_id, const ImVec2& screen_space_pos);
    ImVec2 GetNodeScreenSpacePos(const int node_id);
    int NumSelectedNodes();
    int NumSelectedLinks();
    void GetSelectedNodes(int* node_ids);
    void GetSelectedLinks(int* link_ids);

    // declaration of our shims
    bool IsLinkCreated(int& started_at_attribute_id, int& ended_at_attribute_id);
    bool IsLinkHovered(int& id);
    bool IsNodeHovered(int& id);
    void SetNodeScreenSpacePos(int node_id, float x, float y);
    void GetNodeScreenSpacePos(int node_id, float& x, float& y);
    rust::Vec<int> GetSelectedNodes();
}
