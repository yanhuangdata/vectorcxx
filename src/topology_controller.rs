use crate::config_event::{ConfigAction, ConfigEvent};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Once;
use std::sync::Arc;
use tracing::{debug, error, info, Level};
use vector::config::ConfigBuilder;
use vector::config::ComponentKey;
use vector::topology::RunningTopology;
use vector::{config, config::format, metrics, test_util::runtime};

pub struct TopologyController {
    config_message_sender: Option<tokio::sync::mpsc::Sender<ConfigEvent>>,
    vector_thread_join_handle: Option<std::thread::JoinHandle<()>>,
    generation_id: Arc<AtomicU32>,
}

static START: Once = Once::new();
static INIT: Once = Once::new();

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

fn increment_generation_id(generation_id: &AtomicU32) {
    generation_id.fetch_add(1, Ordering::Relaxed);
}

fn _print_ids(config: &mut ConfigBuilder) {
    let mut source_ids = Vec::new();
    let mut transform_ids = Vec::new();
    let mut sink_ids = Vec::new();
    for (key, _) in &config.sources {
        source_ids.push(key.id());
    }
    for (key, _) in &config.transforms {
        transform_ids.push(key.id());
    }
    for (key, _) in &config.sinks {
        sink_ids.push(key.id());
    }
    info!("source ids: {:?}", source_ids);
    info!("transform ids: {:?}", transform_ids);
    info!("sink ids: {:?}", sink_ids);
}

async fn _handle_reload(new: ConfigBuilder, old: &mut ConfigBuilder, topology: &mut RunningTopology) -> bool {
    let new_copy = new.clone();
    match topology
        .reload_config_and_respawn(new.build().unwrap())
        .await
    {
        Ok(true) => {
            info!("vector config reloaded succeed");
            *old = new_copy;
            _print_ids(old);
        },
        Ok(false) => {
            info!("vector config reloaded does not succeed");
            return false;
        },
        Err(()) => {
            error!("error happen while reloading config");
            // TODO: handle error here
            return false;
        }
    }
    true
}

async fn reload_vector(
    config_event: ConfigEvent,
    config_builder: &mut ConfigBuilder,
    topology: &mut RunningTopology,
) {
    debug!("sources before {:?}: {:?}", config_event.action, config_builder.sources);
    debug!("transforms before {:?}: {:?}", config_event.action, config_builder.transforms);
    debug!("sinks before {:?}: {:?}", config_event.action,  config_builder.sinks);

    match config_event.action {
        ConfigAction::INIT => {
            // should not go here
        }
        ConfigAction::ADD | ConfigAction::UPDATE => {
            let config_str = &config_event.config_str;
            let new_builder: ConfigBuilder =
                config::format::deserialize(config_str.as_str(), config::Format::Json)
                    .unwrap();
            let mut config_builder_new = config_builder.clone();
            if new_builder.sources.len() > 0 {
                config_builder_new.sources.extend(new_builder.sources);
            }
            if new_builder.transforms.len() > 0 {
                config_builder_new.transforms.extend(new_builder.transforms);
            }
            debug!("sources after {:?}: {:?}", config_event.action, config_builder_new.sources);
            debug!("transforms after {:?}: {:?}", config_event.action, config_builder_new.transforms);
            debug!("sinks after {:?}: {:?}", config_event.action, config_builder_new.sinks);
            if !_handle_reload(config_builder_new, config_builder, topology).await {
                // TODO: handle error here
            }
        }
        ConfigAction::DELETE => {
            // source and transform can not use same name in vector
            let mut config_builder_new = config_builder.clone();

            for id in &config_event.config_ids {
                let key = &ComponentKey::from(id.clone());
                if config_builder_new.sources.get(key).is_some() {
                    config_builder_new.sources.remove(key);
                } else if config_builder_new.transforms.get(key).is_some() {
                    config_builder_new.transforms.remove(key);
                }
            }
            debug!("sources after {:?}: {:?}", config_event.action, config_builder_new.sources);
            debug!("transforms after {:?}: {:?}", config_event.action, config_builder_new.transforms);
            debug!("sinks after {:?}: {:?}", config_event.action, config_builder_new.sinks);
            if !_handle_reload(config_builder_new, config_builder, topology).await {
                // TODO: handle error here
            }
        }
        ConfigAction::EXIT => {
            // should not go here
        }
    }
}

