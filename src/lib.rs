#![feature(extern_types)]

mod config_event;
mod topology_controller;
mod model;
mod memory_queue_client;

use crate::topology_controller::TopologyController;
use crate::memory_queue_client::MemoryQueueClient;
use crate::model::CxxLogEvent;

#[cxx::bridge(namespace = "vectorcxx")]
mod ffi {
    extern "Rust" {
        /**
         * TopologyController
         */
        type TopologyController;

        fn new_topology_controller() -> Box<TopologyController>;

        fn start(self: &mut TopologyController, topology_config: &str) -> Result<bool>;

        fn add_config(self: &mut TopologyController, config: String) -> bool;

        fn update_config(self: &mut TopologyController, config: String) -> bool;

        fn delete_config(self: &mut TopologyController, config_ids: Vec<String>) -> bool;

        fn exit(self: &mut TopologyController) -> bool;

        fn stop(self: &mut TopologyController) -> bool;

        fn get_generation_id(self: &mut TopologyController) -> u32;
    }

    extern "Rust" {
        /**
         * LogEvent
         */
        type CxxLogEvent;

        unsafe fn get<'a>(self: &'a CxxLogEvent, key: &str) -> &'a str;

        // return a field as timestamp with microsecond precision
        fn get_timestamp(self: &CxxLogEvent, key: &str) -> i64;

        fn fields(self: &CxxLogEvent) -> Vec<String>;
    }

    extern "Rust" {
        /**
         * MemoryQueueClient
         */
        type MemoryQueueClient;

        fn new_memory_queue_client() -> Box<MemoryQueueClient>;

        fn poll(self: &mut MemoryQueueClient) -> Vec<CxxLogEvent>;
    }
}

pub fn new_topology_controller() -> Box<TopologyController> {
    Box::new(TopologyController::new())
}

pub fn new_memory_queue_client() -> Box<MemoryQueueClient> {
    Box::new(MemoryQueueClient::new())
}
