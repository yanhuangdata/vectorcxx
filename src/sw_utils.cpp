#include "vectorcxx/src/lib.rs.h"
#include "vectorcxx/src/sw_utils.h"
#include <iostream>

bool vectorcxx::send_event_to_sw(rust::String event) {
    std::cout << "C++ get event: " << event.c_str() << std::endl;
    return true;
}

bool vectorcxx::send_events_to_sw(rust::Vec<vectorcxx::SwEvent> events) {
    for (auto ev : events) {
        std::cout << ev.message << std::endl;
        std::cout << ev.timestamp << std::endl;
    }
//    std::cout << "C++ get event: " << event.c_str() << std::endl;
    return true;
}

