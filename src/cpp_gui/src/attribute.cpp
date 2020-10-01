#include "attribute.h"

#include <iostream>

namespace franzplot_gui {

Attribute::Attribute(int node_id) : id(new_id()), node_id(node_id) {
}

TextAttribute::TextAttribute(int node_id, const std::string& label)
    :
        Attribute(node_id),
        label(label),
        imgui_label("##" + std::to_string(this->id))
{
        this->buffer.fill('\0');
}

void TextAttribute::Render() {
    imnodes::BeginInputAttribute(this->id);
    ImGui::Text(this->label.c_str());
    ImGui::SameLine();
    ImGui::PushItemWidth(75);
    ImGui::InputText(this->imgui_label.c_str(), this->buffer.data(), this->buffer.size());
    ImGui::SameLine(); ImGui::Dummy(ImVec2(1, 1)); // this leaves just a tiny bit of empty space after the input text widget
    ImGui::PopItemWidth();
    imnodes::EndInputAttribute();
    return;
}

OutputAttribute::OutputAttribute(int node_id)
    :
        Attribute(node_id)
{
}

void OutputAttribute::Render() {
    imnodes::BeginOutputAttribute(this->id);
    auto node_dimensions = imnodes::GetNodeDimensions(this->node_id);
    const char label[] = "Output";
    ImGui::Indent(node_dimensions.x - 16 -ImGui::CalcTextSize(label).x);
    ImGui::Text(label);
    imnodes::EndInputAttribute();
    return;
}

}
