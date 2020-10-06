#include "attribute.h"

#include <cassert>
#include <iostream>

#include <imnodes.h>

namespace franzplot_gui {

Attribute::Attribute(int attribute_id, int node_id, AttributeKind kind) : id(attribute_id), node_id(node_id), kind(kind) {
}

void Attribute::Render() {
    switch (kind) {
        case AttributeKind::In:
            imnodes::BeginInputAttribute(this->id);
            break;
        case AttributeKind::Out:
            imnodes::BeginOutputAttribute(this->id);
            break;
        case AttributeKind::Static:
            imnodes::BeginStaticAttribute(this->id);
            break;
        case AttributeKind::Unknown:
            assert(0);
            break;
    }
    this->RenderContents();
    switch (kind) {
        case AttributeKind::In:
            imnodes::EndInputAttribute();
            break;
        case AttributeKind::Out:
            imnodes::EndOutputAttribute();
            break;
        case AttributeKind::Static:
            imnodes::EndStaticAttribute();
            break;
        case AttributeKind::Unknown:
            assert(0);
            break;
    }
}

TextAttribute::TextAttribute(int attribute_id, int node_id, const std::string& label)
    :
        Attribute(attribute_id, node_id, AttributeKind::Static),
        label(label),
        imgui_label("##" + std::to_string(this->id))
{
    buffer.fill('\0');
}

void TextAttribute::RenderContents() {
    ImGui::Text(this->label.c_str());
    ImGui::SameLine();
    ImGui::PushItemWidth(75);
    ImGui::InputText(this->imgui_label.c_str(), this->buffer.data(), this->buffer.size());
    ImGui::SameLine(); ImGui::Dummy(ImVec2(1, 1)); // this leaves just a tiny bit of empty space after the input text widget
    ImGui::PopItemWidth();
    return;
}

OutputAttribute::OutputAttribute(int attribute_id, int node_id)
    :
        Attribute(attribute_id, node_id, AttributeKind::Out)
{
}

void OutputAttribute::RenderContents() {
    auto node_dimensions = imnodes::GetNodeDimensions(this->node_id);
    const char label[] = "Output";
    ImGui::Indent(node_dimensions.x - 17 -ImGui::CalcTextSize(label).x);
    ImGui::Text(label);
    return;
}

IntervalAttribute::IntervalAttribute(int attribute_id, int node_id, const std::string& label)
    :
        Attribute(attribute_id, node_id, AttributeKind::In),
        label(label)
{
}

void IntervalAttribute::RenderContents() {
    ImGui::Text(this->label.c_str());
    return;
}

}
