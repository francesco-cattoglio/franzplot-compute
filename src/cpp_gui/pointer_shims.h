#pragma once

namespace imnodes {

    // forward declarations of the actual imnodes functions
    bool IsLinkCreated(int* started_at_attribute_id, int* ended_at_attribute_id, bool* created_from_snap);
    bool IsLinkHovered(int* id);
    bool IsNodeHovered(int* id);

    // declaration of our shims
    bool IsLinkCreated(int& started_at_attribute_id, int& ended_at_attribute_id);
    bool IsLinkHovered(int& id);
    bool IsNodeHovered(int& id);

}
