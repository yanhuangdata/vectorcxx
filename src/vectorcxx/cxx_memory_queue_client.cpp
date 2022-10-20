#include "vectorcxx/cxx_memory_queue_client.h"

namespace vectorcxx {
  rust::Box<MemoryQueueClient> &CxxMemoryQueueClient::get_instance() {
    static auto client = vectorcxx::new_memory_queue_client();
    return client;
  }
}
