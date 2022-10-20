#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_vector.hpp>

#include "vector_test_helper.h"
#include "vectorcxx/cxx_memory_queue_client.h"

using Catch::Matchers::VectorContains;
using vectorcxx::CxxMemoryQueueClient;
using vectorcxx::TopologyController;
using vectorcxx::test::run;
using vectorcxx::test::send_http_events;

TEST_CASE("consume events from memory queue") {
  run("http_to_memory_queue", [](rust::Box<TopologyController> &tc) {
    send_http_events({"e0", "e1"});

    auto &memory_queue_client = CxxMemoryQueueClient::get_instance();
    auto events = memory_queue_client->poll();
    REQUIRE(events.size() == 1);
    REQUIRE(events[0].get("_target_table") == "main");
    REQUIRE(events[0].get("source_type") == "http");
    REQUIRE(events[0].get_timestamp("timestamp") > 0);
    auto const &message = events[0].get("message");
    REQUIRE(message == "e0");

    do {
      events = memory_queue_client->poll();
    } while (events.empty());

    REQUIRE(events.size() == 1);
    REQUIRE(events[0].get("_target_table") == "main");
    REQUIRE(events[0].get("source_type") == "http");
    auto const &message_1 = events[0].get("message");
    REQUIRE(message_1 == "e1");
  });
}

TEST_CASE("iterate field names from pulled events") {
  run("http_to_memory_queue", [](rust::Box<TopologyController> &tc) {
    send_http_events({"e0", "e1"});
    auto &memory_queue_client = CxxMemoryQueueClient::get_instance();
    auto events = memory_queue_client->poll();
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
