#![feature(extern_types)]

mod config_event;
mod topology;
mod topology_controller;

use crate::config_event::{ConfigAction, ConfigEvent};
use crate::ffi::{ExportResult, KafkaSinkParams, SwEvent, SwEvents};
use std::panic;
use std::sync::Once;
use std::sync::{Arc, Mutex};
use tokio_test::block_on;

// use futures::executor::block_on;
use time::macros::format_description;
use tracing::{debug, error, info, Level};

use vector::{
    config,
    config::format,
    config::Config,
    kafka::KafkaCompression,
    serde::{default_decoding, default_framing_stream_based},
    sinks::blackhole::BlackholeConfig,
    sinks::console::{ConsoleSinkConfig, Encoding, Target},
    sinks::file::{Compression, FileSinkConfig},
    sinks::kafka::config::KafkaSinkConfig,
    sinks::memory_queue::MemoryQueueConfig,
    sinks::util::encoding::{EncodingConfig, StandardEncodings},
    sources::demo_logs::DemoLogsConfig,
    sources::file::FileConfig,
    sources::http::SimpleHttpConfig,
    sources::util,
    topology::{GLOBAL_RX, GLOBAL_VEC_RX},
    transforms::remap::RemapConfig,
};

use crate::topology_controller::TopologyController;
use cxx::{CxxString, SharedPtr};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use serde_json;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};
use vector::config::ComponentKey;
use vector::config::ConfigBuilder;
use vector::config::SinkConfig;
use vector::test_util::runtime;
use vector::topology::RunningTopology;

#[cxx::bridge(namespace = "vectorcxx")]
mod ffi {
    pub struct ExportResult {
        pub succeed: bool,
        pub err_msg: String,
    }

    pub struct KafkaSinkParams {
        pub(crate) url: String,
        pub(crate) topic: String,
        username: String,
        password: String,
    }

    pub struct FileSinkParams {
        pub(crate) target_path: String,
    }

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

