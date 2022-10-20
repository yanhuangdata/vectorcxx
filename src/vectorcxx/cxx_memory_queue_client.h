
#pragma once

#include "vectorcxx/src/lib.rs.h"
namespace vectorcxx {
  class CxxMemoryQueueClient {
  public:
    static rust::Box<MemoryQueueClient> &get_instance();
  };
}
