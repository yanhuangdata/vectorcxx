// use tempfile::tempdir;
use vector::{
    config::Config,
    sinks::console::{ConsoleSinkConfig, Encoding, Target},
    sources::file::FileConfig,
    transforms::remap::RemapConfig,
    test_util::{start_topology, start_topology_new},
    kafka::KafkaCompression,
    sinks::kafka::config::KafkaSinkConfig,
    sinks::file::{FileSinkConfig, Compression},
    sinks::util::encoding::{EncodingConfig, StandardEncodings},
};
use vector::config::SinkConfig;

pub async fn start(file_path: String, data_dir: String) {
    let mut old_config = Config::builder();
    // let http_config = SimpleHttpConfig::generate_config();
    let file_config = FileConfig {
        include: vec![std::path::Path::new(&file_path).join("*.log")],
        data_dir: Some(std::path::Path::new(&data_dir).to_path_buf()),
        keep_watching: false,
        ..Default::default()
    };

    let console_sink = ConsoleSinkConfig {
        target: Target::Stdout,
        encoding: Encoding::Text.into(),
    };

    let demo_logs = file_config;

    old_config.add_source("in", demo_logs);
    old_config.add_sink(
        "console_sink_1",
        &["in"],
        console_sink,
    );

    let (topology, _crash) = start_topology(old_config.build().unwrap(), false).await;

    topology.sources_finished().await;
    topology.stop().await;
}

pub async fn export_json_result_to_kafka(task_id: String, file_path: String, data_dir: String, kafka_server: String, kafka_topic: String) -> (bool, String) {
    let kafka_sink_config = KafkaSinkConfig {
        bootstrap_servers: kafka_server,
        topic: kafka_topic,
        key_field: None,
        encoding: EncodingConfig::from(StandardEncodings::Json),
        batch: Default::default(),
        compression: KafkaCompression::None,
        auth: Default::default(),
        socket_timeout_ms: 60000,
        message_timeout_ms: 300000,
        librdkafka_options: Default::default(),
        headers_field: None,
    };

    let result = export_json_result_to_sink(task_id, file_path, data_dir, kafka_sink_config).await;
    return result;
}

pub async fn export_json_result_to_file(task_id: String, file_path: String, data_dir: String, target_file_path: String) -> (bool, String) {
    let config = FileSinkConfig {
        path: target_file_path.try_into().unwrap(),
        idle_timeout_secs: None,
        encoding: EncodingConfig::from(vector::sinks::file::Encoding::Ndjson),
        compression: Compression::None,
    };

    let result = export_json_result_to_sink(task_id, file_path, data_dir, config).await;
    return result;
}

pub async fn export_json_result_to_sink<S: SinkConfig + 'static>(task_id: String, file_path: String, data_dir: String, sink: S) -> (bool, String) {
    // println!("file_path {}, data_dir {}, kafka_server {}, topic {} ", file_path, data_dir, kafka_server, kafka_topic);
    let mut old_config = Config::builder();
    let sink_type = sink.sink_type().to_string();

    let source_config_name = format!("{}_{}_for_{}", task_id, "source", sink_type);
    let remap_config_name = format!("{}_{}", task_id, "remap");
    let sink_config_name = format!("{}_{}_to_{}", task_id, "sink", sink_type);


    let file_config = FileConfig {
        include: vec![std::path::Path::new(&file_path).join("*.json")],
        data_dir: Some(std::path::Path::new(&data_dir).to_path_buf()),
        keep_watching: false,
        ..Default::default()
    };

    let remap_config = RemapConfig {
        source: Some(". = parse_json!(.message) # sets `.` to an array of objects".to_owned()),
        file: None,
        ..Default::default()
    };

    old_config.add_sink(
        sink_config_name,
        &[remap_config_name.as_str()],
        sink,
    );

    old_config.add_transform(
        remap_config_name,
        &[source_config_name.as_str()],
        remap_config,
    );

    old_config.add_source(source_config_name, file_config);
    old_config.healthchecks.enabled = true;
    old_config.healthchecks.set_require_healthy(true);

    start_topology_new(old_config.build().unwrap(), true).await
}