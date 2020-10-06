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

        virtual bool IsCompatible(Attribute* other);
        virtual void RenderContents() = 0;

        const int id;
        const int node_id;
        const AttributeKind kind;
        const imnodes::PinShape shape;
    protected:
};

class Text final : public Attribute {
    public:
        Text(int attribute_id, int node_id, const std::string& label, int text_field_size = 75);

        void RenderContents() override;

    private:
        std::array<char, 256> buffer;
        const int text_field_size;
        const std::string label;
        const std::string imgui_label;
};

class QuadText final : public Attribute {
    public:
        QuadText(int attribute_id, int node_id, const std::string& label, int text_field_size = 35);

        void RenderContents() override;

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

class OutputInterval final : public Attribute {
    public:
        OutputInterval(int attribute_id, int node_id);

        void RenderContents() override;

    private:
};

class Output final : public Attribute {
    public:
        Output(int attribute_id, int node_id);

        void RenderContents() override;

    private:
};

class InputInterval final : public Attribute {
    public:
        InputInterval(int attribute_id, int node_id, const std::string& label);

        void RenderContents() override;

    private:
        const std::string label;
};

}
