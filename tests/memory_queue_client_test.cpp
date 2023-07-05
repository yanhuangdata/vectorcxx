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

TEST_CASE("consume events by constructor") {
  auto events = vectorcxx::new_cxx_log_events({"message", "e0", "new_field", "new_value"}, 2);
  REQUIRE(events.size() == 4);
  auto event = &events[0];
  auto non_utf_event = &events[2];
  auto another_event = &events[3];
  auto const &message = event->get_string("message");
  auto const &fields = event->fields();
  REQUIRE(message == "e0");
  REQUIRE(fields.size() == 2);
  REQUIRE(fields[0] == "message");
  REQUIRE(fields[1] == "new_field");
  REQUIRE(event->get_string("new_field") == "new_value");

  auto const &new_fields = non_utf_event->fields();
  REQUIRE(new_fields.size() == 1);
  REQUIRE(new_fields[0] == "invalid_key");
  REQUIRE(non_utf_event->get_string("invalid_key") == "Hello �World");

  auto const &another_fields = another_event->fields();
  REQUIRE(another_fields.size() == 1);
  REQUIRE(another_fields[0] == "invalid_key");
  REQUIRE(another_event->get_string("invalid_key") == "Hello �World");
}

TEST_CASE("poll random generated events") {
  auto events_total = 2000;
  auto batch_size = 100;
  auto batches_expected = (events_total - 1) / batch_size + 1;
  auto memory_queue_client = vectorcxx::new_memory_queue_client_with_random_events(
    2000, events_total, 100, batch_size, false);
  rust::Vec<vectorcxx::CxxLogEvent> events;
  uint32_t events_got = 0;
  uint32_t batches_got = 0;
  do {
    events = memory_queue_client->poll();
    events_got += events.size();
    if (!events.empty()) {
      batches_got += 1;
      REQUIRE(events[0].get_string("_target_table") == "table_a");
    }
  } while (!events.empty());
  REQUIRE(events_got == events_total);
  REQUIRE(batches_got == batches_expected);
}

TEST_CASE("poll random generated json events") {
  auto events_total = 100;
  auto batch_size = 50;
  auto batches_expected = (events_total - 1) / batch_size + 1;
  auto event_len = 100;
  // expected generated fields and "_message", "_target_table", "_datatype"
  auto fields_expected = event_len / 50 + 3;
  auto memory_queue_client = vectorcxx::new_memory_queue_client_with_random_events(
    2000, events_total, event_len, batch_size, true);
  rust::Vec<vectorcxx::CxxLogEvent> events;
  uint32_t events_got = 0;
  uint32_t batches_got = 0;
  do {
    events = memory_queue_client->poll();
    events_got += events.size();
    if (!events.empty()) {
      batches_got += 1;
      REQUIRE(events[0].get_string("_target_table") == "table_a");
      REQUIRE(events[0].get_string("_datatype") == "json");
      auto fields = events[0].fields();
      REQUIRE(fields.size() == fields_expected);
    }
  } while (!events.empty());
  REQUIRE(events_got == events_total);
  REQUIRE(batches_got == batches_expected);
}

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
    REQUIRE(events[0].get_string("-Target-Es") == "table_a");
    REQUIRE(events[0].get_string("source_type") == "http");
    REQUIRE(events[0].get_timestamp("timestamp") > 0);
    auto const &message = events[0].get_string("message");
    REQUIRE(message == "e0");

    do {
      events = memory_queue_client->poll();
    } while (events.empty());

    REQUIRE(events.size() == 1);
    REQUIRE(events[0].get_string("_target_table") == "main");
    REQUIRE(events[0].get_string("-Target-Es") == "table_a");
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

    REQUIRE(events.size() == 1);
    REQUIRE(events[0].get_string("_target_table") == "main");
    REQUIRE(events[0].get_string("-Target-Es") == "table_a");
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

TEST_CASE("consume events with raw message") {
  run("http_to_memory_queue_with_parsing", [](rust::Box<TopologyController> &tc) {
    nlohmann::json event = nlohmann::json::parse(
      R"({"_datatype":"json","message":"{\"b\":2}","_time":1})");
    send_http_events({event.dump()});

    auto &memory_queue_client = CxxMemoryQueueClient::get_instance();
    rust::Vec<vectorcxx::CxxLogEvent> events;
    do {
      events = memory_queue_client->poll();
    } while (events.empty());

    REQUIRE(events.size() == 1);
    REQUIRE(std::string(events[0].get_string("_datatype").data(), events[0].get_string("_datatype").size()) == "json");
    REQUIRE(std::string(events[0].get_string("_message").data(), events[0].get_string("_message").size()) == "{\"b\":2}");

  });
}

