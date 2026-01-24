use chrono::NaiveTime;
use serde::Serialize;

use super::BaseLogEvent;

#[derive(Debug, Clone, Serialize)]
pub struct GenericEvent {
    pub timestamp: NaiveTime,
    pub event_name: String,
    pub fields: Vec<String>,
}

impl BaseLogEvent for GenericEvent {
    fn timestamp(&self) -> &NaiveTime {
        &self.timestamp
    }

    fn event_name(&self) -> &String {
        &self.event_name
    }
}
