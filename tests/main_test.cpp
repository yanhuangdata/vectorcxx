#include "vectorcxx/src/lib.rs.h"
#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <exception>
#include <iostream>
#include <thread>
#include <nlohmann/json.hpp>
#include <time.h>
//#include "readerwriterqueue/readerwritercircularbuffer.h"

//TEST_CASE("vectorcxx api test") {
//    try {
//        struct vectorcxx::KafkaSinkParams kafka_params {
//            "host.docker.internal:9092", "quickstart-events-1", "", ""
//        };
//        auto result = vectorcxx::export_to_kafka("00000", "/tmp/data", "/tmp/data_dir", kafka_params);
//        auto err_msg = result.err_msg.c_str();
//        std::cout << "result: " << result.succeed << ", " << err_msg << std::endl;
//
//        struct vectorcxx::FileSinkParams file_params {"/tmp/data/result_file.log"};
//        auto new_result = vectorcxx::export_to_file("00000", "/tmp/data", "/tmp/data_dir", file_params);
//        auto new_err_msg = new_result.err_msg.c_str();
//        std::cout << "new result: " << new_result.succeed << ", " << new_err_msg << std::endl;
//    } catch (std::exception& e) {
//        std::cout << "exception: " << e.what() << std::endl;
//    }
//}

//TEST_CASE("test cpp callback") {
//    auto result = vectorcxx::start_sw_sink_vec_test();
//    auto err_msg = result.err_msg.c_str();
//    std::cout << "result: " << result.succeed << ", " << err_msg << std::endl;
//}

//TEST_CASE("test cpp poll with json config") {
//    nlohmann::json config_json = nlohmann::json::parse(R"(
//    {
//        "sources": {
//            "my_source_id": {
//                "type": "http",
//                "address": "0.0.0.0:80",
//                "encoding": "text"
//            }
//        },
//        "transforms": {
//            "add_some_field": {
//                "type": "remap",
//                "inputs": ["my_source_id"],
//                "source": "._target_es = \"mock_es\""
//            }
//        },
//        "sinks": {
//            "my_sink_id": {
//                "inputs": ["add_some_field"],
//                "type": "memory_queue",
//                "rate": null
//            }
//        }
//    })");
//
//    std::string config_temp = config_json.dump();
//    auto config = rust::String(config_temp);
//    auto start_thread = std::thread(vectorcxx::start_ingest_to_vector, config);
//    std::cout << "config string: " << config_temp << std::endl;
//    std::cout << config_json.contains("transforms") << std::endl;
//    std::cout << config_json.at("transforms").contains("add_some_field") << std::endl;
//    std::cout << config_json.at("transforms").at("add_some_field").contains("source") << std::endl;
//    std::cout << "transfroms config: " << config_json.at("transforms").at("add_some_field").at("source").dump() << std::endl;
//    std::this_thread::sleep_for(std::chrono::milliseconds(8000));
//    config_json.at("transforms").at("add_some_field").at("source") = "._target_es = \"new_mock_es\"";
//    auto new_config = rust::String(config_json.dump());
//
//    auto another_thread = std::thread(vectorcxx::crud_vector_config, new_config);
//    while (true) {
//        auto result = vectorcxx::poll_vector_events();
//        if (result.events.empty()) {
//            std::this_thread::sleep_for(std::chrono::milliseconds(1000));
////            std::cout << "no event now" << std::endl;
//        } else {
//            auto target_es = result.target;
////            std::cout << " target_es: " << nlohmann::json::parse(target_es) << std::endl;
//            std::cout << " target_es: " << target_es << std::endl;
//            for (auto ev : result.events) {
//                std::cout << ev.message << std::endl;
//                std::cout << ev.timestamp << std::endl;
//            }
//        }
//    }
////    auto err_msg = result.err_msg.c_str();
////    std::cout << "result: " << result.succeed << ", " << err_msg << std::endl;
//}

// void consume_events(uint32_t expected) {
//     while (expected) {
//         auto result = vectorcxx::poll_vector_events();
//         if (result.events.empty()) {
//             std::this_thread::sleep_for(std::chrono::milliseconds(1000));
//         } else {
//             // auto target_es = result.target;
//             // std::cout << " target_es: " << target_es << ", parsed: " << result.parsed  << std::endl;
//             for (auto& ev : result.events) {
//                 // std::cout << ev.message << std::endl;
//                 // std::cout << ev.timestamp << std::endl;
//             }
//             --expected;
//         }
//     }
// }

void consume_and_verify_events(uint32_t expected) {
    uint32_t cnt_a = 0;
    uint32_t cnt_b = 0;
    uint32_t cnt_wrong = 0;
    uint32_t total = 0;
    while (1) {
        auto result = vectorcxx::poll_vector_events();
        if (result.events.empty()) {
            std::this_thread::sleep_for(std::chrono::milliseconds(50));
        } else {
            auto const &target_es = result.target;
            std::cout << " target_es: " << std::string_view(target_es.data(), target_es.size()) << ", parsed: "
                      << result.parsed << std::endl;
            for (auto const &ev: result.events) {
                std::cout << std::string_view(ev.message.data(), ev.message.size()) << std::endl;
            }
            time_t end_t = time(NULL);
            std::cout << "time: " << ctime(&start_t)<< "polled events, size: " << result.events.size() << ", " << total << ", " << expected << std::endl;

            // std::cout << "time: " << ctime(&end_t) << ", expected: " << expected << std::endl;
            
        }
    }
    std::cout << "cnt_a:  " << cnt_a << ", cnt_b:  " << cnt_b << ", cnt_wrong:  " << 0 << std::endl;
}

