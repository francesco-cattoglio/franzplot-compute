#pragma once

#include "rust/cxx.h"

#include <memory>
#include <string>
#include <map>

#include <imgui.h>

namespace franzplot_gui {

struct SharedThing;
void init_imnodes();
void shutdown_imnodes();
void show_node_graph(SharedThing thing);
void do_something(SharedThing thing);

} // namespace franzplot_gui
