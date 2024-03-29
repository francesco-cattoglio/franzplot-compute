#include "imnodes_shims.h"
#include "imgui.h"
#include "franzplot-compute/src/cpp_gui/mod.rs.h"

namespace ImNodes {

    void Initialize() {
        ImNodes::CreateContext();
        return;
    }

    void Shutdown() {
        ImNodes::DestroyContext(nullptr);
        return;
    }

    bool IsLinkCreated(int& started_at_attribute_id, int& ended_at_attribute_id) {
        return IsLinkCreated(&started_at_attribute_id, &ended_at_attribute_id, nullptr);
    }

    bool IsLinkHovered(int& id) {
        return IsLinkHovered(&id);
    }

    bool IsNodeHovered(int& id) {
        return IsNodeHovered(&id);
    }

    bool IsAnyAttributeActive(int& id) {
        return IsAnyAttributeActive(&id);
    }

    std::array<float, 2> GetNodePosition(const int node_id) {
        ImVec2 pos = GetNodeGridSpacePos(node_id);
        return {pos.x, pos.y};
    }

    void SetNodePosition(int node_id, std::array<float, 2> position) {
        ImVec2 pos = ImVec2{position[0], position[1]};
        return SetNodeGridSpacePos(node_id, pos);
    }

    std::array<float, 2> GetEditorPanning() {
        ImVec2 pan = EditorContextGetPanning();
        return {pan.x, pan.y};
    }

    void SetEditorPanning(std::array<float, 2> panning) {
        ImVec2 pan = ImVec2{panning[0], panning[1]};
        return EditorContextResetPanning(pan);
    }

    rust::Vec<int> GetSelectedNodes() {
        rust::Vec<int> to_return;
        const int num_selected_nodes = NumSelectedNodes();
        if (num_selected_nodes > 0)
        {
            std::vector<int> selected_nodes;
            selected_nodes.resize(num_selected_nodes);
            ImNodes::GetSelectedNodes(selected_nodes.data());
            // copy the Cpp array over the rust one
            to_return.reserve(num_selected_nodes);
            for (int node_id : selected_nodes)
                to_return.push_back(node_id);
        }

        return to_return;
    }

    void ApplyStyle(const StyleShim& new_style) {
        auto& style = ImNodes::GetStyle();
        style.GridSpacing = new_style.grid_spacing;
        style.NodePadding = ImVec2{new_style.node_padding_horizontal, new_style.node_padding_vertical};
        style.LinkThickness = new_style.link_thickness;

        style.PinCircleRadius = new_style.pin_circle_radius;
        style.PinQuadSideLength = new_style.pin_quad_side_length;
        style.PinTriangleSideLength = new_style.pin_triangle_side_length;
        style.PinLineThickness = new_style.pin_line_thickness;
        style.PinHoverRadius = new_style.pin_hover_radius;
    }

    void EnableCtrlScroll(bool enabled, const bool& key_modifier) {
        if (enabled) {
            ImNodes::GetIO().EmulateThreeButtonMouse.Modifier = &key_modifier;
        } else {
            ImNodes::GetIO().EmulateThreeButtonMouse.Modifier = nullptr;
        }
    }
}
