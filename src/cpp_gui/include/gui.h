#pragma once

#include "rust/cxx.h"

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

    private:
        Graph graph;
        rust::Box<RustEventProxy> boxed_proxy;
};

std::unique_ptr<Gui> create_gui_instance(rust::Box<RustEventProxy> boxed_proxy);

}
