#pragma once

#include "rust/cxx.h"

#include <array>

#include "graph.h"

typedef std::array<char, 32> VarName;

namespace franzplot_gui {

struct RustEventProxy;
struct State;
struct GuiRequests;

class Gui {
    public:
        Gui() = delete;
        Gui(Gui&) = delete;
        Gui(rust::Box<RustEventProxy>& boxed_proxy);

        GuiRequests Render(State& rust_state, std::uint32_t x_size, std::uint32_t y_size);
        void UpdateSceneTexture(std::size_t scene_texture_id);

    private:
        bool ValidVarName(const VarName& name);
        void RenderGraphPage(State& rust_state);
        GuiRequests RenderScenePage(State& rust_state);
        void RenderSettingsPage(State& rust_state);
        void ClearAllMarks();
        void MarkClean(int id);
        void MarkError(std::int32_t id, const rust::String& message);
        void MarkWarning(std::int32_t id, const rust::String& message);

        VarName new_var_name;
        std::vector<VarName> globals_names;
        std::vector<float> globals_values;

        Graph graph;
        std::size_t scene_texture_id;
        rust::Box<RustEventProxy> boxed_proxy;
};

std::unique_ptr<Gui> create_gui_instance(rust::Box<RustEventProxy> boxed_proxy);

}
