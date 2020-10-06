#pragma once

#include <array>
#include <string>

#include <imnodes.h>

namespace franzplot_gui {

enum class AttributeKind {
    In,
    Out,
    Static,
    Unknown
};

class Attribute {
    public:
        Attribute(int attribute_id, int node_id, AttributeKind kind, imnodes::PinShape = imnodes::PinShape_CircleFilled);
        virtual ~Attribute() {}

        void Render();

        virtual void RenderContents() = 0;

        const int id;
        const int node_id;
        const AttributeKind kind;
        const imnodes::PinShape shape;
    protected:
};

class TextAttribute : public Attribute {
    public:
        TextAttribute(int attribute_id, int node_id, const std::string& label, int text_field_size = 75);

        virtual ~TextAttribute() {}

        virtual void RenderContents() override;

    private:
        std::array<char, 256> buffer;
        const int text_field_size;
        const std::string label;
        const std::string imgui_label;
};

class QuadTextAttribute : public Attribute {
    public:
        QuadTextAttribute(int attribute_id, int node_id, const std::string& label, int text_field_size = 35);

        virtual ~QuadTextAttribute() {}

        virtual void RenderContents() override;

    private:
        std::array<char, 256> buffer_1;
        std::array<char, 256> buffer_2;
        std::array<char, 256> buffer_3;
        std::array<char, 256> buffer_4;
        const int text_field_size;
        const std::string label;
        const std::string imgui_label_1;
        const std::string imgui_label_2;
        const std::string imgui_label_3;
        const std::string imgui_label_4;
};

class OutputInterval : public Attribute {
    public:
        OutputInterval(int attribute_id, int node_id);

        virtual ~OutputInterval() {}

        virtual void RenderContents() override;

    private:
};

class OutputAttribute : public Attribute {
    public:
        OutputAttribute(int attribute_id, int node_id);

        virtual ~OutputAttribute() {}

        virtual void RenderContents() override;

    private:
};

class IntervalAttribute : public Attribute {
    public:
        IntervalAttribute(int attribute_id, int node_id, const std::string& label);

        virtual ~IntervalAttribute() {}

        virtual void RenderContents() override;

    private:
        const std::string label;
};

}
