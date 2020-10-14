#pragma once

#include "rust/cxx.h"

#include <array>

#include "graph.h"

namespace franzplot_gui {

struct RustEventProxy;

class Gui {
    public:
        Gui() = delete;
        Gui(Gui&) = delete;
        Gui(rust::Box<RustEventProxy>& boxed_proxy);

        void test_boxed_proxy();
        void Render();
        void MarkError(std::int32_t id, rust::Str message);

    private:
        std::array<char, 32> new_globals_name;
        std::vector<std::array<char, 32>> globals_names;
        std::vector<float> globals_values;

        Graph graph;
        rust::Box<RustEventProxy> boxed_proxy;
};

std::unique_ptr<Gui> create_gui_instance(rust::Box<RustEventProxy> boxed_proxy);

}
