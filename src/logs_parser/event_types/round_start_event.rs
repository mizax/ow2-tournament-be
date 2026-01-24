use chrono::NaiveTime;
use serde::Serialize;

use super::BaseLogEvent;

#[derive(Debug, Clone, Serialize)]
pub struct RoundStartEvent {
    pub timestamp: NaiveTime,
    pub event_name: String,
    pub round: Option<i64>,
}

impl BaseLogEvent for RoundStartEvent {
    fn timestamp(&self) -> &NaiveTime {
        &self.timestamp
    }

    fn event_name(&self) -> &String {
        &self.event_name
    }
}
