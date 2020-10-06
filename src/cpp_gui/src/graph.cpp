#include "graph.h"

#include <imnodes.h>

namespace franzplot_gui {

void Graph::Render() {

    imnodes::BeginNodeEditor();
    // render all links
    for (auto& entry : this->links) {
        int link_idx = entry.first;
        auto attr_pair = entry.second;
        imnodes::Link(link_idx, attr_pair.first, attr_pair.second);
    }

    for (auto& entry : this->nodes) {
        entry.second.Render();
    }

    imnodes::EndNodeEditor();

    // event processing

    int start_attr, end_attr;
    if (imnodes::IsLinkCreated(&start_attr, &end_attr))
    {
        auto attr_pair = std::make_pair(start_attr, end_attr);
        this->links.insert(std::make_pair(this->next_id++, attr_pair));
    }
    int link_id;
    if (imnodes::IsLinkDestroyed(&link_id)) {
        this->links.erase(link_id);
    }

}

int Graph::NextId() {
    return this->next_id++;
}

void Graph::Test() {
    Node new_node = Node::TemplatedCurve(std::bind(&Graph::NextId, this));
    nodes.insert(std::make_pair(new_node.id, std::move(new_node)));
}

}
