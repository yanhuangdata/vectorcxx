#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_vector.hpp>

#include "vector_test_helper.h"
#include "vectorcxx/cxx_memory_queue_client.h"
#include <nlohmann/json.hpp>

using Catch::Matchers::VectorContains;
using vectorcxx::CxxMemoryQueueClient;
using vectorcxx::TopologyController;
using vectorcxx::test::run;
using vectorcxx::test::send_http_events;
using nlohmann::json;

TEST_CASE("consume events from memory queue") {
  run("http_to_memory_queue", [](rust::Box<TopologyController> &tc) {
    send_http_events({"e0", "e1"});

    auto &memory_queue_client = CxxMemoryQueueClient::get_instance();
    rust::Vec<vectorcxx::CxxLogEvent> events;
    do {
      events = memory_queue_client->poll();
    } while (events.empty());
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].get_string("_target_table") == "main");
    REQUIRE(events[0].get_string("\"-Target-Es\"") == "table_a");
    REQUIRE(events[0].get_string("source_type") == "http");
    REQUIRE(events[0].get_timestamp("timestamp") > 0);
    auto const &message = events[0].get_string("message");
    REQUIRE(message == "e0");

    do {
      events = memory_queue_client->poll();
    } while (events.empty());

    REQUIRE(events.size() == 1);
    REQUIRE(events[0].get_string("_target_table") == "main");
    REQUIRE(events[0].get_string("\"-Target-Es\"") == "table_a");
    REQUIRE(events[0].get_string("source_type") == "http");
    auto const &message_1 = events[0].get_string("message");
    REQUIRE(message_1 == "e1");
  });
}

TEST_CASE("iterate field names from pulled events") {
  run("http_to_memory_queue", [](rust::Box<TopologyController> &tc) {
    send_http_events({"e0", "e1"});
    auto &memory_queue_client = CxxMemoryQueueClient::get_instance();
    rust::Vec<vectorcxx::CxxLogEvent> events;
    do {
      events = memory_queue_client->poll();
    } while (events.empty());
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


TEST_CASE("consume events containing different value types") {
  run("http_json_to_memory_queue", [](rust::Box<TopologyController> &tc) {
    nlohmann::json event = R"(
      {
        "age": 99,
        "pi": 3.141,
        "happy": true,
        "words": "give me five"
      }
    )"_json;
    send_http_events({event.dump()});

    auto &memory_queue_client = CxxMemoryQueueClient::get_instance();
    rust::Vec<vectorcxx::CxxLogEvent> events;
    do {
      events = memory_queue_client->poll();
    } while (events.empty());

    auto fields = events[0].fields();
    REQUIRE(fields.size() > 0);
    std::vector<std::string> field_vec;
    field_vec.reserve(fields.size());
    for (auto const &field : fields) {
      field_vec.emplace_back(field);
    }
    REQUIRE_THAT(field_vec, VectorContains(std::string("timestamp")));
    REQUIRE_THAT(field_vec, VectorContains(std::string("source_type")));
    REQUIRE_THAT(field_vec, VectorContains(std::string("_source")));
    REQUIRE_THAT(field_vec, VectorContains(std::string("age")));
    REQUIRE_THAT(field_vec, VectorContains(std::string("happy")));
    REQUIRE_THAT(field_vec, VectorContains(std::string("pi")));
    REQUIRE_THAT(field_vec, VectorContains(std::string("words")));

    REQUIRE(events.size() == 1);
    REQUIRE(events[0].get_string("_target_table") == "main");
    REQUIRE(events[0].get_string("\"-Target-Es\"") == "table_a");
    REQUIRE(events[0].get_string("source_type") == "http");
    REQUIRE(events[0].get_timestamp("timestamp") > 0);
    REQUIRE(events[0].get_value_type("age") == "integer");
    REQUIRE(events[0].get_integer("age") == 99);
    REQUIRE(events[0].get_value_type("happy") == "boolean");
    REQUIRE(events[0].get_boolean("happy") == true);
    REQUIRE(events[0].get_value_type("pi") == "float");
    REQUIRE(events[0].get_double("pi") == 3.141);
    auto const &words = events[0].get_string("words");
    REQUIRE(words == "give me five");
  });
}