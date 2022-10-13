mod topology;

use tokio_test::block_on;
use std::panic;
use std::sync::{Arc, Mutex};
use crate::ffi::{ExportResult, KafkaSinkParams, SwEvent, SwEvents};
use std::sync::Once;
use std::fmt::{Debug, Display};

// use futures::executor::block_on;
use tracing::{debug, info, error, Level};
use time::macros::format_description;

use vector::{
    config::Config, config::format,
    sinks::console::{ConsoleSinkConfig, Encoding, Target},
    sources::file::FileConfig, sources::demo_logs::DemoLogsConfig, sources::http::SimpleHttpConfig,
    sources::util, transforms::remap::RemapConfig, kafka::KafkaCompression,
    sinks::kafka::config::KafkaSinkConfig, sinks::file::{FileSinkConfig, Compression},
    sinks::util::encoding::{EncodingConfig, StandardEncodings}, sinks::blackhole::BlackholeConfig,
    sinks::memory_queue::MemoryQueueConfig,
    topology::{GLOBAL_RX, GLOBAL_VEC_RX},
    serde::{default_decoding, default_framing_stream_based}, config};

use vector::test_util::runtime;
use vector::config::SinkConfig;
use vector::config::ConfigBuilder;
use vector::config::ComponentKey;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use std::net::{Ipv4Addr, SocketAddr};
use cxx::{CxxString, SharedPtr};
use vector::topology::RunningTopology;
use std::sync::atomic::{Ordering, AtomicU32};
use serde_json;

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
        fn start_topology(file_path: String, data_dir: String) -> bool;
        fn export_to_kafka(task_id: String, file_path: String, data_dir: String, sink_config: KafkaSinkParams) -> ExportResult;
        fn export_to_file(task_id: String, file_path: String, data_dir: String, sink_config: FileSinkParams) -> ExportResult;
        fn start_ingest_to_vector(config: String) -> ExportResult;
        fn crud_vector_config(action: String, ids: Vec<String>, config_str: String, stage_id: u32) -> bool;
        fn poll_vector_events() -> SwEvents;
        fn get_stage_id() -> u32;

        fn add_config(config: String) -> bool;

        fn update_config(config: String) -> bool;

        fn delete_config(ids: Vec<String>) -> bool;

        fn exit() -> bool;
    }
}

#[derive(Debug)]
enum ConfigAction {
    INIT,
    ADD,
    UPDATE,
    DELETE,
    EXIT,
}

impl std::fmt::Display for ConfigAction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

pub struct ConfigEvent {
    action: ConfigAction,
    config_ids: Vec<String>,
    config_str: String,
    stage_id: u32,
}

pub fn setup_logging() {
    let timer = tracing_subscriber::fmt::time::time();
    let collector = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        // disable color to make CLion happy
        .with_ansi(false)
        .with_thread_ids(true)
        .with_timer(timer)
        .finish();
    tracing::subscriber::set_global_default(collector).expect("setting default subscriber failed");
}

pub static mut GLOBAL_CONFIG_TX: Option<&mut tokio::sync::mpsc::Sender<ConfigEvent>> = None;
pub static mut GLOBAL_CONFIG_RX: Option<&mut tokio::sync::mpsc::Receiver<ConfigEvent>> = None;
static GLOBAL_STAGE_ID: AtomicU32 = AtomicU32::new(0);

pub fn start_topology(file_path: String, data_dir: String) -> bool {
    info!("starting topology");
    let result = panic::catch_unwind(|| {
        block_on(topology::start(file_path, data_dir))
    });
    return !result.is_err();
}

pub fn export_to_kafka(task_id: String, file_path: String, data_dir: String, sink_config: KafkaSinkParams) -> ExportResult {
    let global_buffer = Arc::new(Mutex::new(String::new()));
    let old_hook = panic::take_hook();

    panic::set_hook({
        let global_buffer = global_buffer.clone();
        Box::new(move |info| {
            let mut global_buffer = global_buffer.lock().unwrap();

            if let Some(s) = info.payload().downcast_ref::<&str>() {
                let panic_msg = format!("{} in file {}, line {}", s, info.location().unwrap().file(), info.location().unwrap().line());
                global_buffer.push_str(panic_msg.as_str());
            }
        })
    });

    let result = panic::catch_unwind(|| {
        let (succeed, err_msg) = block_on(
            topology::export_json_result_to_kafka(task_id, file_path, data_dir, sink_config.url, sink_config.topic)
        );
        if succeed {
            return ExportResult { succeed: true, err_msg: "".to_string() };
        }
        return ExportResult { succeed: false, err_msg };
    });

    panic::set_hook(old_hook);

    match result {
        Ok(res) => {
            ExportResult { succeed: res.succeed, err_msg: res.err_msg }
        }
        Err(err) => {
            ExportResult { succeed: false, err_msg: format!("panic in export_to_kafka: {:?}", global_buffer.lock().unwrap()) }
        }
    }
}

