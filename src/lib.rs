#![feature(extern_types)]

mod config_event;
mod topology_controller;

use tracing::info;
use vector::event::{EventContainer, EventRef};
use crate::ffi::{SwEvent, SwEvents};
// use futures::executor::block_on;
use vector::topology::GLOBAL_VEC_RX;
use crate::topology_controller::TopologyController;

#[cxx::bridge(namespace = "vectorcxx")]
mod ffi {
    pub struct SwEvent {
        pub parsed: bool,
        pub target: String,
        pub source_type: String,
        pub message: String,
        // pub timestamp: i64,
    }

    pub struct SwEvents {
        // pub target: String,
        pub events: Vec<SwEvent>,
    }
    pub struct SwSharedEvent {
        // pub target: String,
        // pub message: String,
        pub timestamp: i64,
        keys: Vec<String>,
        values: Vec<String>,
    }

    pub struct SwSharedEvents {
        pub target: String,
        pub events: Vec<SwSharedEvent>,
    }

    extern "Rust" {
        type TopologyController;

        fn poll_vector_events() -> SwEvents;

        /**
         * TopologyController
         */
        fn new_topology_controller() -> Box<TopologyController>;

        fn start(self: &mut TopologyController, topology_config: &str) -> Result<bool>;

        fn add_config(self: &mut TopologyController, config: String) -> bool;

        fn update_config(self: &mut TopologyController, config: String) -> bool;

        fn delete_config(self: &mut TopologyController, config_ids: Vec<String>) -> bool;

        fn exit(self: &mut TopologyController) -> bool;

        fn stop(self: &mut TopologyController) -> bool;

        fn get_generation_id(self: &mut TopologyController) -> u32;
    }
}

fn _process_event(event: EventRef, events_list: &mut Vec<SwEvent>, source_type_key: &str) {
    // let mut ev = String::new();
    let mut parsed: bool = false;
    let mut source_type = String::new();
    // for now, no extra json decoding, will add soon
    // if let Some(value) = event.as_log().get("sw_events") {
    //     ev = value.to_string_lossy();
    //     parsed = true;
    // // } else if let Some(value) = event.as_log().get(default_msg_key){
    // //     ev = event.as_log().get(default_msg_key).unwrap().to_string_lossy();
    // // } else if let Some(value) = event.as_log().get("_message"){
    // //     ev = event.as_log().get(default_msg_key).unwrap().to_string_lossy();
    // // } else if source_type == "kafka" {
    // //     ev = event.as_log().get(default_msg_key).unwrap().to_string_lossy();
    // } else {
    //     ev = serde_json::to_string(event.as_log()).unwrap();
    // }

    let ev = serde_json::to_string(event.as_log()).unwrap();

    if let Some(value) = event.as_log().get("X-NILE-PARSED") {
        if !value.is_empty() {
            parsed = true;
        }
    }

    let mut target_table = String::new();
    let log_event = event.as_log();
    if let Some(target) = log_event.get("_target_table") {
        target_table = target.to_string_lossy();
    }

    if let Some(value) = log_event.get(source_type_key) {
        source_type = value.to_string_lossy();
    }

    let new_event = SwEvent {
        parsed,
        target: target_table,
        source_type,
        message: ev,
    };
    events_list.push(new_event);
}

pub fn poll_vector_events() -> SwEvents {
    let out_rx = unsafe { &mut GLOBAL_VEC_RX };
    let mut events_list: Vec<SwEvent> = Vec::new();
    // let mut target_es = String::new();
    // let mut parsed: bool = false;
    // let mut source_type = String::new();
    let _default_msg_key = vector::config::log_schema().message_key();
    let source_type_key = vector::config::log_schema().source_type_key();

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
                value.iter_events().for_each(|event| _process_event(event, &mut events_list, &source_type_key));
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

    SwEvents {
        // parsed,
        // target: target_es,
        events: events_list,
        // source_type,
    }
}

pub fn new_topology_controller() -> Box<TopologyController> {
    Box::new(TopologyController::new())
}
