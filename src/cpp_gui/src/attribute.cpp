#include "attribute.h"

#include <iostream>

void TextAttribute::Render() {
    std::cout << "Rendering attribute with id " << this->id << std::endl;
    return;
}
