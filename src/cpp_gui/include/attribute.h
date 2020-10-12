#pragma once

#include <array>
#include <string>

#include <imnodes.h>

namespace franzplot_gui {

enum class AttributeKind {
    Input,
    Output,
    Static
};

enum class PinKind {
    Interval,
    Geometry,
    Matrix
};

class Attribute {
    public:
        Attribute(int attribute_id, int node_id, const std::string& label, AttributeKind kind);
        virtual ~Attribute() {}

        virtual void Render() = 0;

        const int id;
        const int node_id;
        const std::string label;
        const AttributeKind kind;
    protected:
};

class InputAttribute : public Attribute {
    public:
        InputAttribute(int attribute_id, int node_id, const std::string& label, PinKind pin_kind);
        virtual ~InputAttribute() {}

        void Render() final;
        virtual void RenderContents() = 0;

        const PinKind pin_kind;
};

class OutputAttribute : public Attribute {
    public:
        OutputAttribute(int attribute_id, int node_id, const std::string& label, PinKind pin_kind);
        virtual ~OutputAttribute() {}

        void Render() final;
        virtual void RenderContents() = 0;

        const PinKind pin_kind;
};

class StaticAttribute : public Attribute {
    public:
        StaticAttribute(int attribute_id, int node_id, const std::string& label);
        virtual ~StaticAttribute() {}

        void Render() final;
        virtual std::string ContentsToJson() = 0;
        virtual void RenderContents() = 0;
};

class SimpleInput final : public InputAttribute {
    public:
        SimpleInput(int attribute_id, int node_id, const std::string& label, PinKind pin_kind);

        void RenderContents() override;

    private:
};

class SimpleOutput final : public OutputAttribute {
    public:
        SimpleOutput(int attribute_id, int node_id, const std::string& label, PinKind pin_kind);

        void RenderContents() override;

    private:
};

class IntSlider final : public StaticAttribute {
    public:
        IntSlider(int attribute_id, int node_id, const std::string& label, int min, int max);

        std::string ContentsToJson() override;
        void RenderContents() override;

    private:
        int min;
        int max;
        int value;
};

class Text final : public StaticAttribute {
    public:
        Text(int attribute_id, int node_id, const std::string& label, int text_field_size = 75);

        std::string ContentsToJson() override;
        void RenderContents() override;

        std::string buffer;

    private:
        const std::string imgui_label;
        const int text_field_size;
};

class MatrixRow final : public StaticAttribute {
    public:
        MatrixRow(int attribute_id, int node_id, const std::string& label, int text_field_size = 35);

        std::string ContentsToJson() override;
        void RenderContents() override;

        std::array<std::string, 4> buffer;

    private:
        const std::array<const std::string, 4> imgui_label;
        const int text_field_size;
};

// some helper function definitions
imnodes::PinShape ToShape(PinKind kind);
bool IsCompatible(InputAttribute& input, OutputAttribute& output);

}