impl TopologyController {
    pub fn new() -> Self {
        Self {
            config_message_sender: None,
            vector_thread_join_handle: None,
            generation_id: Arc::new(AtomicU32::new(0)),
        }
    }

    // start a new thread to run a topology
    pub fn start(&mut self, topology_config: &str) -> Result<bool, &'static str> {
        START.call_once(|| {
            setup_logging();
        });

        info!("start vector service");

        let res = format::deserialize(topology_config, config::Format::Json);
        if res.is_err() {
            error!("deserialize error {:?}", res.unwrap_err());
            return Err("failed to deserialize config string");
        }
        let config_builder: ConfigBuilder = res.unwrap();
        debug!(
            "config_builder deserialized; sources={:?} transforms={:?} sinks={:?} global={:?}",
            config_builder.sources,
            config_builder.transforms,
            config_builder.sinks,
            config_builder.global
        );
        INIT.call_once(|| {
            let builder_for_schema = config_builder.clone();
            let _init_result = config::init_log_schema_from_builder(builder_for_schema, false).expect("init log schema failed");
            #[cfg(not(feature = "enterprise-tests"))]
            metrics::init_global().expect("metrics initialization failed");
        });

        let mut config_builder_copy = config_builder.clone();

        let (config_message_sender, mut config_message_receiver) = tokio::sync::mpsc::channel(1);
        info!("config update message channel created");
        self.config_message_sender = Some(config_message_sender);

        let shared_generation_id = self.generation_id.clone();
        let config = config_builder.build().unwrap();
        info!("config constructed via config builder");
        let join_handle = std::thread::spawn(|| {
            let rt = runtime();

            rt.block_on(async move {
                let (mut topology, _crash) =
                    vector::test_util::start_topology(config, false).await;
                info!("vector topology started");
                let mut sources_finished = topology.sources_finished();

                loop {
                    tokio::select! {
                        Some(config_event) = config_message_receiver.recv() => {
                            info!("receive config event action={:?} config={:?}",
                                config_event.action.to_string().as_str(),
                                config_event.config_str);
                            match config_event.action {
                                ConfigAction::EXIT => {
                                    info!("received exit request");
                                    break;
                                },
                                _ => {
                                    info!("about to reload vector");
                                    reload_vector(config_event, &mut config_builder_copy, &mut topology).await;
                                }
                            }
                            increment_generation_id(&shared_generation_id);
                        }
                        _ = &mut sources_finished => {
                            info!("sources finished");
                            break;
                        },
                        else => {
                            info!("should not reach here")
                        }
                    }
                    // FIXME: why sleep 1 second?
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }

                topology.stop().await;
            });
        });
        info!("vector thread spawned");
        self.vector_thread_join_handle = Some(join_handle);
        Ok(true)
    }

    pub fn add_config(&mut self, config: String) -> bool {
        self.send_config_event("add".to_string(), vec![], config)
    }

    pub fn delete_config(&mut self, config_ids: Vec<String>) -> bool {
        self.send_config_event("delete".to_string(), config_ids, "".to_string())
    }

    pub fn update_config(&mut self, config: String) -> bool {
        self.send_config_event("update".to_string(), vec![], config)
    }

    pub fn exit(&mut self) -> bool {
        self.send_config_event("exit".to_string(), vec![], "".to_string())
    }

    pub fn stop(&mut self) -> bool {
        if self.vector_thread_join_handle.is_some() {
            self.vector_thread_join_handle
                .take()
                .unwrap()
                .join()
                .unwrap();
            return true;
        }
        false
    }

    // a self increment id to indicate which generation of config is currently running
    pub fn get_generation_id(&self) -> u32 {
        self.generation_id.load(Ordering::Relaxed)
    }


    fn send_config_event(&self, action: String, ids: Vec<String>, config_str: String) -> bool {
        let get_action = |action| match action {
            "init" => ConfigAction::INIT,
            "add" => ConfigAction::ADD,
            "update" => ConfigAction::UPDATE,
            "delete" => ConfigAction::DELETE,
            "exit" => ConfigAction::EXIT,
            _ => ConfigAction::INIT,
        };
        info!(
            "about to send vector config event action={:?}",
            action.as_str()
        );
        debug!(
            "sending config event: action={:?} config={:?}",
            action.as_str(),
            config_str
        );
        self.config_message_sender
            .as_ref()
            .unwrap()
            .try_send(ConfigEvent {
                action: get_action(action.as_str()),
                config_ids: ids,
                config_str,
            })
            .unwrap();
        true
    }
}
