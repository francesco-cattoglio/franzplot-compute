#pragma once

#include "rust/cxx.h"

#include <memory>
#include <string>
#include <map>

#include <imgui.h>

namespace franzplot_gui {

struct OtherShared;
struct RustProxy;
class GuiInstance {
    public:
        GuiInstance() = delete;
        GuiInstance(GuiInstance&) = delete;
        GuiInstance(rust::Box<RustProxy>& boxed_proxy) : boxed_proxy(std::move(boxed_proxy)) { test_boxed_proxy(); }

        void test_boxed_proxy();
    private:
        rust::Box<RustProxy> boxed_proxy;
};
struct SharedThing;
void init_imnodes();
std::unique_ptr<GuiInstance> init_2(rust::Box<RustProxy> other_shared);
void shutdown_imnodes();
void show_node_graph(SharedThing thing);
void do_something(SharedThing thing);

} // namespace franzplot_gui
