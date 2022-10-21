#pragma once

#include "vectorcxx/src/lib.rs.h"
#include <filesystem>

namespace vectorcxx::test {
  // this is the default file sink used by all testing configs
  inline auto FILE_SINK_PATH = std::filesystem::path("/tmp/vector_test_sink.log");

  std::vector<std::string>
  read_events_from_sink(const std::filesystem::path &file_path = FILE_SINK_PATH);

  void send_http_events(const std::vector<std::string> &events, uint32_t port = 9999);

  std::string load_config(const std::string &file_name);

  void wait(uint32_t milliseconds = 200);

  /**
   * run a topology with some operations performed during running
   * After running, the vector service will be stopped so that all the events are flushed and can be
   * asserted later
   */
  void run(const std::string &config_file,
           const std::function<void(rust::Box<vectorcxx::TopologyController> &)> &operations);

  void setup_data();
} // namespace