use std::fmt::{Debug, Display};

#[derive(Debug)]
pub enum ConfigAction {
    INIT,
    ADD,
    UPDATE,
    DELETE,
    EXIT,
}

impl Display for ConfigAction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

pub struct ConfigEvent {
    pub action: ConfigAction,
    pub config_ids: Vec<String>,
    pub config_str: String,
}

impl Debug for ConfigEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("ConfigEvent")
            .field("action", &self.action)
            .field("config_ids", &self.config_ids)
            .field("config_str", &self.config_str)
            .finish()
    }
}
