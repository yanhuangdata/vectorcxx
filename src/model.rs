use std::str;
use vector::event::LogEvent;

pub struct CxxLogEvent {
    pub log_event: LogEvent,
}

impl CxxLogEvent {
    pub fn get(&self, key: &str) -> &str {
        if self.log_event.get(key).is_none() {
            return "";
        }
        let value_ref = self.log_event.get(key).unwrap();
        let value_bytes = value_ref.as_bytes().unwrap();
        str::from_utf8(value_bytes).unwrap()
    }

    pub fn get_timestamp(&self, key: &str) -> i64 {
        let value_ref = self.log_event.get(key).unwrap();
        value_ref.as_timestamp().unwrap().timestamp_micros()
    }

    pub fn fields(&self) -> Vec<String> {
        self.log_event.keys().map(|s| s)
            .expect("log event should have some fields").collect()
    }
}