        fn start_topology(file_path: String, data_dir: String) -> bool;
        fn export_to_kafka(
            task_id: String,
            file_path: String,
            data_dir: String,
            sink_config: KafkaSinkParams,
        ) -> ExportResult;
        fn export_to_file(
            task_id: String,
            file_path: String,
            data_dir: String,
            sink_config: FileSinkParams,
        ) -> ExportResult;
        fn start_ingest_to_vector(config: String) -> ExportResult;
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

pub fn start_topology(file_path: String, data_dir: String) -> bool {
    info!("starting topology");
    let result = panic::catch_unwind(|| block_on(topology::start(file_path, data_dir)));
    return !result.is_err();
}

pub fn export_to_kafka(
    task_id: String,
    file_path: String,
    data_dir: String,
    sink_config: KafkaSinkParams,
) -> ExportResult {
    let global_buffer = Arc::new(Mutex::new(String::new()));
    let old_hook = panic::take_hook();

    panic::set_hook({
        let global_buffer = global_buffer.clone();
        Box::new(move |info| {
            let mut global_buffer = global_buffer.lock().unwrap();

            if let Some(s) = info.payload().downcast_ref::<&str>() {
                let panic_msg = format!(
                    "{} in file {}, line {}",
                    s,
                    info.location().unwrap().file(),
                    info.location().unwrap().line()
                );
                global_buffer.push_str(panic_msg.as_str());
            }
        })
    });

    let result = panic::catch_unwind(|| {
        let (succeed, err_msg) = block_on(topology::export_json_result_to_kafka(
            task_id,
            file_path,
            data_dir,
            sink_config.url,
            sink_config.topic,
        ));
        if succeed {
            return ExportResult {
                succeed: true,
                err_msg: "".to_string(),
            };
        }
        return ExportResult {
            succeed: false,
            err_msg,
        };
    });

    panic::set_hook(old_hook);

    match result {
        Ok(res) => ExportResult {
            succeed: res.succeed,
            err_msg: res.err_msg,
        },
        Err(err) => ExportResult {
            succeed: false,
            err_msg: format!(
                "panic in export_to_kafka: {:?}",
                global_buffer.lock().unwrap()
            ),
        },
    }
}

pub fn export_to_file(
    task_id: String,
    file_path: String,
    data_dir: String,
    sink_config: ffi::FileSinkParams,
) -> ExportResult {
    let global_buffer = Arc::new(Mutex::new(String::new()));
    let old_hook = panic::take_hook();

    panic::set_hook({
        let global_buffer = global_buffer.clone();
        Box::new(move |info| {
            let mut global_buffer = global_buffer.lock().unwrap();

            if let Some(s) = info.payload().downcast_ref::<&str>() {
                let panic_msg = format!(
                    "{} in file {}, line {}",
                    s,
                    info.location().unwrap().file(),
                    info.location().unwrap().line()
                );
                global_buffer.push_str(panic_msg.as_str());
            }
        })
    });

    let result = panic::catch_unwind(|| {
        let (succeed, err_msg) = block_on(topology::export_json_result_to_file(
            task_id,
            file_path,
            data_dir,
            sink_config.target_path,
        ));
        if succeed {
            return ExportResult {
                succeed: true,
                err_msg: "".to_string(),
            };
        }
        return ExportResult {
            succeed: false,
            err_msg,
        };
    });

    panic::set_hook(old_hook);

    match result {
        Ok(res) => ExportResult {
            succeed: res.succeed,
            err_msg: res.err_msg,
        },
        Err(err) => ExportResult {
            succeed: false,
            err_msg: format!(
                "panic in export_to_file: {:?}",
                global_buffer.lock().unwrap()
            ),
        },
    }
}

pub async fn ingest_to_blackhole(f: fn(v: Vec<SwEvent>) -> bool) -> (bool, String) {
    // http start
    let http_config = SimpleHttpConfig {
        address: SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), 80),
        encoding: None,
        headers: Vec::new(),
        query_parameters: Vec::new(),
        tls: None,
        auth: None,
        path_key: "path".to_string(),
        path: "/".to_string(),
        strict_path: true,
        framing: Some(default_framing_stream_based()),
        decoding: Some(default_decoding()),
    };
    // http end

    let source_path: String = "/tmp/data".to_string();
    let data_dir: String = "/tmp/data_dir".to_string();
    let file_config = FileConfig {
        include: vec![std::path::Path::new(&source_path).join("*.json")],
        data_dir: Some(std::path::Path::new(&data_dir).to_path_buf()),
        // keep_watching: false,
        ..Default::default()
    };

    let mut config = Config::builder();
    config.add_source("in1", http_config);

    let sink_config = MemoryQueueConfig {
        rate: None,
        // out_sender: Some(out_tx),
    };
    config.add_sink("cxx_sink", &["in1"], sink_config);

    let (topology, _crash) =
        vector::test_util::start_topology(config.build().unwrap(), false).await;

    let out_rx = unsafe { &mut GLOBAL_VEC_RX };
    let ten_millis = std::time::Duration::from_millis(1000);

    std::thread::spawn(move || loop {
        if let Some(rx) = out_rx {
            match rx.try_next() {
                Ok(Some(value)) => {
                    let mut events_list: Vec<SwEvent> = Vec::new();

                    for event in value {
                        let key = vector::config::log_schema().message_key();
                        let ev = event.as_log().get(key).unwrap().to_string_lossy();
                        // let ts_key = vector::config::log_schema().timestamp_key();
                        // let ts = event.as_log().get(ts_key).unwrap().as_timestamp().unwrap().timestamp_millis();
                        let new_event = ffi::SwEvent {
                            parsed: false,
                            target: "whatever".to_string(),
                            source_type: "".to_string(),
                            message: ev,
                            // timestamp: ts
                        };
                        events_list.push(new_event);
                        // print!("\nsingle value is {:?}, ts is {:?}\n", ev, ts);
                    }

                    f(events_list);
                }
                Ok(None) => {
                    std::thread::sleep(ten_millis);
                    break;
                }
                Err(_) => {
                    std::thread::sleep(ten_millis);
                }
            }
        }
    });

    topology.sources_finished().await;
    topology.stop().await;

    info!("all events are ingested");
    // (flag, msg)
    (true, "DONE".to_string())
}

