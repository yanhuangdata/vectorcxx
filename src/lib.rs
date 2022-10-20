#![feature(extern_types)]

mod config_event;
mod topology_controller;
mod model;

use tracing::info;
use vector::event::EventContainer;
use vector::topology::GLOBAL_VEC_RX;
use crate::topology_controller::TopologyController;
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

        fn poll_vector_events() -> Vec<CxxLogEvent>;

        unsafe fn get<'a>(self: &'a CxxLogEvent, key: &str) -> &'a str;

        // return a field as timestamp with microsecond precision
        fn get_timestamp(self: &CxxLogEvent, key: &str) -> i64;

        fn fields(self: &CxxLogEvent) -> Vec<String>;
    }
}

pub fn poll_vector_events() -> Vec<CxxLogEvent> {
    let out_rx = unsafe { &mut GLOBAL_VEC_RX };
    let mut events_list: Vec<CxxLogEvent> = Vec::new();

    if let Some(rx) = out_rx {
        match rx.try_next() {
            Ok(Some(value)) => {
                // info!("\nevent is {:?}", value[0].as_log());
                // if let Some(target) = value[0].as_log().get("-Target-Es") {
                //     target_es = target.to_string_lossy();
                // } else if let Some(target) = value[0].as_log().get("_target_es") {
                //     target_es = target.to_string_lossy();
                // }
                info!("event array received length={:?}", value.len());
                events_list.reserve(value.len());
                value.iter_events().for_each(|event_ref|
                    events_list.push(CxxLogEvent { log_event: event_ref.into_log() })
                );
            }
            Ok(None) => {
                // std::thread::sleep(ten_millis);
                // info!("polling channel closed");
            }
            Err(_) => {
                // std::thread::sleep(ten_millis);
                // info!("polling vector events error: {:?}\n", e);
            }
        }
    }
    events_list
}

pub fn new_topology_controller() -> Box<TopologyController> {
    Box::new(TopologyController::new())
}
