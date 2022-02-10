#include "vectorcxx/src/lib.rs.h"
#include "catch2/catch.hpp"
#include <exception>
#include <iostream>
#include <librdkafka/rdkafka.h>

TEST_CASE("vectorcxx api test") {
    try {
        auto result = vectorcxx::export_to_kafka("00000", "/tmp/data", "/tmp/data_dir", "host.docker.internal:9092", "quickstart-events-1");
        auto err_msg = result.err_msg.c_str();
        std::cout << "result: " << result.succeed << ", " << err_msg << std::endl;
    } catch (std::exception& e) {
        std::cout << "exception: " << e.what() << std::endl;
    }
}