TEST_CASE("test cpp poll with http json array") {

    nlohmann::json config_json = nlohmann::json::parse(R"({
    "sinks": {
        "sw_default_sink": {
            "inputs": [
                "sw_final_transform"
            ],
            "rate": null,
            "type": "memory_queue"
        }
    },
    "sources": {
        "sw_source_df_sw_vector": {
            "address": "0.0.0.0:9099",
            "encoding": "json",
            "headers": [
                "-Target-Es",
                "X-NILE-PARSED"
            ],
            "type": "http"
        },
        "sw_source_vector_test": {
            "address": "0.0.0.0:9999",
            "type": "vector",
            "version": "1"
        }
    },
    "transforms": {
        "sw_transform_df_sw_vector": {
            "inputs": [
                "sw_source_df_sw_vector"
            ],
            "source": "",
            "type": "remap"
        },
        "sw_transform_vector_test": {
            "inputs": [
                "sw_source_vector_test"
            ],
            "source": "._target_es = \"vector_es\"",
            "type": "remap"
        },
        "sw_final_transform": {
            "inputs": [
                "sw_transform_*"
            ],
            "source": "if (._datatype == \"nginx__access_log\") {\nstructured=parse_regex!(.message, r'^(?P<remote>[^ ]*) (?P<host>[^ ]*) (?P<user>[^ ]*) \\[(?P<time>[^\\]]*)\\] \"(?P<method>\\S+)(?: +(?P<path>[^\"]*?)(?: +\\S*)?)?\" (?P<code>[^ ]*) (?P<size>[^ ]*)(?: \"(?P<referer>[^\"]*)\" \"(?P<agent>[^\"]*)\"(?:\\s+\"?(?P<http_x_forwarded_for>[^\"]*)\"?)?)?$')\n.=merge(.,structured) \n} else if (._datatype == \"json\") {\nstructured=parse_json!(.message)\n.=merge!(.,structured) \n}\n",
            "type": "remap"
        }
    }
})");

    // auto config_json = nlohmann::json::parse(R"({"sinks":{"sw_default_sink":{"inputs":["sw_transform_*"],"rate":null,"type":"memory_queue"}},"sources":{"sw_source_df_sw_vector":{"address":"0.0.0.0:8088","encoding":"json","headers":["-Target-Es"],"type":"http"}},"transforms":{"sw_transform_df_sw_vector":{"inputs":["sw_source_df_sw_vector"],"source":". = unnest!(.sw_events)\n","type":"remap"}}})");

    std::string config_temp = config_json.dump();
    auto config = rust::String(config_temp);
    auto start_thread = std::thread(vectorcxx::start_ingest_to_vector, config);
    std::cout << "config string: " << config_temp << std::endl;

    nlohmann::json new_config_json = nlohmann::json::parse(R"(
    {
        "sources": {
            "my_source_id_2": {
                "type": "http",
                "address": "0.0.0.0:8089",
                "encoding": "json"
            }
        },
        "transforms": {
            "sw_transform_2": {
                "type": "remap",
                "inputs": ["my_source_id_2"],
                "source": ". = unnest!(.sw_events)\n"
            }
        }
    })");

    std::cout << "new config string: " << new_config_json << std::endl;

    rust::Vec<rust::String> ids;
    auto new_config = rust::String(new_config_json.dump());
    std::this_thread::sleep_for(std::chrono::milliseconds(1000));
    vectorcxx::crud_vector_config("add", ids, new_config, 2);
    consume_and_verify_events(2);
    // // auto consume_thread = std::thread(consume_events, 2);
    // // consume_thread.join();

    // ids.push_back("my_source_id_2");
    // ids.push_back("add_some_field_2");
    // vectorcxx::crud_vector_config("delete", ids, rust::String(), 3);
    // consume_events(1);
    // // auto new_consume_thread = std::thread(consume_events, 1);
    // // new_consume_thread.join();

    // rust::String config_str;
    // vectorcxx::crud_vector_config("exit", ids, config_str, 4);
    start_thread.join();
}

// TEST_CASE("test cpp poll with kafka json array") {
//     nlohmann::json config_json = nlohmann::json::parse(R"(
//     {
//         "sources": {
//             "my_source_id": {
//                 "type": "kafka",
//                 "bootstrap_servers": "host.docker.internal:9092",
//                 "group_id": "consumer_from_vector",
//                 "key_field": "message",
//                 "topics": [
//                     "brian-test"
//                 ]
//             }
//         },
//         "transforms": {
//             "add_some_field": {
//                 "type": "remap",
//                 "inputs": ["my_source_id"],
//                 "source": "._target_es = \"_internal\"\n"
//             }
//         },
//         "sinks": {
//             "my_sink_id": {
//                 "inputs": ["add_some_field"],
//                 "type": "memory_queue",
//                 "rate": null
//             }
//         }
//     })");

//     std::string config_temp = config_json.dump();
//     auto config = rust::String(config_temp);
//     auto start_thread = std::thread(vectorcxx::start_ingest_to_vector, config);
//     std::cout << "config string: " << config_temp << std::endl;

//     auto consume_thread = std::thread(consume_events, 3);
//     consume_thread.join();
// }