pub fn export_to_file(task_id: String, file_path: String, data_dir: String, sink_config: ffi::FileSinkParams) -> ExportResult {
    let global_buffer = Arc::new(Mutex::new(String::new()));
    let old_hook = panic::take_hook();

    panic::set_hook({
        let global_buffer = global_buffer.clone();
        Box::new(move |info| {
            let mut global_buffer = global_buffer.lock().unwrap();

            if let Some(s) = info.payload().downcast_ref::<&str>() {
                let panic_msg = format!("{} in file {}, line {}", s, info.location().unwrap().file(), info.location().unwrap().line());
                global_buffer.push_str(panic_msg.as_str());
            }
        })
    });

    let result = panic::catch_unwind(|| {
        let (succeed, err_msg) = block_on(
            topology::export_json_result_to_file(task_id, file_path, data_dir, sink_config.target_path)
        );
        if succeed {
            return ExportResult { succeed: true, err_msg: "".to_string() };
        }
        return ExportResult { succeed: false, err_msg };
    });

    panic::set_hook(old_hook);

    match result {
        Ok(res) => {
            ExportResult { succeed: res.succeed, err_msg: res.err_msg }
        }
        Err(err) => {
            ExportResult { succeed: false, err_msg: format!("panic in export_to_file: {:?}", global_buffer.lock().unwrap()) }
        }
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

    let (topology, _crash) = vector::test_util::start_topology(config.build().unwrap(), false).await;

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

                println!("receiving events, cnt {:?}", value.len());
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

fn get_stage_id() -> u32 {
    GLOBAL_STAGE_ID.load(Ordering::Relaxed)
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
                let panic_msg = format!("{} in file {}, line {}", s, info.location().unwrap().file(), info.location().unwrap().line());
                global_buffer.push_str(panic_msg.as_str());
            }
        })
    });

    let result = panic::catch_unwind(|| {
        unsafe {
            // let (succeed, err_msg) = block_on(
            //     start_vector_service(config)
            // );
            let (succeed, err_msg) = start_vector_service(config);
            if succeed {
                return ExportResult { succeed: true, err_msg: "".to_string() };
            }
            return ExportResult { succeed: false, err_msg };
        }
    });

    panic::set_hook(old_hook);

    match result {
        Ok(res) => {
            ExportResult { succeed: res.succeed, err_msg: res.err_msg }
        }
        Err(_) => {
            ExportResult { succeed: false, err_msg: format!("panic in start_ingest_to_vector: {:?}", global_buffer.lock().unwrap()) }
        }
    }
}

pub async fn reload_vector(config_event: ConfigEvent, config_builder: &mut ConfigBuilder, topology: &mut RunningTopology) {
    match config_event.action {
        ConfigAction::INIT => {
            // should not go here
        }
        ConfigAction::ADD | ConfigAction::UPDATE => {
            let config_str = &config_event.config_str;
            let new_builder: ConfigBuilder = config::format::deserialize(config_str.as_str(), Some(config::Format::Json)).unwrap();
            let mut config_builder_new = config_builder.clone();
            if new_builder.sources.len() > 0 {
                config_builder_new.sources.extend(new_builder.sources);
            }
            if new_builder.transforms.len() > 0 {
                config_builder_new.transforms.extend(new_builder.transforms);
            }
            info!("ConfigBuilder sources {:?}", config_builder_new.sources);
            info!("ConfigBuilder transforms {:?}", config_builder_new.transforms);
            info!("ConfigBuilder sinks {:?}", config_builder_new.sinks);
            topology.reload_config_and_respawn(config_builder_new.build().unwrap()).await.unwrap();
            GLOBAL_STAGE_ID.store(config_event.stage_id, Ordering::Relaxed);
        }
        ConfigAction::DELETE => {
            // source and transform can not use same name in vector
            let mut config_builder_new = config_builder.clone();

            for id in &config_event.config_ids {
                let key = &ComponentKey::from(&id);
                if config_builder_new.sources.get(key).is_some() {
                    config_builder_new.sources.remove(key);
                } else if config_builder_new.transforms.get(key).is_some() {
                    config_builder_new.transforms.remove(key);
                }
            }
            info!("ConfigBuilder sources {:?}", config_builder_new.sources);
            info!("ConfigBuilder transforms {:?}", config_builder_new.transforms);
            info!("ConfigBuilder sinks {:?}", config_builder_new.sinks);
            topology.reload_config_and_respawn(config_builder_new.build().unwrap()).await.unwrap();
            GLOBAL_STAGE_ID.store(config_event.stage_id, Ordering::Relaxed);
        }
        ConfigAction::EXIT => {
            // should not go here
        }
    }
    // let new_builder: ConfigBuilder = config::format::deserialize(config_str.as_str(), Some(config::Format::Json)).unwrap();
    // let mut config_builder_new = config_builder.clone();
    // if new_builder.sources.len() > 0 {
    //     config_builder_new.sources.extend(new_builder.sources);
    // }
    // if new_builder.transforms.len() > 0 {
    //     config_builder_new.transforms.extend(new_builder.transforms);
    // }
    // let res = topology.reload_config_and_respawn(config_builder_new.build().unwrap()).await.unwrap();
}

