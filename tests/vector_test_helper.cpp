#include "vector_test_helper.h"
#include <algorithm>
#include <cpr/cpr.h>
#include <fmt/format.h>
#include <iostream>
#include <nlohmann/json.hpp>

#include <spdlog/spdlog.h>

using vectorcxx::CxxLogEvent;

namespace vectorcxx::test {

  inline auto DATA_DIR = std::filesystem::path("/tmp/vector");

  // get a file path under data folder
  std::filesystem::path _file_path(const std::string &file_name) {
    return std::filesystem::path(__FILE__).parent_path() / "data" / file_name;
  }

  std::string load_config(const std::string &file_name) {
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

  void wait(uint32_t milliseconds) {
    std::this_thread::sleep_for(std::chrono::milliseconds(milliseconds));
  }

  std::vector<std::string> read_events_from_sink(const std::filesystem::path &file_path) {
    return _read_file_by_lines(file_path);
  }

  void send_http_events(const std::vector<std::string> &events, uint32_t port) {
    spdlog::info("sending events via http events={}", events.size());
    for (const auto &event : events) {
      auto res = cpr::Post(cpr::Url{fmt::format("http://localhost:{}", port)}, cpr::Body{event},
                           cpr::Header{{"-Target-Es", "table_a"}, {"_target_table", "main"}});
      spdlog::info("1 sending event to http status={}", res.status_code);
    }
  }

  static void _setup() {
    // ensure the file sink is cleared
    std::filesystem::remove(FILE_SINK_PATH);
    std::filesystem::remove_all(DATA_DIR);
    std::filesystem::create_directory(DATA_DIR);
  }

  struct VectorService {
    explicit VectorService(const std::string &config_file) {
      _setup();
      auto config = rust::String(load_config(config_file));
      spdlog::info("starting vector");

      tc = vectorcxx::new_topology_controller();
      tc.value()->start(config);

      // wait for the vector service to start
      spdlog::info("waiting for vector to get started");
      wait();
    }

    ~VectorService() {
      auto &controller = get_controller();
      controller->exit();
      controller->stop();
    }

    rust::Box<vectorcxx::TopologyController> &get_controller() {
      return tc.value();
    }

    std::optional<rust::Box<vectorcxx::TopologyController>> tc;
  };

  struct OneShotVectorService {
    explicit OneShotVectorService(const std::string &config_file) {
      _setup();
      auto config = rust::String(load_config(config_file));
      spdlog::info("starting vector");

      tc = vectorcxx::new_one_shot_topology_controller();
      tc.value()->start(config);
    }

    ~OneShotVectorService() {}

    rust::Box<vectorcxx::OneShotTopologyController> &get_controller() {
      return tc.value();
    }

    std::optional<rust::Box<vectorcxx::OneShotTopologyController>> tc;
  };

  void run(const std::string &config_file,
           const std::function<void(rust::Box<vectorcxx::TopologyController> &)> &operations) {
    VectorService vector_service(config_file);
    operations(vector_service.get_controller());
  }

  void run_one_shot(const std::string &config_file,
           const std::function<void(rust::Box<vectorcxx::OneShotTopologyController> &)> &operations) {
    OneShotVectorService vector_service(config_file);
    operations(vector_service.get_controller());
  }
} // namespace
