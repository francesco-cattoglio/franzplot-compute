#pragma once

#include "rust/cxx.h"

#include <memory>
#include <string>
#include <map>

#include <imgui.h>

namespace franzplot_gui {

struct RustEventProxy;
class GuiInstance {
    public:
        GuiInstance() = delete;
        GuiInstance(GuiInstance&) = delete;
        GuiInstance(rust::Box<RustEventProxy>& boxed_proxy) : boxed_proxy(std::move(boxed_proxy)) { test_boxed_proxy( ); }

        void test_boxed_proxy();
    private:
        rust::Box<RustEventProxy> boxed_proxy;
};
void init_imnodes();
std::unique_ptr<GuiInstance> init_2(rust::Box<RustEventProxy> other_shared);
void shutdown_imnodes();
//void show_node_graph(SharedThing thing);
//void do_something(SharedThing thing);

} // namespace franzplot_gui
