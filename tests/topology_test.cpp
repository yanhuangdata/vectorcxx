#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include "vectorcxx/src/lib.rs.h"
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

  struct ConsumedEvents {
    bool parsed;
    std::string source_type;
    std::string target;
    std::string message;
  };

  std::vector<ConsumedEvents> consume_events(uint32_t expected) {
    std::vector<ConsumedEvents> events;
    events.reserve(expected);

    auto read_event_count = 0;
    while (read_event_count < expected) {
      try {
        auto result = vectorcxx::poll_vector_events();
        if (result.events.empty()) {
          std::this_thread::sleep_for(std::chrono::milliseconds(100));
        } else {
          spdlog::info("events pulled from memory queue event_count={}", result.events.size());
          for (const auto &ev : result.events) {
            events.emplace_back(
                ConsumedEvents{ev.parsed, std::string(ev.source_type.data(), ev.source_type.size()),
                               std::string(ev.target.data(), ev.target.size()),
                               std::string(ev.message.data(), ev.message.size())});
          }
          read_event_count += result.events.size();
        }
      } catch (std::exception &e) {
        spdlog::error("polling failed", "error", e.what());
      }
    }
    return events;
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
    auto events = consume_events(2);
    REQUIRE(events.size() == 2);
    REQUIRE(events[0].parsed == false);
    REQUIRE(events[0].target == "main");
    REQUIRE(events[0].source_type == "http");
    auto message = nlohmann::json::parse(events[0].message);
    REQUIRE(message.at("_source") == "my_source");
    REQUIRE(message.at("message") == "e0");
    REQUIRE(message.contains("timestamp"));

    auto message_1 = nlohmann::json::parse(events[1].message);
    REQUIRE(message_1.at("_source") == "my_source");
    REQUIRE(message_1.at("message") == "e1");
    REQUIRE(message_1.contains("timestamp"));
  });
}