use chrono::NaiveTime;
use enum_dispatch::enum_dispatch;

#[enum_dispatch]
pub trait BaseLogEvent {
    fn timestamp(&self) -> &NaiveTime;
    fn event_name(&self) -> &String;
}
