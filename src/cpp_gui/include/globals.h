#pragma once

namespace franzplot_gui {

class Graph;
struct Globals {
    bool show_another_window = false;
    ImVec4 clear_color = ImVec4(0.45f, 0.55f, 0.60f, 1.00f);
    Graph* graph;
};
static Globals globals;

}
