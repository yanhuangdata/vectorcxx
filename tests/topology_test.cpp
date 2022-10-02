#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include "vectorcxx/src/lib.rs.h"
#include <exception>
#include <iostream>
#include <cstdio>
#include <fstream>
#include <thread>
#include <nlohmann/json.hpp>
#include <filesystem>
#include <sstream>

namespace {
    // get a file path under data folder
    std::filesystem::path _file_path(const std::string &file_name) {
        return std::filesystem::path(__FILE__).parent_path() / "data" / file_name;
    }

    std::string _load_file(const std::string &file_name) {
        std::ifstream stream(_file_path(file_name));
        std::stringstream buffer;
        buffer << stream.rdbuf();
        auto file_content = buffer.str();
        // validate the file is a valid json
        auto _ = nlohmann::json::parse(file_content);
        return file_content;
    }

    std::vector<std::string> _read_file_by_lines(const std::filesystem::path &file_path) {
        std::ifstream input(file_path);
        std::vector<std::string> lines;
        for (std::string line; getline(input, line);) {
            lines.emplace_back(std::move(line));
        }
        return lines;
    }

    void _send_events_to_stdin(const std::string &events_file) {
        REQUIRE(std::freopen(_file_path(events_file).c_str(), "r", stdin));
    }

    // a list of pre-configured vector configs for testing
    static constexpr const char stdin_to_file[] = "stdin_to_file.json";
    static constexpr const char file_to_console[] = "file_to_console.json";
    static constexpr const char file_to_file[] = "file_to_file.json";
    static constexpr const char stdin_to_file_with_transform[] = "stdin_to_file_with_transform.json";

    // this is the default file sink used by all testing configs
    static inline auto FILE_SINK_PATH = std::filesystem::path("/tmp/vector_test.log");

    template<char const *CONFIG_FILE>
    struct VectorService {
        VectorService() {
            // ensure the file sink is cleared
            std::filesystem::remove(FILE_SINK_PATH);
            auto config = rust::String(_load_file(CONFIG_FILE));
            vector_service_thread = std::thread(vectorcxx::start_ingest_to_vector, config);
        }

        ~VectorService() {
            vectorcxx::crud_vector_config("exit", {}, "", 0);
            vector_service_thread.join();
        }

        std::thread vector_service_thread;
    };

    void _wait(uint32_t milliseconds = 100) {
        std::this_thread::sleep_for(std::chrono::milliseconds(milliseconds));
    }
}

TEST_CASE_METHOD(VectorService<stdin_to_file>, "start stdin to file topology") {
   _send_events_to_stdin("events.log");
   _wait();
   auto events = _read_file_by_lines(FILE_SINK_PATH);
   REQUIRE(events.size() == 10);
}

// TEST_CASE_METHOD(VectorService<stdin_to_file_with_transform>, "start topology with transformation") {
//     _send_events_to_stdin("events.log");
//     _wait();
//     auto events = _read_file_by_lines(FILE_SINK_PATH);
//     REQUIRE(events.size() == 9);
// }