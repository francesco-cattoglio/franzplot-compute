#pragma once

#include "library.h"

namespace franzplot_gui {

class Attribute {
    public:
        Attribute(int attribute_id, int node_id);
        virtual ~Attribute() {}

        virtual void Render() {};

        const int id;
        const int node_id;
    protected:
};

class TextAttribute : public Attribute {
    public:
        TextAttribute(int attribute_id, int node_id, const std::string& label);

        virtual ~TextAttribute() {}

        virtual void Render() override;

    private:
        std::array<char, 256> buffer;
        const std::string label;
        const std::string imgui_label;
};

class OutputAttribute : public Attribute {
    public:
        OutputAttribute(int attribute_id, int node_id);

        virtual ~OutputAttribute() {}

        virtual void Render() override;

    private:
};

class IntervalAttribute : public Attribute {
    public:
        IntervalAttribute(int attribute_id, int node_id, const std::string& label);

        virtual ~IntervalAttribute() {}

        virtual void Render() override;

    private:
        const std::string label;
};

}