TEST_CASE("consume events with object and array field") {
  run("http_to_memory_queue_with_parsing", [](rust::Box<TopologyController> &tc) {
    nlohmann::json event = nlohmann::json::parse(
      R"({"_datatype":"json", "empty_obj":{},"empty_list":[], "some_obj":{"k1": "v1"},"some_list":[1, 2]})");
    send_http_events({event.dump()});

    auto &memory_queue_client = CxxMemoryQueueClient::get_instance();
    rust::Vec<vectorcxx::CxxLogEvent> events;
    do {
      events = memory_queue_client->poll();
    } while (events.empty());

    // std::vector<std::vector<std::string>> field_vec;
    // auto fields = events[0].fields();
    // field_vec.reserve(fields.size());
    // for (auto const &field : fields) {
    //   auto field_name = std::string(field.data(), field.size());
    //   auto type = events[0].get_value_type(field);
    //   auto field_type = std::string(type.data(), type.size());
    //   std::vector<std::string> one_field = {field_name, field_type};
    //   field_vec.emplace_back(one_field);
    //   std::cout << "key: " << field_name << ", type: " << field_type << std::endl;
    // }

    REQUIRE(events.size() == 1);
    REQUIRE(std::string(events[0].get_string("_datatype").data(), events[0].get_string("_datatype").size()) == "json");

    REQUIRE(std::string(events[0].get_object_as_string("empty_obj").data(), events[0].get_object_as_string("empty_obj").size()) == "{}");
    REQUIRE(std::string(events[0].get_array_as_string("empty_list").data(), events[0].get_array_as_string("empty_list").size()) == "[]");
    REQUIRE(events[0].get_string_array("empty_list").empty());

    REQUIRE(std::string(events[0].get_object_as_string("some_obj").data(), events[0].get_object_as_string("some_obj").size()) == "{\"k1\":\"v1\"}");
    REQUIRE(std::string(events[0].get_array_as_string("some_list").data(), events[0].get_array_as_string("some_list").size()) == "[1,2]");
    auto some_list = events[0].get_string_array("some_list");
    REQUIRE(some_list.size() == 2);
    REQUIRE(std::string(some_list[0].data(), some_list[0].size()) == "1");
    REQUIRE(std::string(some_list[1].data(), some_list[1].size()) == "2");

    REQUIRE(std::string(events[0].get_value_type("some_list[0]").data(), events[0].get_value_type("some_list[0]").size()) == "integer");
    REQUIRE(events[0].get_integer("some_list[0]") == 1);
    REQUIRE(std::string(events[0].get_value_type("some_obj.k1").data(), events[0].get_value_type("some_obj.k1").size()) == "string");
    REQUIRE(std::string(events[0].get_string("some_obj.k1").data(), events[0].get_string("some_obj.k1").size()) == "v1");
    // REQUIRE(std::string(events[0].get_string("_message").data(), events[0].get_string("_message").size()) == "{\"b\":2}");

  });
}

TEST_CASE("consume events with chinese field name") {
  run("http_to_memory_queue_with_parsing", [](rust::Box<TopologyController> &tc) {
    nlohmann::json event = nlohmann::json::parse(
      R"({"_datatype":"json", "-Target-Es":"main", "name":"湖北", "确诊人数":"67466"})");
    send_http_events({event.dump()});

    auto &memory_queue_client = CxxMemoryQueueClient::get_instance();
    rust::Vec<vectorcxx::CxxLogEvent> events;
    do {
      events = memory_queue_client->poll();
    } while (events.empty());

    REQUIRE(events.size() == 1);
    REQUIRE(std::string(events[0].get_string("_datatype").data(), events[0].get_string("_datatype").size()) == "json");
    REQUIRE(std::string(events[0].get_string("-Target-Es").data(), events[0].get_string("-Target-Es").size()) == "main");
    REQUIRE(std::string(events[0].get_string("name").data(), events[0].get_string("name").size()) == "湖北");
    REQUIRE(std::string(events[0].get_string("确诊人数").data(), events[0].get_string("确诊人数").size()) == "67466");
  });
}

