#include "vectorcxx/src/lib.rs.h"
#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>
#include <exception>
#include <iostream>
#include <thread>
#include <nlohmann/json.hpp>
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

void consume_events(uint32_t expected) {
    while (expected) {
        auto result = vectorcxx::poll_vector_events();
        if (result.events.empty()) {
            std::this_thread::sleep_for(std::chrono::milliseconds(1000));
        } else {
            auto target_es = result.target;
            std::cout << " target_es: " << target_es << std::endl;
            for (auto ev : result.events) {
                std::cout << ev.message << std::endl;
                std::cout << ev.timestamp << std::endl;
            }
            --expected;
        }
    }
}

// TEST_CASE("test cpp poll with json config change") {
//     nlohmann::json config_json = nlohmann::json::parse(R"(
//     {
//         "sources": {
//             "my_source_id_1": {
//                 "type": "http",
//                 "address": "0.0.0.0:80",
//                 "encoding": "text",
//                 "headers": ["_target_es"]
//             },
//             "my_source_id_2": {
//                 "type": "http",
//                 "address": "0.0.0.0:8000",
//                 "encoding": "text",
//                 "headers": ["_target_es"]
//             }
//         },
//         "sinks": {
//             "my_sink_id": {
//                 "inputs": ["my_source_id*"],
//                 "type": "memory_queue",
//                 "rate": null
//             }
//         }
//     })");

//     std::string config_temp = config_json.dump();
//     auto config = rust::String(config_temp);
//     auto start_thread = std::thread(vectorcxx::start_ingest_to_vector, config);
//     std::cout << "config string: " << config_temp << std::endl;
//     std::this_thread::sleep_for(std::chrono::milliseconds(8000));
//     config_json.at("sources").at("my_source_id_1").at("headers") = {"_target_es", "new_target_es"};
//     config_json.at("sources").at("my_source_id_1").at("address") = "0.0.0.0:8080";

//     auto new_config = rust::String(config_json.dump());
//     //    new_config = vector

//     auto crud_thread = std::thread(vectorcxx::crud_vector_config, "update", "", new_config);
//     auto consume_thread = std::thread(consume_events, 3);

//     crud_thread.join();
//     consume_thread.join();

//     rust::Vec<rust::String> ids {"my_source_id_1"};
//     auto crud_thread_1 = std::thread(vectorcxx::crud_vector_config, "delete", ids, new_config);
//     auto consume_thread_1 = std::thread(consume_events, 3);
//     crud_thread_1.join();
//     consume_thread_1.join();
// }

//TEST_CASE("test cpp poll file source") {
//    nlohmann::json config_json = nlohmann::json::parse(R"(
//    {
//        "sources": {
//            "my_source_id_1": {
//                "type": "file",
//                "include": ["/Users/cj/Playground/vector_test/raw_syslog_output.json"],
//                "read_from": "beginning",
//                "data_dir": "/Users/cj/Playground/vector_test/data_dir",
//                "keep_watching": false
//            }
//        },
//        "transforms": {
//            "add_some_field": {
//                "type": "remap",
//                "inputs": ["my_source_id*"],
//                "source": "._target_es = \"mock_file_es\""
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
//    std::this_thread::sleep_for(std::chrono::milliseconds(3000));
//
//    auto consume_thread = std::thread(consume_events, 1);
//
//    consume_thread.join();
//}

TEST_CASE("test cpp poll with http json array") {
    // nlohmann::json config_json = nlohmann::json::parse(R"(
    // {
    //     "sources": {
    //         "my_source_id_1": {
    //             "type": "http",
    //             "address": "0.0.0.0:80",
    //             "encoding": "json",
    //             "headers": ["-Target-Es"]
    //         }
    //     },
    //     "transforms": {
    //         "add_some_field": {
    //             "type": "remap",
    //             "inputs": ["my_source_id*"],
    //             "source": ". = unnest!(.sw_events)\n"
    //         }
    //     },
    //     "sinks": {
    //         "my_sink_id": {
    //             "inputs": ["add_some_*"],
    //             "type": "memory_queue",
    //             "rate": null
    //         }
    //     }
    // })");

    auto config_json = nlohmann::json::parse(R"({"sinks":{"sw_default_sink":{"inputs":["sw_transform_*"],"rate":null,"type":"memory_queue"}},"sources":{"sw_source_df_sw_vector":{"address":"0.0.0.0:8088","encoding":"json","headers":["-Target-Es"],"type":"http"}},"transforms":{"sw_transform_df_sw_vector":{"inputs":["sw_source_df_sw_vector"],"source":". = unnest!(.sw_events)\n","type":"remap"}}})");

    std::string config_temp = config_json.dump();
    auto config = rust::String(config_temp);
    auto start_thread = std::thread(vectorcxx::start_ingest_to_vector, config);
    std::cout << "config string: " << config_temp << std::endl;

    nlohmann::json new_config_json = nlohmann::json::parse(R"(
    {
        "sources": {
            "my_source_id_2": {
                "type": "http",
                "address": "0.0.0.0:8088",
                "encoding": "text"
            }
        },
        "transforms": {
            "add_some_field_2": {
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
    vectorcxx::crud_vector_config("add", ids, new_config);

    auto consume_thread = std::thread(consume_events, 2);
    consume_thread.join();

    ids.push_back("my_source_id_2");
    ids.push_back("add_some_field_2");
    vectorcxx::crud_vector_config("delete", ids, rust::String());
    auto new_consume_thread = std::thread(consume_events, 1);
    new_consume_thread.join();

    rust::String config_str;
    vectorcxx::crud_vector_config("exit", ids, config_str);
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


