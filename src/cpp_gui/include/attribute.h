#pragma once

class Attribute {
    public:
        Attribute(int id) : id(id) {}
        virtual ~Attribute() {}

        virtual void Render() = 0;

    protected:
        int id;
};

class TextAttribute : Attribute {
    public:
        TextAttribute(int id) : Attribute(id) {}
        virtual ~TextAttribute() {}

        virtual void Render() override;
};
