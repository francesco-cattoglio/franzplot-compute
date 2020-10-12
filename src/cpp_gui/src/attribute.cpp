#include "attribute.h"

#include <cassert>
#include <iostream>

#include <imgui.h>
#include <misc/cpp/imgui_stdlib.h>

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
{}

void Text::RenderContents() {
    ImGui::Text(this->label.c_str());
    ImGui::SameLine();
    ImGui::PushItemWidth(text_field_size);
    ImGui::InputText(imgui_label.c_str(), &buffer);
    ImGui::SameLine(); ImGui::Dummy(ImVec2(1, 1)); // this leaves just a tiny bit of empty space after the input text widget
    ImGui::PopItemWidth();
    return;
}

std::string Text::ContentsToJson() {
    std::string to_return;
    to_return += std::string() + "\"" + buffer + "\"";
    return to_return;
}

MatrixRow::MatrixRow(int attribute_id, int node_id, const std::string& label, int text_field_size)
    :
        StaticAttribute(attribute_id, node_id, label),
        imgui_label( {
                "##" + std::to_string(this->id) + ":1",
                "##" + std::to_string(this->id) + ":2",
                "##" + std::to_string(this->id) + ":3",
                "##" + std::to_string(this->id) + ":4"
                }),
        text_field_size(text_field_size)
{}

void MatrixRow::RenderContents() {
    // do not display the label for this attribute
    // ImGui::Text(this->label.c_str());
    // ImGui::SameLine();
    ImGui::PushItemWidth(text_field_size);
    assert(buffer.size() == imgui_label.size());
    for (size_t i = 0; i < buffer.size(); i++) {
        ImGui::InputText(imgui_label[i].c_str(), &buffer[i]);
        ImGui::SameLine();
    }
    ImGui::TextUnformatted(""); // burns the last SameLine command and adds just a tiny bit of space, which is nice
    ImGui::PopItemWidth();
    return;
}

std::string MatrixRow::ContentsToJson() {
    std::string to_return;
    to_return += std::string()
        + "[\"" + buffer[0]
        + "\", \"" + buffer[1]
        + "\", \"" + buffer[2]
        + "\", \"" + buffer[3]
        + "\"]";
    return to_return;
}

IntSlider::IntSlider(int attribute_id, int node_id, const std::string& label, int min, int max)
    :
        StaticAttribute(attribute_id, node_id, label),
        min(min),
        max(max),
        value(min)
{}

std::string IntSlider::ContentsToJson() {
    return std::to_string(value);
}

void IntSlider::RenderContents() {
    ImGui::Text(label.c_str());
    ImGui::SameLine();
    ImGui::PushItemWidth(45);
    ImGui::SliderInt("", &value, min, max);
    ImGui::PopItemWidth();
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
