#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>


#include "vector_test_helper.h"
#include <exception>
#include <regex>
#include <string>
#include <iostream>

using Catch::Matchers::ContainsSubstring;
using vectorcxx::test::run;
using vectorcxx::test::run_one_shot;
using vectorcxx::test::send_http_events;
using vectorcxx::test::read_events_from_sink;
using vectorcxx::test::load_config;
using vectorcxx::test::wait;
using vectorcxx::TopologyController;
using vectorcxx::OneShotTopologyController;

TEST_CASE("start single event http to file topology") {
  run("http_to_file",
      [](rust::Box<TopologyController> &tc) { send_http_events({"hello"}); });
  auto events = read_events_from_sink();
  REQUIRE(events.size() == 1);
}

TEST_CASE("start http to file topology") {
  run("http_to_file", [](rust::Box<TopologyController> &tc) {
    send_http_events({"hello", "world"});
  });
  auto events = read_events_from_sink();
  REQUIRE(events.size() == 2);
}

TEST_CASE("start http to file with transform topology") {
  run("http_to_file_with_transform", [](rust::Box<TopologyController> &tc) {
    send_http_events({"e0", "e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8", "e9"});
  });
  auto events = read_events_from_sink();
  REQUIRE(events.size() == 10);
  REQUIRE_THAT(events[0], ContainsSubstring("e0"));
  // added by a remap transform
  REQUIRE_THAT(events[0], ContainsSubstring("my_source"));
}

TEST_CASE("add new source to topology") {
  run("file_to_file", [](rust::Box<TopologyController> &tc) {
    tc->add_config(load_config("source/http"));
    send_http_events({"hello", "world"});
  });
  auto events = read_events_from_sink();
  REQUIRE(events.size() == 2);
}

TEST_CASE("update existing source in topology") {
  run("file_to_file", [](rust::Box<TopologyController> &tc) {
    auto config = load_config("source/http");
    tc->add_config(config);
    uint32_t new_port = 8888;
    config = std::regex_replace(config, std::regex("9999"), std::to_string(new_port));
    tc->update_config(config);
    send_http_events({"hello", "world"}, new_port);
  });
  auto events = read_events_from_sink();
  REQUIRE(events.size() == 2);
}

TEST_CASE("delete transform from topology") {
  run("http_to_file_with_transform", [](rust::Box<TopologyController> &tc) {
    tc->add_config(load_config("transform/add_field"));
    tc->delete_config({"transform_remap_field"});
    send_http_events({"e0", "e1"});
  });
  auto events = read_events_from_sink();
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
  run("http_to_file_with_transform", [](rust::Box<TopologyController> &tc) {
    auto new_config = load_config("source_with_transform/http_with_transform");
    tc->add_config(new_config);
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
    send_http_events({"e0", "e1"});
  });
  auto events = read_events_from_sink();
  REQUIRE(events.size() == 2);
}

TEST_CASE("topology controller init") {
  auto tc = vectorcxx::new_topology_controller();
  REQUIRE(tc.into_raw() != nullptr);
}

TEST_CASE("get generation id") {
  run("file_to_file", [](rust::Box<TopologyController> &tc) {
    REQUIRE(tc->get_generation_id() == 1);
    auto config = load_config("source/http");
    tc->add_config(config);
    REQUIRE(tc->get_generation_id() == 2);
  });
}

TEST_CASE("test one shot topology") {
  run_one_shot("batch_file_to_file", [](rust::Box<OneShotTopologyController> &tc) {});
  auto events = read_events_from_sink();
  REQUIRE(events.size() == 2);
}

// test a kafka sink which is not started, which will fail on health check
TEST_CASE("test one shot topology with sink not healthy") {
  try {
    run_one_shot("batch_file_to_kafka", [](rust::Box<OneShotTopologyController> &tc) {});
  } catch (std::exception &e) {
    REQUIRE(strcmp(e.what(), "health check for sink failed") == 0);
  }
}

TEST_CASE("run vector service with one time topology") {
  run("file_to_file", [](rust::Box<TopologyController> &tc) {
    tc->add_config(load_config("source/http"));
    send_http_events({"hello", "world"});

    // add a one time topology
    auto config = load_config("batch_file_to_file");
    auto tc_one_shot = vectorcxx::new_one_shot_topology_controller();
    REQUIRE(tc_one_shot->start(config));

    // update the long run service config
    uint32_t new_port = 8888;
    config = std::regex_replace(config, std::regex("9999"), std::to_string(new_port));
    tc->update_config(config);
    send_http_events({"hello", "world"}, new_port);
  });
  auto events = read_events_from_sink();
  REQUIRE(events.size() == 6);
}

