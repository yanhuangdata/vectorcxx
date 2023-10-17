use tracing::trace;
use vector::event::{EventArray, EventContainer, LogEvent, Value, EventMetadata, EventRef};
use vector::sinks::memory_queue::{MemoryQueueSink, MemoryQueueConfig};
use vector::sinks::VectorSink;
use vector::transforms::metric_to_log::{MetricToLog, MetricToLogConfig};
use vector::LogNamespace;
use vector::test_util::{random_events_with_stream, random_string};
use futures::executor::block_on;
use crate::CxxLogEvent;
use futures::{stream, Stream, StreamExt};
use std::collections::BTreeMap;

pub struct MemoryQueueClient {
    receiver: Option<futures::channel::mpsc::Receiver<EventArray>>,
    metric_to_log: MetricToLog,
}


fn random_json_events(
    len: usize,
    count: usize,
) -> Vec<LogEvent> {
    let key_len = 10;
    let val_len = 40;
    let field_cnt = len / (key_len + val_len);

    let events = (0..count)
    .map(|_| {
        let mut fields = BTreeMap::new();

        for _idx in 0..field_cnt {
            fields.insert(random_string(key_len), Value::from(random_string(val_len)));
        }
        let mut log_event = LogEvent::from_map(fields.clone(), EventMetadata::default());
        log_event.insert("_message", Value::from(fields).to_string_lossy());
        log_event.insert("_datatype", "json");
        log_event.insert("_target_table", "table_a");
        log_event
    })
    .collect::<Vec<_>>();
    events
}

fn random_simple_events(
    len: usize,
    count: usize
) -> Vec<LogEvent> {
    let (init_events, _) = random_events_with_stream(len, count, None);
    let events = init_events.into_iter().map(|event| {
        let mut ev = event.into_log();
        ev.insert("_target_table", "table_a");
        ev
    }).collect::<Vec<_>>();
    events
}

fn random_batches_with_stream(
    len: usize,
    count: usize,
    chunk_size: usize,
    is_json: bool
) -> (Vec<LogEvent>, impl Stream<Item = EventArray>) {
    let events = if is_json {
        random_json_events(len, count)
    } else {
        random_simple_events(len, count)
    };

    let chunks: Vec<Vec<LogEvent>> = events.chunks(chunk_size).map(|s| s.into()).collect();
    let stream = stream::iter(chunks.clone()).map(|chunk| EventArray::from(chunk));
    (events, stream)

}

impl MemoryQueueClient {
    // this new API could only be called once since there is only one receiver each time
    // C++ side should cache this object and reuse it
    pub fn new() -> Self {
        let receiver = MemoryQueueSink::take_message_receiver();
        let metric_config: MetricToLogConfig = Default::default();
        let metric_to_log = MetricToLog::new(
            metric_config.host_tag.as_deref(),
            metric_config.timezone.unwrap_or_default(),
            LogNamespace::Vector,
            metric_config.metric_tag_values,
        );
        if receiver.is_none() {
            panic!("memory queue receiver can only be taken once");
        } else {
            MemoryQueueClient {
                receiver,
                metric_to_log
            }
        }
    }

    pub fn new_with_random_events(queue_size: usize, events_count: usize, event_len: usize, batch_size: usize, is_json: bool) -> Self {
        let config = MemoryQueueConfig {
            rate: None,
            acknowledgements: Default::default(),
            queue_size: Some(queue_size)
        };

        let sink = MemoryQueueSink::new(config);
        let receiver = MemoryQueueSink::take_message_receiver();
        let stream_sink = VectorSink::Stream(Box::new(sink));
        let (_input_lines, events) = 
            random_batches_with_stream(event_len, events_count, batch_size, is_json);
        let _ = block_on(stream_sink.run(Box::pin(events)));
        let metric_config: MetricToLogConfig = Default::default();
        let metric_to_log = MetricToLog::new(
            metric_config.host_tag.as_deref(),
            metric_config.timezone.unwrap_or_default(),
            LogNamespace::Legacy,
            metric_config.metric_tag_values,
        );
        MemoryQueueClient { receiver, metric_to_log }
    }

    pub fn poll(&mut self) -> Vec<CxxLogEvent> {
        let mut events: Vec<CxxLogEvent> = Vec::new();

        if let Some(rx) = &mut self.receiver {
            match rx.try_next() {
                Ok(Some(value)) => {
                    events.reserve(value.len());
                    value.iter_events().for_each(|event_ref| {
                        match event_ref
                        {
                            EventRef::Log(log) => events.push(CxxLogEvent { log_event: log.clone() }),
                            EventRef::Metric(metric) => events.push(CxxLogEvent { log_event: self.metric_to_log.transform_one(metric.clone()).unwrap() }),
                            _ => ()
                        }
                    }
                    );
                }
                Ok(None) => {}
                Err(e) => {
                    trace!("failed to poll events: error={:?}", e);
                }
            }
        }

        events
    }
}

impl Drop for MemoryQueueClient {
    fn drop(&mut self) {
        if let Some(rx) = self.receiver.take() {
            MemoryQueueSink::set_message_receiver(rx);
        }
    }
}