#include "attribute.h"

#include <cassert>
#include <iostream>

#include <imgui.h>

namespace franzplot_gui {

Attribute::Attribute(int attribute_id, int node_id, const std::string& label, AttributeKind kind)
    : id(attribute_id), node_id(node_id), label(label), kind(kind)
{}

InputAttribute::InputAttribute(int attribute_id, int node_id, const std::string& label, PinKind pin_kind)
    :
        Attribute(attribute_id, node_id, label, AttributeKind::Input),
        pin_kind(pin_kind)
{}

// middle derived classes
void InputAttribute::Render() {
    imnodes::BeginInputAttribute(id, ToShape(pin_kind));
    this->RenderContents(); // call actual rendering implemented in child class
    imnodes::EndInputAttribute();
}

OutputAttribute::OutputAttribute(int attribute_id, int node_id, const std::string& label, PinKind pin_kind)
    :
        Attribute(attribute_id, node_id, label, AttributeKind::Output),
        pin_kind(pin_kind)
{}

void OutputAttribute::Render() {
    imnodes::BeginOutputAttribute(id, ToShape(pin_kind));
    this->RenderContents(); // call actual rendering implemented in child class
    imnodes::EndOutputAttribute();
}

StaticAttribute::StaticAttribute(int attribute_id, int node_id, const std::string& label)
    :
        Attribute(attribute_id, node_id, label, AttributeKind::Static)
{}

void StaticAttribute::Render() {
    imnodes::BeginStaticAttribute(id);
    this->RenderContents(); // call actual rendering implemented in child class
    imnodes::EndStaticAttribute();
}

// final derived classes
SimpleInput::SimpleInput(int attribute_id, int node_id, const std::string& label, PinKind pin_kind)
    :
        InputAttribute(attribute_id, node_id, label, pin_kind)
{}

void SimpleInput::RenderContents() {
    ImGui::Text(label.c_str());
    return;
}

SimpleOutput::SimpleOutput(int attribute_id, int node_id, const std::string& label, PinKind pin_kind)
    :
        OutputAttribute(attribute_id, node_id, label, pin_kind)
{}

#define MAGIC_OFFSET 17
void SimpleOutput::RenderContents() {
    auto node_dimensions = imnodes::GetNodeDimensions(this->node_id);
    ImGui::Indent(node_dimensions.x - MAGIC_OFFSET - ImGui::CalcTextSize(label.c_str()).x);
    ImGui::Text(label.c_str());
    return;
}

Text::Text(int attribute_id, int node_id, const std::string& label, int text_field_size)
    :
        StaticAttribute(attribute_id, node_id, label),
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

MatrixRow::MatrixRow(int attribute_id, int node_id, const std::string& label, int text_field_size)
    :
        StaticAttribute(attribute_id, node_id, label),
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

void MatrixRow::RenderContents() {
    // do not display the label for this attribute
    // ImGui::Text(this->label.c_str());
    // ImGui::SameLine();
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

// helper functions
imnodes::PinShape ToShape(PinKind kind) {
    switch (kind) {
        case PinKind::Geometry:
            return imnodes::PinShape_TriangleFilled;

        case PinKind::Interval:
            return imnodes::PinShape_CircleFilled;

        case PinKind::Matrix:
            return imnodes::PinShape_QuadFilled;
    }

    assert(0 && "unreachable code reached");
}

bool IsCompatible(InputAttribute& input, OutputAttribute& output) {
    return input.pin_kind == output.pin_kind;
}

}
