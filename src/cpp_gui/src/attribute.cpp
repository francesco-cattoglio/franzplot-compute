#include "attribute.h"

#include <cassert>
#include <iostream>

#include <imgui.h>

namespace franzplot_gui {

Attribute::Attribute(int attribute_id, int node_id, AttributeKind kind, imnodes::PinShape shape)
    : id(attribute_id), node_id(node_id), kind(kind), shape(shape)
{
}

bool Attribute::IsCompatible(Attribute& /*other*/) {
    return false;
}

void Attribute::Render() {
    switch (kind) {
        case AttributeKind::In:
            imnodes::BeginInputAttribute(this->id, shape);
            break;
        case AttributeKind::Out:
            imnodes::BeginOutputAttribute(this->id, shape);
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

Text::Text(int attribute_id, int node_id, const std::string& label, int text_field_size)
    :
        Attribute(attribute_id, node_id, AttributeKind::Static),
        label(label),
        imgui_label("##" + std::to_string(this->id)),
        text_field_size(text_field_size)
{
    buffer.fill('\0');
}

void Text::RenderContents() {
    ImGui::Text(this->label.c_str());
    ImGui::SameLine();
    ImGui::PushItemWidth(text_field_size);
    ImGui::InputText(this->imgui_label.c_str(), this->buffer.data(), this->buffer.size());
    ImGui::SameLine(); ImGui::Dummy(ImVec2(1, 1)); // this leaves just a tiny bit of empty space after the input text widget
    ImGui::PopItemWidth();
    return;
}

QuadText::QuadText(int attribute_id, int node_id, const std::string& label, int text_field_size)
    :
        Attribute(attribute_id, node_id, AttributeKind::Static),
        label(label),
        imgui_label_1("##" + std::to_string(this->id) + ":1"),
        imgui_label_2("##" + std::to_string(this->id) + ":2"),
        imgui_label_3("##" + std::to_string(this->id) + ":3"),
        imgui_label_4("##" + std::to_string(this->id) + ":4"),
        text_field_size(text_field_size)
{
    buffer_1.fill('\0');
    buffer_2.fill('\0');
    buffer_3.fill('\0');
    buffer_4.fill('\0');
}

void QuadText::RenderContents() {
    ImGui::Text(this->label.c_str());
    ImGui::SameLine();
    ImGui::PushItemWidth(text_field_size);
    ImGui::InputText(imgui_label_1.c_str(), buffer_1.data(), buffer_1.size());
    ImGui::SameLine();
    ImGui::InputText(imgui_label_2.c_str(), buffer_2.data(), buffer_2.size());
    ImGui::SameLine();
    ImGui::InputText(imgui_label_3.c_str(), buffer_3.data(), buffer_3.size());
    ImGui::SameLine();
    ImGui::InputText(imgui_label_4.c_str(), buffer_4.data(), buffer_4.size());
    ImGui::PopItemWidth();
    return;
}

#define MAGIC_OFFSET 17

InputInterval::InputInterval(int attribute_id, int node_id, const std::string& label)
    :
        Attribute(attribute_id, node_id, AttributeKind::In, imnodes::PinShape_QuadFilled),
        label(label)
{
}

bool InputInterval::IsCompatible(Attribute& rhs) {
    return typeid(rhs) == typeid(OutputInterval);
}

void InputInterval::RenderContents() {
    ImGui::Text(this->label.c_str());
    return;
}

OutputInterval::OutputInterval(int attribute_id, int node_id)
    :
        Attribute(attribute_id, node_id, AttributeKind::Out, imnodes::PinShape_QuadFilled)
{
}

void OutputInterval::RenderContents() {
    auto node_dimensions = imnodes::GetNodeDimensions(this->node_id);
    const char label[] = "Interval";
    ImGui::Indent(node_dimensions.x - MAGIC_OFFSET -ImGui::CalcTextSize(label).x);
    ImGui::Text(label);
    return;
}

InputGeometry::InputGeometry(int attribute_id, int node_id)
    :
        Attribute(attribute_id, node_id, AttributeKind::In, imnodes::PinShape_TriangleFilled)
{
}

bool InputGeometry::IsCompatible(Attribute& rhs) {
    return typeid(rhs) == typeid(OutputGeometry);
}

void InputGeometry::RenderContents() {
    const char label[] = "Geometry";
    ImGui::Text(label);
    return;
}

OutputGeometry::OutputGeometry(int attribute_id, int node_id)
    :
        Attribute(attribute_id, node_id, AttributeKind::Out, imnodes::PinShape_TriangleFilled)
{
}

void OutputGeometry::RenderContents() {
    auto node_dimensions = imnodes::GetNodeDimensions(this->node_id);
    const char label[] = "Output";
    ImGui::Indent(node_dimensions.x - MAGIC_OFFSET -ImGui::CalcTextSize(label).x);
    ImGui::Text(label);
    return;
}

InputMatrix::InputMatrix(int attribute_id, int node_id)
    :
        Attribute(attribute_id, node_id, AttributeKind::In)
{
}

bool InputMatrix::IsCompatible(Attribute& rhs) {
    return typeid(rhs) == typeid(OutputMatrix);
}

void InputMatrix::RenderContents() {
    const char label[] = "Matrix";
    ImGui::Text(label);
    return;
}

OutputMatrix::OutputMatrix(int attribute_id, int node_id)
    :
        Attribute(attribute_id, node_id, AttributeKind::Out)
{
}

void OutputMatrix::RenderContents() {
    auto node_dimensions = imnodes::GetNodeDimensions(this->node_id);
    const char label[] = "Output";
    ImGui::Indent(node_dimensions.x - MAGIC_OFFSET -ImGui::CalcTextSize(label).x);
    ImGui::Text(label);
    return;
}

}
