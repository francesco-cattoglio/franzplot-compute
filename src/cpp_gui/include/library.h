#pragma once
#include "rust/cxx.h"
#include <memory>
#include <string>
#include <map>

#include <imgui.h>

namespace franzplot_gui {

class Node;
class ThingC {
public:
  ThingC(std::string appname);
  ~ThingC();

  std::string appname;
};

struct SharedThing;

void add_node(Node&&);
std::unique_ptr<ThingC> make_demo(rust::Str appname);
const std::string &get_name(const ThingC &thing);
void do_thing(SharedThing state);
void init_imnodes();
void shutdown_imnodes();
void show_node_graph();

} // namespace franzplot_gui
