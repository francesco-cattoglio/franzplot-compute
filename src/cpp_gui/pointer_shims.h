#pragma once

namespace imnodes {

    bool IsLinkCreated(int& started_at_attribute_id, int& ended_at_attribute_id);
    bool IsLinkCreated(int* started_at_attribute_id, int* ended_at_attribute_id, bool* created_from_snap);
}
