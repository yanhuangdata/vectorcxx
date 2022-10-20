#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>


#include "vector_test_helper.h"
#include <exception>
#include <regex>
#include <string>

using Catch::Matchers::ContainsSubstring;
using vectorcxx::test::run;
using vectorcxx::test::send_http_events;
using vectorcxx::test::read_events_from_sink;
using vectorcxx::test::load_config;
using vectorcxx::test::wait;
using vectorcxx::TopologyController;

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
    wait(1000);
    send_http_events({"hello", "world"});
  });
  auto events = read_events_from_sink();
  REQUIRE(events.size() == 2);
}

TEST_CASE("update existing source in topology") {
  run("file_to_file", [](rust::Box<TopologyController> &tc) {
    auto config = load_config("source/http");
    tc->add_config(config);
    wait(1000);
    uint32_t new_port = 8888;
    config = std::regex_replace(config, std::regex("9999"), std::to_string(new_port));
    tc->update_config(config);
    wait(1000);
    send_http_events({"hello", "world"}, new_port);
  });
  auto events = read_events_from_sink();
  REQUIRE(events.size() == 2);
}

TEST_CASE("delete transform from topology") {
  run("http_to_file_with_transform", [](rust::Box<TopologyController> &tc) {
    tc->add_config(load_config("transform/add_field"));
    wait(2000);
    tc->delete_config({"transform_remap_field"});
    wait(2000);
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
    wait(2000);
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
    wait(2000);
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
    REQUIRE(tc->get_generation_id() == 0);
    auto config = load_config("source/http");
    tc->add_config(config);
    wait(1000);
    REQUIRE(tc->get_generation_id() == 1);
  });
}