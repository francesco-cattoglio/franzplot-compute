#include "pointer_shims.h"

namespace imnodes {

    bool IsLinkCreated(int& started_at_attribute_id, int& ended_at_attribute_id) {
        return IsLinkCreated(&started_at_attribute_id, &ended_at_attribute_id, nullptr);
    }

    bool IsLinkHovered(int& id) {
        return IsLinkHovered(&id);
    }

    bool IsNodeHovered(int& id) {
        return IsNodeHovered(&id);
    }
}
