#pragma once

#include "rust/cxx.h"
#include "imgui.h"

namespace ImNodes {
    struct StyleShim;

    // forward declarations of init/shutdown, which was changed recently in imnodes
    void Initialize();
    void Shutdown();

    // forward declarations of the actual imnodes functions
    bool IsLinkCreated(int* started_at_attribute_id, int* ended_at_attribute_id, bool* created_from_snap);
    bool IsLinkHovered(int* id);
    bool IsNodeHovered(int* id);
    bool IsAnyAttributeActive(int* id);
    void SetNodeScreenSpacePos(int node_id, const ImVec2& screen_space_pos);
    ImVec2 GetNodeScreenSpacePos(const int node_id);
    int NumSelectedNodes();
    int NumSelectedLinks();
    void GetSelectedNodes(int* node_ids);
    void GetSelectedLinks(int* link_ids);

    // declaration of our shims
    bool IsLinkCreated(int& started_at_attribute_id, int& ended_at_attribute_id);
    bool IsAnyAttributeActive(int& attribute_id);
    bool IsLinkHovered(int& id);
    bool IsNodeHovered(int& id);
    void SetNodePosition(int node_id, std::array<float, 2> position);
    std::array<float, 2> GetEditorPanning();
    void SetEditorPanning(std::array<float, 2> panning);
    std::array<float, 2> GetNodePosition(int node_id);
    rust::Vec<int> GetSelectedNodes();
    void ApplyStyle(const StyleShim& style);
    void EnableCtrlScroll(bool enabled, const bool& key_modifier);
}
