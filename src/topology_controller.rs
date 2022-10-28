use crate::config_event::{ConfigAction, ConfigEvent};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Once;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tracing::{debug, error, info, Level};
use vector::config::{ConfigBuilder, Config, ComponentKey, ConfigDiff};
use vector::topology::RunningTopology;
use vector::{config, config::format, metrics, test_util::runtime};

pub struct TopologyController {
    vector_thread_join_handle: Option<std::thread::JoinHandle<()>>,
    generation_id: Arc<AtomicU32>,
    topology: Arc<Mutex<Option<RunningTopology>>>,
    config_builder: Arc<Mutex<Option<ConfigBuilder>>>,
    rt: Arc<tokio::runtime::Runtime>,
}

pub struct OneShotTopologyController {
    rt: Arc<tokio::runtime::Runtime>,
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
) -> bool {
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
                return false;
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
                return false;
            }
        }
        ConfigAction::EXIT => {
            // should not go here
        }
    }
    true
}

fn advance_generation(result: bool, generation_id: &AtomicU32) -> bool {
    if result {
        generation_id.fetch_add(1, Ordering::Relaxed);
    }
    result
}

pub fn init_config(config_str: &str) -> Option<ConfigBuilder> {
    START.call_once(|| {
        setup_logging();
    });

    let res = format::deserialize(config_str, config::Format::Json);
    if res.is_err() {
        error!("deserialize error {:?}", res.unwrap_err());
        return None;
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

    info!("config constructed via config builder");
    Some(config_builder)
}

impl TopologyController {
    pub fn new() -> Self {
        Self {
            vector_thread_join_handle: None,
            generation_id: Arc::new(AtomicU32::new(0)),
            topology: Arc::new(Mutex::new(None)),
            config_builder: Arc::new(Mutex::new(None)),
            rt: Arc::new(runtime()),
        }
    }

    // run a topology with tokio runtime
    pub fn start(&mut self, topology_config: &str) -> Result<bool, String> {

        let builder = init_config(topology_config);
        if builder.is_none() {
            return Err("failed to init topology config".to_string());
        }
        info!("start vector service");

        let config_builder = builder.unwrap();
        *self.config_builder.lock().unwrap() = Some(config_builder.clone());
        let topology_cp = self.topology.clone();
        let config = config_builder.build().unwrap();
        info!("config constructed via config builder");

        let rt = self.rt.clone();

        let join_handle = std::thread::spawn(move || {

            rt.block_on(async {
                let (topology, _crash) =
                    vector::test_util::start_topology(config, false).await;
                info!("vector topology started");
                *topology_cp.lock().unwrap() = Some(topology);
                // no need to handle source_finished here
                let mut sources_finished = topology_cp.lock().unwrap().as_ref().unwrap().sources_finished();
            });
        });

        info!("vector thread spawned");
        advance_generation(true, &self.generation_id);
        // self.vector_thread_join_handle = Some(join_handle);
        Ok(true)
    }

    pub fn add_config(&mut self, config: String) -> bool {
        let res = self.rt.block_on(self.handle_config_event("add".to_string(), vec![], config));
        advance_generation(res, &self.generation_id)
    }

    pub fn delete_config(&mut self, config_ids: Vec<String>) -> bool {
        let res = self.rt.block_on(self.handle_config_event("delete".to_string(), config_ids, "".to_string()));
        advance_generation(res, &self.generation_id)
    }

    pub fn update_config(&mut self, config: String) -> bool {
        let res = self.rt.block_on(self.handle_config_event("update".to_string(), vec![], config));
        advance_generation(res, &self.generation_id)
    }

    pub fn exit(&mut self) -> bool {
        // no need to handle config event, stop topology directly.
        let res = self.stop();
        advance_generation(res, &self.generation_id)
    }

    pub fn stop(&mut self) -> bool {
        // avoid double stop
        if self.topology.lock().unwrap().as_ref().is_none() {
            return true;
        }
        // here we need to enter runtime context explicitly, or there will be tokio timer panic in
        // the topology stop method, it's weird.
        let _guard = self.rt.enter();
        self.rt.block_on(self.topology.lock().unwrap().take().unwrap().stop());
        *self.topology.lock().unwrap() = None;

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


    async fn handle_config_event(&self, action: String, ids: Vec<String>, config_str: String) -> bool {
        let get_action = |action| match action {
            "init" => ConfigAction::INIT,
            "add" => ConfigAction::ADD,
            "update" => ConfigAction::UPDATE,
            "delete" => ConfigAction::DELETE,
            "exit" => ConfigAction::EXIT,
            _ => ConfigAction::INIT,
        };
        info!(
            "about to handle vector config event action={:?}",
            action.as_str()
        );
        debug!(
            "handling config event: action={:?} config={:?}",
            action.as_str(),
            config_str
        );

        reload_vector(ConfigEvent {
            action: get_action(action.as_str()),
            config_ids: ids,
            config_str,
        }, self.config_builder.lock().unwrap().as_mut().unwrap(), self.topology.lock().unwrap().as_mut().unwrap()).await
    }
}

impl OneShotTopologyController {
    pub fn new() -> Self {
        Self {
            rt: Arc::new(runtime()),
        }
    }

    // run topology and return after finished, no need to maintain datas for long run
    pub fn start(&mut self, config_str: &str) -> Result<bool, String> {
        let config_builder = init_config(config_str);
        if config_builder.is_none() {
            return Err("failed to init topology config".to_string());
        }
        info!("start one time vector topology");

        let config = config_builder.unwrap().build().unwrap();
        info!("config constructed via config builder");

        let (res, err) = self.rt.block_on(async {
            let (res, err) = start_topology_sync(config, true).await;
            (res, err)
        });
        if res {
            Ok(true)
        } else {
            Err(err)
        }
    }
}

// this function start topology and waiting for source finished
pub async fn start_topology_sync(
    mut config: Config,
    require_healthy: impl Into<Option<bool>>,
) -> (bool, String) {
    config.healthchecks.set_require_healthy(require_healthy);
    let diff = ConfigDiff::initial(&config);
    let pieces = vector::topology::build_or_log_errors(&config, &diff, HashMap::new())
        .await
        .unwrap();
    let result = vector::topology::start_validated(config, diff, pieces).await;
    if result.is_none() {
        return (false, "health check for sink failed".to_string());
    }

    let (topology, _crash) = result.unwrap();
    topology.sources_finished().await;
    topology.stop().await;
    (true, String::new())
}
