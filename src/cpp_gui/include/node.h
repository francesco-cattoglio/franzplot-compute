#pragma once

#include <vector>
#include <memory>

#include "attribute.h"
class Node {

    private:
        std::vector<std::shader_ptr<Attribute>> _in_attributes;
        std::vector<std::shader_ptr<Attribute>> _out_attributes;
        std::vector<std::shader_ptr<Attribute>> _static_attributes;
};
