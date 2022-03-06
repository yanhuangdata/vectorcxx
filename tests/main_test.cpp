#include "vectorcxx/src/lib.rs.h"
#include "catch2/catch.hpp"
#include <exception>
#include <iostream>

TEST_CASE("vectorcxx api test") {
    try {
        struct vectorcxx::KafkaSinkParams kafka_params {
            "host.docker.internal:9092", "quickstart-events-1", "", ""
        };
        auto result = vectorcxx::export_to_kafka("00000", "/tmp/data", "/tmp/data_dir", kafka_params);
        auto err_msg = result.err_msg.c_str();
        std::cout << "result: " << result.succeed << ", " << err_msg << std::endl;

        struct vectorcxx::FileSinkParams file_params {"/tmp/data/result_file.log"};
        auto new_result = vectorcxx::export_to_file("00000", "/tmp/data", "/tmp/data_dir", file_params);
        auto new_err_msg = new_result.err_msg.c_str();
        std::cout << "new result: " << new_result.succeed << ", " << new_err_msg << std::endl;
    } catch (std::exception& e) {
        std::cout << "exception: " << e.what() << std::endl;
    }
}

