use tracing::{debug, error, trace};
use vector::event::{EventArray, EventContainer};
use vector::sinks::memory_queue::MemoryQueueSink;
use crate::CxxLogEvent;

pub struct MemoryQueueClient {
    receiver: Option<futures::channel::mpsc::Receiver<EventArray>>,
}

impl MemoryQueueClient {
    // this new API could only be called once since there is only one receiver each time
    // C++ side should cache this object and reuse it
    pub fn new() -> Self {
        let receiver = MemoryQueueSink::take_message_receiver();
        if receiver.is_none() {
            panic!("memory queue receiver can only be taken once");
        } else {
            MemoryQueueClient {
                receiver,
            }
        }
    }

    pub fn poll(&mut self) -> Vec<CxxLogEvent> {
        let mut events: Vec<CxxLogEvent> = Vec::new();

        if let Some(rx) = &mut self.receiver {
            match rx.try_next() {
                Ok(Some(value)) => {
                    trace!("event array received length={:?}", value.len());
                    events.reserve(value.len());
                    value.iter_events().for_each(|event_ref|
                        events.push(CxxLogEvent { log_event: event_ref.into_log() })
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