TEST_CASE("test one shot topology with invalid source config") {
  try {
    run_one_shot("file_to_file_invalid", [](rust::Box<OneShotTopologyController> &tc) {});
  } catch (std::exception &e) {
    std::cout << "error: " << e.what() << std::endl;
    REQUIRE_THAT(e.what(), ContainsSubstring("configuration error: \"unknown variant `beginnings`"));
  }
}

TEST_CASE("test reload vector from valid config string can proceed") {
  auto config_string = R"j(
    {
      "data_dir": "/tmp/vector/",
      "sources": {
        "source_http": {
          "type": "http_server",
          "address": "0.0.0.0:9999",
          "encoding": "text"
        }
      },
      "sinks": {
        "sink_file": {
          "type": "file",
          "inputs": [
            "transform_*"
          ],
          "encoding": {
            "codec": "json"
          },
          "path": "/tmp/vector_test_sink.log"
        }
      },
      "transforms": {
        "transform_add_field": {
          "type": "remap",
          "inputs": ["source_*"],
          "source": ".age = 99 \n .abc=1"
        }
      }
    }
  )j";
  // start topology controller
  run("http_to_file",
    [&config_string](rust::Box<TopologyController> &tc) { 
      send_http_events({"hello"}); 
      // reload with config str
      auto config_str = rust::String(config_string);
      REQUIRE(tc->handle_config_reload(config_str));
      send_http_events({"hello", "world"});
  });
  auto events = read_events_from_sink();
  REQUIRE(events.size() == 3);
  REQUIRE_THAT(events[0], !ContainsSubstring(R"("age")"));
  REQUIRE_THAT(events[1], ContainsSubstring(R"("age")"));
  REQUIRE_THAT(events[2], ContainsSubstring(R"("age")"));
}

TEST_CASE("test reload vector from invalid config string should fallback to existing topology config") {
  auto config_string = R"j({"data_dir": "/tmp/vector/","sources": {"source_http": {"type": "http_server_invalid","address": "0.0.0.0:9999","encoding": "text"}},
      "sinks": {
        "sink_file": {
          "type": "file",
          "inputs": [
            "transform_*"
          ],
          "encoding": {
            "codec": "json"
          },
          "path": "/tmp/vector_test_sink.log"
        }
      },
      "transforms": {
        "transform_add_field": {
          "type": "remap",
          "inputs": ["source_*"],
          "source": ".age = 42"
        }
      }
    }
  )j";
  // start topology controller
  run("http_to_file",
    [&config_string](rust::Box<TopologyController> &tc) { 
      send_http_events({"hello"}); 
      // reload with config str
      auto config_str = rust::String(config_string);
      REQUIRE_THROWS(tc->handle_config_reload(config_str));
      send_http_events({"hello", "world"});
  });
  auto events = read_events_from_sink();
  REQUIRE(events.size() == 3);
  REQUIRE_THAT(events[0], !ContainsSubstring(R"("age")"));
  REQUIRE_THAT(events[1], !ContainsSubstring(R"("age")"));
  REQUIRE_THAT(events[2], !ContainsSubstring(R"("age")"));
}

TEST_CASE("test reload vector from config string with health check") {
  // source includes an invalid port `99999`, which should fail the health check and reload.
  auto config_string = R"j(
    {
      "data_dir": "/tmp/vector/",
      "sources": {
        "source_http": {
          "type": "http_server",
          "address": "0.0.0.0:99999",
          "encoding": "text"
        }
      },
      "sinks": {
        "sink_file": {
          "type": "file",
          "inputs": [
            "transform_*"
          ],
          "encoding": {
            "codec": "json"
          },
          "path": "/tmp/vector_test_sink.log"
        }
      },
      "transforms": {
        "transform_add_field": {
          "type": "remap",
          "inputs": ["source_*"],
          "source": ".age = 42"
        }
      }
    }
  )j";
  // start topology controller
  run("http_to_file",
    [&config_string](rust::Box<TopologyController> &tc) { 
      send_http_events({"hello"}); 
      // reload with config str
      auto config_str = rust::String(config_string);
      REQUIRE_THROWS(tc->handle_config_reload(config_str));
      send_http_events({"hello", "world"});
  });
  auto events = read_events_from_sink();
  REQUIRE(events.size() == 3);
  REQUIRE_THAT(events[0], !ContainsSubstring(R"("age")"));
  REQUIRE_THAT(events[1], !ContainsSubstring(R"("age")"));
  REQUIRE_THAT(events[2], !ContainsSubstring(R"("age")"));
}