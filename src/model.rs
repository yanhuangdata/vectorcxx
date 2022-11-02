use std::str;
use vector::event::LogEvent;
use vector::event_path;

pub struct CxxLogEvent {
    pub log_event: LogEvent,
}

impl CxxLogEvent {
    pub fn get_string(&self, key: &str) -> &str {
        if self.log_event.get(key).is_some() {
            let value_ref = self.log_event.get(key).unwrap();
            return str::from_utf8(value_ref.as_bytes().unwrap()).unwrap();
        } else if self.log_event.get(event_path!(key)).is_some() {
            let value_ref = self.log_event.get(event_path!(key)).unwrap();
            return str::from_utf8(value_ref.as_bytes().unwrap()).unwrap();
        }
        ""
    }

    pub fn get_object_as_string(&self, key: &str) -> String {
        if self.log_event.get(key).is_some() {
            return self.log_event.get(key).unwrap().to_string_lossy();
        }
        self.log_event.get(event_path!(key)).unwrap().to_string_lossy()
    }

    pub fn get_array_as_string(&self, key: &str) -> String {
        if self.log_event.get(key).is_some() {
            return self.log_event.get(key).unwrap().to_string_lossy();
        }
        self.log_event.get(event_path!(key)).unwrap().to_string_lossy()
    }

    pub fn get_value_type(&self, key: &str) -> &str {
        if self.log_event.get(key).is_some() {
            return self.log_event.get(key).unwrap().kind_str();
        } else if self.log_event.get(event_path!(key)).is_some() {
            return self.log_event.get(event_path!(key)).unwrap().kind_str();
        }
        ""
    }

    // notice that vector value only has i64, not u64
    pub fn get_integer(&self, key: &str) -> i64 {
        if self.log_event.get(key).is_some() {
            return self.log_event.get(key).unwrap().as_integer().unwrap();
        }
        return self.log_event.get(event_path!(key)).unwrap().as_integer().unwrap();
    }

    pub fn get_boolean(&self, key: &str) -> bool {
        if self.log_event.get(key).is_some() {
            return self.log_event.get(key).unwrap().as_boolean().unwrap();
        }
        self.log_event.get(event_path!(key)).unwrap().as_boolean().unwrap()
    }

    // not float 32, just float 64 as double
    pub fn get_double(&self, key: &str) -> f64 {
        if self.log_event.get(key).is_some() {
            return self.log_event.get(key).unwrap().as_float().unwrap().into_inner();
        }
        self.log_event.get(event_path!(key)).unwrap().as_float().unwrap().into_inner()
    }

    pub fn get_timestamp(&self, key: &str) -> i64 {
        if self.log_event.get(key).is_some() {
            let value_ref = self.log_event.get(key).unwrap();
            return value_ref.as_timestamp().unwrap().timestamp_micros();
        }
        let value_ref = self.log_event.get(event_path!(key)).unwrap();
        value_ref.as_timestamp().unwrap().timestamp_micros()
    }

    pub fn fields(&self) -> Vec<String> {
        self.log_event.keys().map(|s| s)
            .expect("log event should have some fields").collect()
    }
}