pub fn poll_vector_events() -> SwEvents {
    let out_rx = unsafe { &mut GLOBAL_VEC_RX };
    let ten_millis = std::time::Duration::from_millis(1000);
    let mut events_list: Vec<SwEvent> = Vec::new();
    // let mut target_es = String::new();
    // let mut parsed: bool = false;
    // let mut source_type = String::new();
    let default_msg_key = vector::config::log_schema().message_key();
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

                for event in &value {
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

                    let mut ev = serde_json::to_string(event.as_log()).unwrap();

                    if let Some(value) = event.as_log().get("X-NILE-PARSED") {
                        if !value.is_empty() {
                            parsed = true;
                        }
                    }

                    // let ts_key = vector::config::log_schema().timestamp_key();
                    // //let ts = event.as_log().get(ts_key).unwrap().as_timestamp().unwrap().timestamp_millis();
                    // let mut ts = 0;

                    // if let Some(value) = event.as_log().get(ts_key) {
                    //     if let Some(timestamp) = value.as_timestamp() {
                    //         ts = timestamp.timestamp_millis();
                    //     }
                    // }
                    let mut target_event_set = String::new();
                    if let Some(target) = event.as_log().get("-Target-Es") {
                        target_event_set = target.to_string_lossy();
                    } else if let Some(target) = event.as_log().get("_target_es") {
                        target_event_set = target.to_string_lossy();
                    }

                    if let Some(value) = event.as_log().get(source_type_key) {
                        source_type = value.to_string_lossy();
                    }

                    let new_event = SwEvent {
                        parsed,
                        target: target_event_set,
                        source_type,
                        message: ev,
                    };
                    events_list.push(new_event);
                }
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

pub fn start_ingest_to_vector(config: String) -> ExportResult {
    // unsafe {
    //     let (succeed, err_msg) = block_on(start_vector_service(config));
    //     return ExportResult {succeed:succeed, err_msg: err_msg};
    // }

    let global_buffer = Arc::new(Mutex::new(String::new()));
    let old_hook = panic::take_hook();

    panic::set_hook({
        let global_buffer = global_buffer.clone();
        Box::new(move |info| {
            let mut global_buffer = global_buffer.lock().unwrap();

            if let Some(s) = info.payload().downcast_ref::<&str>() {
                let panic_msg = format!(
                    "{} in file {}, line {}",
                    s,
                    info.location().unwrap().file(),
                    info.location().unwrap().line()
                );
                global_buffer.push_str(panic_msg.as_str());
            }
        })
    });

    let result = panic::catch_unwind(|| {
        unsafe {
            // let (succeed, err_msg) = block_on(
            //     start_vector_service(config)
            // );
            // let (succeed, err_msg) = start_vector_service(config);
            // FIXME: we need to re-implement the start_vector_service
            let (succeed, err_msg) = (true, "DONE".to_string());
            if succeed {
                return ExportResult {
                    succeed: true,
                    err_msg: "".to_string(),
                };
            }
            return ExportResult {
                succeed: false,
                err_msg,
            };
        }
    });

    panic::set_hook(old_hook);

    match result {
        Ok(res) => ExportResult {
            succeed: res.succeed,
            err_msg: res.err_msg,
        },
        Err(_) => ExportResult {
            succeed: false,
            err_msg: format!(
                "panic in start_ingest_to_vector: {:?}",
                global_buffer.lock().unwrap()
            ),
        },
    }
}

pub fn new_topology_controller() -> Box<TopologyController> {
    Box::new(TopologyController::new())
}
