#include "attribute.h"

#include <iostream>

#include <imnodes.h>

namespace franzplot_gui {

Attribute::Attribute(int attribute_id, int node_id) : id(attribute_id), node_id(node_id) {
}

TextAttribute::TextAttribute(int attribute_id, int node_id, const std::string& label)
    :
        Attribute(attribute_id, node_id),
        label(label),
        imgui_label("##" + std::to_string(this->id))
{
        this->buffer.fill('\0');
}

void TextAttribute::Render() {
    imnodes::BeginStaticAttribute(this->id);
    ImGui::Text(this->label.c_str());
    ImGui::SameLine();
    ImGui::PushItemWidth(75);
    ImGui::InputText(this->imgui_label.c_str(), this->buffer.data(), this->buffer.size());
    ImGui::SameLine(); ImGui::Dummy(ImVec2(1, 1)); // this leaves just a tiny bit of empty space after the input text widget
    ImGui::PopItemWidth();
    imnodes::EndStaticAttribute();
    return;
}

OutputAttribute::OutputAttribute(int attribute_id, int node_id)
    :
        Attribute(attribute_id, node_id)
{
}

void OutputAttribute::Render() {
    imnodes::BeginOutputAttribute(this->id);
    auto node_dimensions = imnodes::GetNodeDimensions(this->node_id);
    const char label[] = "Output";
    ImGui::Indent(node_dimensions.x - 17 -ImGui::CalcTextSize(label).x);
    ImGui::Text(label);
    imnodes::EndInputAttribute();
    return;
}

IntervalAttribute::IntervalAttribute(int attribute_id, int node_id, const std::string& label)
    :
        Attribute(attribute_id, node_id),
        label(label)
{
}

void IntervalAttribute::Render() {
    imnodes::BeginInputAttribute(this->id);
    ImGui::Text(this->label.c_str());
    imnodes::EndInputAttribute();
    return;
}

}
