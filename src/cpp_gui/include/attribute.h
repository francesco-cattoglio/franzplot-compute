#pragma once

class Attribute {
    public:
        virtual void Render() = 0;
};

class TextAttribute : Attribute {
    public:
        virtual void Render() override;
};
