#pragma once

namespace franzplot_gui {

struct Globals {
    int next_id = 0;
    bool show_another_window = false;
    ImVec4 clear_color = ImVec4(0.45f, 0.55f, 0.60f, 1.00f);
    std::map<int, Node> nodes;
    std::map<int, int> attr_node_map;
    std::map<int, std::pair<int, int>> links;
};
static Globals globals;

}
