#include "graph.h"

#include <imnodes.h>

namespace franzplot_gui {

void Graph::Render() {

    imnodes::BeginNodeEditor();
    // render all links
    for (auto& entry : this->input_to_output_links) {
        int link_id = entry.first;
        int in_attribute_id = link_id;
        int out_attribute_id = entry.second;
        imnodes::Link(link_id, in_attribute_id, out_attribute_id);
    }

    for (auto& entry : this->nodes) {
        entry.second.Render();
    }

    imnodes::EndNodeEditor();

    // event processing

    // check if a link was destroyed;
    int link_id;
    if (imnodes::IsLinkDestroyed(&link_id)) {
        this->input_to_output_links.erase(link_id);
    }

    int start_attr, end_attr;
    if (imnodes::IsLinkCreated(&start_attr, &end_attr))
    {
        // check which one of the two attributes is the input attribute and which is the output
        int in_attr, out_attr;
        if (this->attributes[start_attr]->kind == AttributeKind::In) {
            in_attr = start_attr;
            out_attr = end_attr;
        } else {
            in_attr = end_attr;
            out_attr = start_attr;
        }
        this->input_to_output_links[in_attr] = out_attr;
    }

}

int Graph::NextId() {
    return this->next_id++;
}

void Graph::Test() {
    AddNode(Node::TemplatedCurve(std::bind(&Graph::NextId, this)));
}

void Graph::AddNode(Node&& node) {
    // we need to keep our attribute-to-node map up-to-date
    for (std::shared_ptr<Attribute> attribute : node.in_attributes)
        attributes[attribute->id] = attribute;

    for (std::shared_ptr<Attribute> attribute : node.out_attributes)
        attributes[attribute->id] = attribute;

    for (std::shared_ptr<Attribute> attribute : node.static_attributes)
        attributes[attribute->id] = attribute;

    nodes.insert(std::make_pair(node.id, node));
}


}
