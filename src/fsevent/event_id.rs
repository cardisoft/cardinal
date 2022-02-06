use crate::utils;
use fsevent_sys::FSEventsGetCurrentEventId;

/// A event id for event ordering.
pub struct EventId {
    pub since: u64,
    pub timestamp: i64,
}

impl EventId {
    // Return current event id and timestamp.
    pub fn now() -> Self {
        let since = unsafe { FSEventsGetCurrentEventId() };
        let timestamp = utils::current_timestamp();
        Self { since, timestamp }
    }
}
