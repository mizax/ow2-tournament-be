use super::{BaseLogEvent, GenericEvent, PlayerStatEvent, RoundStartEvent};
use chrono::NaiveTime;
use enum_dispatch::enum_dispatch;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "payload")]
#[enum_dispatch(BaseLogEvent)]
pub enum LogEvent {
    PlayerStatEvent(PlayerStatEvent),
    RoundStartEvent(RoundStartEvent),
    GenericEvent(GenericEvent),
}
