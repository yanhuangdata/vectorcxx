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
            if value_ref.is_bytes() {
                return str::from_utf8(value_ref.as_bytes().unwrap()).unwrap();
            }
        } else if self.log_event.get(event_path!(key)).is_some() {
            let value_ref = self.log_event.get(event_path!(key)).unwrap();
            if value_ref.is_bytes() {
                return str::from_utf8(value_ref.as_bytes().unwrap()).unwrap();
            }
        }
        ""
    }

    pub fn get_object_as_string(&self, key: &str) -> String {
        if self.log_event.get(key).is_some() {
            return self.log_event.get(key).unwrap().to_string_lossy();
        }
        self.log_event.get(event_path!(key)).unwrap().to_string_lossy()
    }

    /* get array type field value in event, the whole array are dumped as string
     */
    pub fn get_array_as_string(&self, key: &str) -> String {
        if self.log_event.get(key).is_some() {
            return self.log_event.get(key).unwrap().to_string_lossy();
        }
        self.log_event.get(event_path!(key)).unwrap().to_string_lossy()
    }

    /* get array type field value in event, returning an array, child values in array
    will all be converted to strings.
     */
    pub fn get_string_array(&self, key: &str) -> Vec<String> {
        let value = if self.log_event.get(key).is_some() 
                            {self.log_event.get(key).unwrap().as_array()} 
                            else if self.log_event.get(event_path!(key)).is_some()
                            {self.log_event.get(event_path!(key)).unwrap().as_array()}
                            else {None};
        if value.is_some() {
            let array = value.unwrap();
            return array.iter().map(|v|v.to_string_lossy()).collect();
            // return value.unwrap().as_array().unwrap().map(|v|v.to_string_lossy()).collect();
        }
        Vec::new()
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

    /*
    Return all fields of an event.
    Vector uses a depth-first logic to construct and traverse fields, event like
    '''
    {
        "a": "a_val",
        "b": [1, 2],
        "c": {
            "d": "d_val"
        }
    }
    '''
    will return field keys: ["a", "b[0]", "b[1]", "c.d"] from this method
    */
    pub fn fields(&self) -> Vec<String> {
        self.log_event.keys()
            .unwrap()
            .map(|key| if key.contains("\\.") {key.replace("\\.", ".")} else {key})
            .collect()
    }

    /*
    Return the top level field keys of event
    Vector uses a depth-first logic to construct and traverse fields, event like
    '''
    {
        "a": "a_val",
        "b": [1, 2],
        "c": {
            "d": "d_val"
        }
    }
    '''
    will return field keys: ["a", "b", "c"] from this method
    */
    pub fn top_level_fields(&self) -> Vec<String> {
        match &self.log_event.as_map() {
            Some(map) => map.keys().map(|key| if key.contains("\\.") {key.replace("\\.", ".")} else {key.clone()}).collect(),
            None => Vec::new(),
        }
    }
}