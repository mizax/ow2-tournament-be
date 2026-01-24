use serde::Serialize;
use chrono::NaiveTime;
use super::BaseLogEvent;

#[derive(Debug, Clone, Serialize)]
pub struct PlayerStatEvent {
    pub timestamp: NaiveTime,
    pub event_name: String,
    pub round: Option<i64>,
    pub team: String,
    pub player: String,
    pub hero: String,
    pub stats: Vec<String>,
}

impl BaseLogEvent for PlayerStatEvent {
    fn timestamp(&self) -> &NaiveTime {
        &self.timestamp
    }
    fn event_name(&self) -> &String {
        &self.event_name
    }
}
