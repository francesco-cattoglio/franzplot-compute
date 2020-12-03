#pragma once

namespace ImGui {
    // most of the Imgui functions can be used from imgui-rs, but this particular one
    // is not, because it is in the imgui_internal.h header file.
    // Instead of including the entire header, we can just put a forward declaration here.
    void ClearActiveID();
}

