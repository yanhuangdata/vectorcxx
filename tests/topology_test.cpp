#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <catch2/matchers/catch_matchers_vector.hpp>

#include "vectorcxx/src/lib.rs.h"
#include <algorithm>
#include <cpr/cpr.h>
#include <exception>
#include <filesystem>
#include <fmt/format.h>
#include <iostream>
#include <nlohmann/json.hpp>
#include <regex>
#include <spdlog/spdlog.h>
#include <string>
#include <thread>

using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::VectorContains;
using vectorcxx::CxxLogEvent;

namespace {
  // this is the default file sink used by all testing configs
  inline auto FILE_SINK_PATH = std::filesystem::path("/tmp/vector_test_sink.log");
  inline auto DATA_DIR = std::filesystem::path("/tmp/vector");

  // get a file path under data folder
  std::filesystem::path _file_path(const std::string &file_name) {
    return std::filesystem::path(__FILE__).parent_path() / "data" / file_name;
  }

  std::string _load_config(const std::string &file_name) {
    std::ifstream stream(_file_path(file_name + ".json"));
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

  std::vector<std::string>
  _read_events_from_sink(const std::filesystem::path &file_path = FILE_SINK_PATH) {
    return _read_file_by_lines(file_path);
  }

  void _wait(uint32_t milliseconds = 200) {
    std::this_thread::sleep_for(std::chrono::milliseconds(milliseconds));
  }

  void send_http_events(const std::vector<std::string> &events, uint32_t port = 9999) {
    spdlog::info("sending events via http events={}", events.size());
    for (const auto &event : events) {
      auto res = cpr::Post(cpr::Url{fmt::format("http://localhost:{}", port)}, cpr::Body{event},
                           cpr::Header{{"_target_table", "main"}});
      spdlog::info("sending event to http status={}", res.status_code);
    }
  }

  struct VectorService {
    explicit VectorService(const std::string &config_file) {
      _setup();
      auto config = rust::String(_load_config(config_file));
      spdlog::info("starting vector");

      tc = vectorcxx::new_topology_controller();
      tc.value()->start(config);

      // wait for the vector service to start
      spdlog::info("waiting for vector to get started");
      _wait();
    }

    ~VectorService() {
      auto &controller = get_controller();
      controller->exit();
      controller->stop();
    }

    rust::Box<vectorcxx::TopologyController> &get_controller() {
      return tc.value();
    }

    static void _setup() {
      // ensure the file sink is cleared
      std::filesystem::remove(FILE_SINK_PATH);
      std::filesystem::remove_all(DATA_DIR);
      std::filesystem::create_directory(DATA_DIR);
    }

    std::optional<rust::Box<vectorcxx::TopologyController>> tc;
  };

  /**
   * run a topology with some operations performed during running
   * After running, the vector service will be stopped so that all the events are flushed and can be
   * asserted later
   */
  void run(const std::string &config_file,
           const std::function<void(rust::Box<vectorcxx::TopologyController> &)> &operations) {
    VectorService vector_service(config_file);
    operations(vector_service.get_controller());
  }
} // namespace

TEST_CASE("start single event http to file topology") {
  run("http_to_file",
      [](rust::Box<vectorcxx::TopologyController> &tc) { send_http_events({"hello"}); });
  auto events = _read_events_from_sink();
  REQUIRE(events.size() == 1);
}

TEST_CASE("start http to file topology") {
  run("http_to_file", [](rust::Box<vectorcxx::TopologyController> &tc) {
    send_http_events({"hello", "world"});
  });
  auto events = _read_events_from_sink();
  REQUIRE(events.size() == 2);
}

TEST_CASE("start http to file with transform topology") {
  run("http_to_file_with_transform", [](rust::Box<vectorcxx::TopologyController> &tc) {
    send_http_events({"e0", "e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8", "e9"});
  });
  auto events = _read_events_from_sink();
  REQUIRE(events.size() == 10);
  REQUIRE_THAT(events[0], ContainsSubstring("e0"));
  // added by a remap transform
  REQUIRE_THAT(events[0], ContainsSubstring("my_source"));
}

TEST_CASE("add new source to topology") {
  run("file_to_file", [](rust::Box<vectorcxx::TopologyController> &tc) {
    tc->add_config(_load_config("source/http"));
    _wait(1000);
    send_http_events({"hello", "world"});
  });
  auto events = _read_events_from_sink();
  REQUIRE(events.size() == 2);
}

TEST_CASE("update existing source in topology") {
  run("file_to_file", [](rust::Box<vectorcxx::TopologyController> &tc) {
    auto config = _load_config("source/http");
    tc->add_config(config);
    _wait(1000);
    uint32_t new_port = 8888;
    config = std::regex_replace(config, std::regex("9999"), std::to_string(new_port));
    tc->update_config(config);
    _wait(1000);
    send_http_events({"hello", "world"}, new_port);
  });
  auto events = _read_events_from_sink();
  REQUIRE(events.size() == 2);
}

TEST_CASE("delete transform from topology") {
  run("http_to_file_with_transform", [](rust::Box<vectorcxx::TopologyController> &tc) {
    tc->add_config(_load_config("transform/add_field"));
    _wait(2000);
    tc->delete_config({"transform_remap_field"});
    _wait(2000);
    send_http_events({"e0", "e1"});
  });
  auto events = _read_events_from_sink();
  REQUIRE(events.size() == 2);
  REQUIRE_THAT(events[0], ContainsSubstring("e0"));
  // the remap transform is deleted
  REQUIRE_THAT(events[0], !ContainsSubstring("my_source"));
  REQUIRE_THAT(events[0], ContainsSubstring("42"));
  REQUIRE_THAT(events[1], ContainsSubstring("e1"));
  // the remap transform is deleted
  REQUIRE_THAT(events[1], !ContainsSubstring("my_source"));
  REQUIRE_THAT(events[1], ContainsSubstring("42"));
}

// test if a new adding config impacts the previous added config
TEST_CASE("add two transform from topology") {
  run("http_to_file_with_transform", [](rust::Box<vectorcxx::TopologyController> &tc) {
    auto new_config = _load_config("source_with_transform/http_with_transform");
    tc->add_config(new_config);
    _wait(2000);
    std::string new_source_name = "source_http_2";
    std::string new_transform_name = "transform_add_field_2";
    std::string new_port = "8888";
    std::string new_id = "5678";
    new_config = std::regex_replace(new_config, std::regex("source_http_1"), new_source_name);
    new_config = std::regex_replace(new_config, std::regex("9998"), new_port);
    new_config =
        std::regex_replace(new_config, std::regex("transform_add_field_1"), new_transform_name);
    new_config = std::regex_replace(new_config, std::regex("1234"), new_id);
    tc->add_config(new_config);
    _wait(2000);
    send_http_events({"e0", "e1"});
  });
  auto events = _read_events_from_sink();
  REQUIRE(events.size() == 2);
}

TEST_CASE("topology controller init") {
  auto tc = vectorcxx::new_topology_controller();
  REQUIRE(tc.into_raw() != nullptr);
}

TEST_CASE("get generation id") {
  run("file_to_file", [](rust::Box<vectorcxx::TopologyController> &tc) {
    REQUIRE(tc->get_generation_id() == 0);
    auto config = _load_config("source/http");
    tc->add_config(config);
    _wait(1000);
    REQUIRE(tc->get_generation_id() == 1);
  });
}

TEST_CASE("consume events from memory queue") {
  run("http_to_memory_queue", [](rust::Box<vectorcxx::TopologyController> &tc) {
    send_http_events({"e0", "e1"});
    auto events = vectorcxx::poll_vector_events();
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].get("_target_table") == "main");
    REQUIRE(events[0].get("source_type") == "http");
    REQUIRE(events[0].get_timestamp("timestamp") > 0);
    auto const &message = events[0].get("message");
    REQUIRE(message == "e0");

    do {
      events = vectorcxx::poll_vector_events();
    } while (events.empty());

    REQUIRE(events.size() == 1);
    REQUIRE(events[0].get("_target_table") == "main");
    REQUIRE(events[0].get("source_type") == "http");
    auto const &message_1 = events[0].get("message");
    REQUIRE(message_1 == "e1");
  });
}

TEST_CASE("iterate field names from pulled events") {
  run("http_to_memory_queue", [](rust::Box<vectorcxx::TopologyController> &tc) {
    send_http_events({"e0", "e1"});
    auto events = vectorcxx::poll_vector_events();
    REQUIRE(events.size() == 1);
    auto fields = events[0].fields();
    REQUIRE(fields.size() > 0);

    std::vector<std::string> field_vec;
    field_vec.reserve(fields.size());
    for (auto const &field : fields) {
      field_vec.emplace_back(field);
    }
    REQUIRE_THAT(field_vec, VectorContains(std::string("message")));
    REQUIRE_THAT(field_vec, VectorContains(std::string("timestamp")));
    REQUIRE_THAT(field_vec, VectorContains(std::string("source_type")));
    REQUIRE_THAT(field_vec, VectorContains(std::string("_source")));
  });
}