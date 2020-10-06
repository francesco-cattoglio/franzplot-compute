#pragma once

#include "library.h"

namespace franzplot_gui {

enum class AttributeKind {
    In,
    Out,
    Static,
    Unknown
};

class Attribute {
    public:
        Attribute(int attribute_id, int node_id, AttributeKind kind);
        virtual ~Attribute() {}

        void Render();

        virtual void RenderContents() = 0;

        const int id;
        const int node_id;
        const AttributeKind kind;
    protected:
};

class TextAttribute : public Attribute {
    public:
        TextAttribute(int attribute_id, int node_id, const std::string& label);

        virtual ~TextAttribute() {}

        virtual void RenderContents() override;

    private:
        std::array<char, 256> buffer;
        const std::string label;
        const std::string imgui_label;
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
