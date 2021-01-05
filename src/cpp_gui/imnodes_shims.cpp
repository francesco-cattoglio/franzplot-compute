#include "imnodes_shims.h"
#include "franzplot-compute/src/cpp_gui/mod.rs.h"

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

    bool IsAnyAttributeActive(int& id) {
        return IsAnyAttributeActive(&id);
    }

    std::array<float, 2> GetNodePosition(const int node_id) {
        ImVec2 pos = GetNodeScreenSpacePos(node_id);
        return {pos.x, pos.y};
    }

    void SetNodePosition(int node_id, std::array<float, 2> position) {
        ImVec2 pos = ImVec2{position[0], position[1]};
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

    void ApplyStyle(const StyleShim& new_style) {
        auto& style = imnodes::GetStyle();
        style.grid_spacing = new_style.grid_spacing;
        style.node_padding_horizontal = new_style.node_padding_horizontal;
        style.node_padding_vertical = new_style.node_padding_vertical;
        style.pin_circle_radius = new_style.pin_circle_radius;
        style.pin_quad_side_length = new_style.pin_quad_side_length;
        style.pin_triangle_side_length = new_style.pin_triangle_side_length;
        style.pin_line_thickness = new_style.pin_line_thickness;
        style.pin_hover_radius = new_style.pin_hover_radius;
    }

}