TEST_CASE("consume events with nested field name") {
  run("http_to_memory_queue_with_parsing", [](rust::Box<TopologyController> &tc) {
    nlohmann::json event = nlohmann::json::parse(
      R"({"_datatype":"json", "-Target-Es":"main", "name":"湖北", "确诊人数":"67466", "tags.mention.you":"yes"})");
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
    REQUIRE_THAT(field_vec, VectorContains(std::string("tags.mention.you")));
    REQUIRE_THAT(field_vec, VectorContains(std::string("_datatype")));
    REQUIRE_THAT(field_vec, VectorContains(std::string("-Target-Es")));
    REQUIRE_THAT(field_vec, VectorContains(std::string("确诊人数")));

    REQUIRE(events.size() == 1);
    REQUIRE(std::string(events[0].get_string("_datatype").data(), events[0].get_string("_datatype").size()) == "json");
    REQUIRE(std::string(events[0].get_string("-Target-Es").data(), events[0].get_string("-Target-Es").size()) == "main");
    REQUIRE(std::string(events[0].get_string("name").data(), events[0].get_string("name").size()) == "湖北");
    REQUIRE(std::string(events[0].get_string("确诊人数").data(), events[0].get_string("确诊人数").size()) == "67466");
    REQUIRE(std::string(events[0].get_string("tags.mention.you").data(), events[0].get_string("tags.mention.you").size()) == "yes");
  });
}

TEST_CASE("consume events with nested json") {
  run("http_to_memory_queue_with_parsing", [](rust::Box<TopologyController> &tc) {
    nlohmann::json event = {
        {"X-NILE-PARSED", 1},
        {"_datatype", "json"},
        {"tags.hostname[]", {"v86-6-9-96", "v12-3-10-14"}},
        {"_message", "{\"tags\": {\"hostname\": [\"v86-6-9-96\", \"v12-3-10-14\"]}}"}};
    send_http_events({event.dump()});

    auto &memory_queue_client = CxxMemoryQueueClient::get_instance();
    rust::Vec<vectorcxx::CxxLogEvent> events;
    do {
      events = memory_queue_client->poll();
    } while (events.empty());

    REQUIRE(events.size() == 1);
    REQUIRE(std::string(events[0].get_string("_datatype").data(),
                        events[0].get_string("_datatype").size()) == "json");
    REQUIRE(std::string(events[0].get_string("-Target-Es").data(),
                        events[0].get_string("-Target-Es").size()) == "table_a");
    REQUIRE(std::string(events[0].get_array_as_string("tags.hostname[]").data(),
                        events[0].get_array_as_string("tags.hostname[]").size()) ==
            "[\"v86-6-9-96\",\"v12-3-10-14\"]");
    REQUIRE(std::string(events[0].get_string("_message").data(),
                        events[0].get_string("_message").size()) ==
            "{\"tags\": {\"hostname\": [\"v86-6-9-96\", \"v12-3-10-14\"]}}");
  });
}

TEST_CASE("test fields and top level fields") {
  run("http_to_memory_queue_with_parsing", [](rust::Box<TopologyController> &tc) {
    nlohmann::json event = {
        {"_datatype", "json"},
        {"tags.hostname[]", {"v86-6-9-96", "v12-3-10-14"}},
        {"tags.date.today", "1999-09-01"},
        {"tags.hostname_complex[]",
         {"v86-6-9-96", "v12-3-10-14", {"inner_host_1", "inner_host_2"}}},
        {"hardware", {{"cpu", "8-core"}, {"memory", "32G"}}},
        {"_message", "{\"tags\": {\"hostname\": [\"v86-6-9-96\", \"v12-3-10-14\"]}}"}};
    send_http_events({event.dump()});

    auto &memory_queue_client = CxxMemoryQueueClient::get_instance();
    rust::Vec<vectorcxx::CxxLogEvent> events;
    do {
      events = memory_queue_client->poll();
    } while (events.empty());

    {
      auto fields = events[0].fields();
      REQUIRE(fields.size() > 0);
      std::vector<std::string> field_vec;
      field_vec.reserve(fields.size());
      for (auto const &field : fields) {
        field_vec.emplace_back(field);
      }

      REQUIRE_THAT(field_vec, VectorContains(std::string("tags.date.today")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("tags.hostname[][0]")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("tags.hostname[][1]")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("tags.hostname_complex[][0]")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("tags.hostname_complex[][1]")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("tags.hostname_complex[][2][0]")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("tags.hostname_complex[][2][1]")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("hardware.cpu")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("hardware.memory")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("_datatype")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("_message")));
    }

    {
      auto fields = events[0].top_level_fields();
      REQUIRE(fields.size() > 0);
      std::vector<std::string> field_vec;
      field_vec.reserve(fields.size());
      for (auto const &field : fields) {
        field_vec.emplace_back(field);
      }

      REQUIRE_THAT(field_vec, VectorContains(std::string("tags.date.today")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("tags.hostname[]")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("tags.hostname_complex[]")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("hardware")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("_datatype")));
      REQUIRE_THAT(field_vec, VectorContains(std::string("_message")));
    }
  });
}