
#pragma once

#include "vectorcxx_bridge/lib.h"
namespace vectorcxx {
  class CxxMemoryQueueClient {
  public:
    static rust::Box<MemoryQueueClient> &get_instance();
  };
}
