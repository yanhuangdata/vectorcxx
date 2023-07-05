mod config_event;
mod topology_controller;
mod model;
mod memory_queue_client;

use vector::event::LogEvent;
use vector::event::Value;

use crate::topology_controller::TopologyController;
use crate::topology_controller::OneShotTopologyController;
use crate::memory_queue_client::MemoryQueueClient;
use crate::model::CxxLogEvent;
use std::collections::BTreeMap;

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

        fn new_cxx_log_events(inputs: Vec<String>, count: i32) -> Vec<CxxLogEvent>;

        fn get_string(self: &CxxLogEvent, key: &str) -> String;

        unsafe fn get_value_type<'a>(self: &'a CxxLogEvent, key: &str) ->  &'a str;

        fn get_object_as_string(self: &CxxLogEvent, key: &str) -> String;
        
        fn get_array_as_string(self: &CxxLogEvent, key: &str) -> String;

        fn get_string_array(self: &CxxLogEvent, key: &str) -> Vec<String>;
    
        fn get_integer(self: &CxxLogEvent, key: &str) -> i64 ;
    
        fn get_boolean(self: &CxxLogEvent, key: &str) -> bool;
    
        fn get_double(self: &CxxLogEvent, key: &str) -> f64;

        // return a field as timestamp with microsecond precision
        fn get_timestamp(self: &CxxLogEvent, key: &str) -> i64;

        fn fields(self: &CxxLogEvent) -> Vec<String>;

        fn top_level_fields(self: &CxxLogEvent) -> Vec<String>;
    }

    extern "Rust" {
        /**
         * MemoryQueueClient
         */
        type MemoryQueueClient;

        fn new_memory_queue_client() -> Box<MemoryQueueClient>;
        
        fn new_memory_queue_client_with_random_events(
            queue_size: usize, events_count: usize, event_len: usize, batch_size: usize, is_json: bool
        )-> Box<MemoryQueueClient>;

        fn poll(self: &mut MemoryQueueClient) -> Vec<CxxLogEvent>;
    }

    extern "Rust" {
        /**
         * OneShotTopologyController
         */
        type OneShotTopologyController;

        fn new_one_shot_topology_controller() -> Box<OneShotTopologyController>;

        fn start(self: &mut OneShotTopologyController, topology_config: &str) -> Result<bool>;
    }
}

pub fn new_topology_controller() -> Box<TopologyController> {
    Box::new(TopologyController::new())
}

pub fn new_one_shot_topology_controller() -> Box<OneShotTopologyController> {
    Box::new(OneShotTopologyController::new())
}

pub fn new_memory_queue_client() -> Box<MemoryQueueClient> {
    Box::new(MemoryQueueClient::new())
}

// this is an API for generating events to memory queue sink
pub fn new_memory_queue_client_with_random_events(queue_size: usize, events_count: usize, event_len: usize, batch_size: usize, is_json: bool) -> Box<MemoryQueueClient> {
    Box::new(MemoryQueueClient::new_with_random_events(queue_size, events_count, event_len, batch_size, is_json))
}

// this is an API just for generating benchmark testing events
pub fn new_cxx_log_events(inputs: Vec<String>, count: i32) -> Vec<CxxLogEvent> {
    let mut events: Vec<CxxLogEvent> = Vec::new();
    for _i in 0..count {
        let event = CxxLogEvent::new(&inputs);
        events.push(event);
    }
    // test non-utf value parsed(use U+FFFD REPLACEMENT CHARACTER) before constructing LogEvent
    let invalid_kv: Vec<String> = vec!("invalid_key".to_string(), "Hello �World".to_string());
    let new_event = CxxLogEvent::new(&invalid_kv);
    events.push(new_event);

    // test non-utf value not parsed before constructing LogEvent
    let invalid_value = Value::from(b"Hello \xF0\x90\x80World");
    let another_kv = BTreeMap::from([("invalid_key".to_string(), invalid_value)]);
    let another_event = CxxLogEvent {log_event: LogEvent::from(another_kv)};
    events.push(another_event);
    events
}