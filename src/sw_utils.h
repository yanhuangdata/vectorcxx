#pragma once
#include "rust/cxx.h"

namespace vectorcxx {
    struct SwEvent;

    bool send_event_to_sw(rust::String event);

    bool send_events_to_sw(rust::Vec <SwEvent> events);
}
