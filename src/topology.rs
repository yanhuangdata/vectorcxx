use tempfile::tempdir;
use vector::{
    config::Config,
    sinks::console::{ConsoleSinkConfig, Encoding, Target},
    sources::file::FileConfig,
    test_util::start_topology,
    kafka::KafkaCompression,
    sinks::kafka::config::KafkaSinkConfig,
    sinks::util::encoding::{EncodingConfig, StandardEncodings},
};

pub async fn start() {
    let mut old_config = Config::builder();
    // let http_config = SimpleHttpConfig::generate_config();
    let global_dir = tempdir().unwrap();
    let file_config = FileConfig {
        include: vec![std::path::Path::new("/tmp").join("data").join("*.txt")],
        data_dir: Some(global_dir.path().to_path_buf()),
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

    topology.sources_finished().await
}