static START: Once = Once::new();

pub fn start_vector_service(config_str: String) -> (bool, String) {
    START.call_once(|| {
        setup_logging();
    });

    let (g_config_tx, g_config_rx) = tokio::sync::mpsc::channel(2);
    let g_config_tx_box = Box::new(g_config_tx);
    let g_config_rx_box = Box::new(g_config_rx);
    unsafe {
        GLOBAL_CONFIG_TX = Some(Box::leak(g_config_tx_box));
        GLOBAL_CONFIG_RX = Some(Box::leak(g_config_rx_box));
    }
    info!("start vector service");
    debug!("start vector service with config; config={:?}", config_str);

    let res = config::format::deserialize(config_str.as_str(), Some(config::Format::Json));
    if res.is_err() {
        error!("deserialize error {:?}", res.unwrap_err());
        return (false, "failed to deserialize config string for".to_string());
    }
    let config_builder: ConfigBuilder = res.unwrap();
    debug!("config_builder deserialized; sources={:?} transforms={:?} sinks={:?} global={:?}", config_builder.sources, config_builder.transforms, config_builder.sinks, config_builder.global);

    let mut config_builder_copy = config_builder.clone();
    let builder_for_schema = config_builder.clone();
    vector::config::init_log_schema_from_builder(builder_for_schema, false);

    let mut exit_status: bool = true;
    let mut exit_msg: String = "".to_string();
    let rt = runtime();

    rt.block_on(async move {
        let (mut topology, _crash) = vector::test_util::start_topology(config_builder.build().unwrap(), false).await;
        let mut sources_finished = topology.sources_finished();
        let config_rx = unsafe { &mut GLOBAL_CONFIG_RX };
        info!("This is a multi-core vector service");
        GLOBAL_STAGE_ID.store(1, Ordering::Relaxed);
        loop {
            tokio::select! {
                Some(config_event) = config_rx.as_mut().unwrap().recv() => {
                    info!("receive config event action={:?} config={:?}", config_event.action.to_string().as_str(), config_event.config_str);
                   
                    match config_event.action {
                        ConfigAction::EXIT => {
                            info!("received exit request");
                            GLOBAL_STAGE_ID.store(config_event.stage_id, Ordering::Relaxed);
                            // exit_status = true;
                            // exit_msg = "receive exit request from sw".to_string();
                            break;
                        },
                        _ => {
                            reload_vector(config_event, &mut config_builder_copy, &mut topology).await;
                        }
                    }
                }
                _ = &mut sources_finished => {
                    info!("sources finished");
                    // exit_status = true;
                    // exit_msg = "sources finished".to_string();
                    break;
                },
                else => {
                    info!("should not go here")
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    
        // topology.sources_finished().await;
        topology.stop().await;
    });
    

    (exit_status, exit_msg)
}

fn crud_vector_config(action: String, ids: Vec<String>, config_str: String, stage_id: u32) -> bool {
    let get_action = |action| {
        match action {
            "init" => ConfigAction::INIT,
            "add" => ConfigAction::ADD,
            "update" => ConfigAction::UPDATE,
            "delete" => ConfigAction::DELETE,
            "exit" => ConfigAction::EXIT,
            _ => ConfigAction::INIT,
        }
    };
    info!("crud vector config to action={:?}", action.as_str());
    // let config_tx = unsafe { &mut GLOBAL_CONFIG_TX };
    unsafe {
        if let Some(sender) = &mut GLOBAL_CONFIG_TX {
            debug!("sending config event: action={:?} config={:?}", action.as_str(), config_str);
            sender.try_send(ConfigEvent {
                action: get_action(action.as_str()),
                config_ids: ids,
                config_str,
                stage_id,
            });
        }
    }
    true
}

