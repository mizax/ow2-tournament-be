use chrono::NaiveTime;

use crate::logs_parser::event_types::{GenericEvent, LogEvent, PlayerStatEvent, RoundStartEvent};

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("csv parse error: {0}")]
    Csv(#[from] csv::Error),
    #[error("invalid timestamp: {0}")]
    Timestamp(String),
}

pub fn parse_csv(csv: &str) -> Result<Vec<LogEvent>, ParseError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(csv.as_bytes());

    let mut events = Vec::new();
    for record in reader.records() {
        let record = record?;
        events.push(parse_record(&record)?);
    }

    Ok(events)
}

fn parse_record(record: &csv::StringRecord) -> Result<LogEvent, ParseError> {
    let timestamp = parse_timestamp(get(record, 0))?;
    let kind = get(record, 1);

    Ok(match kind.as_str() {
        "player_stat" => handle_player_stat(record, timestamp),
        "round_start" => handle_round_start(record, timestamp),
        _ => handle_generic(record, timestamp, kind),
    })
}

fn handle_player_stat(record: &csv::StringRecord, timestamp: NaiveTime) -> LogEvent {
    let stats = record
        .iter()
        .skip(7)
        .map(|v| v.trim().to_string())
        .collect();

    LogEvent::PlayerStatEvent(PlayerStatEvent {
        timestamp,
        event_name: "player_stat".to_string(),
        round: record.get(3).and_then(|v| v.trim().parse::<i64>().ok()),
        team: get(record, 4),
        player: get(record, 5),
        hero: get(record, 6),
        stats,
    })
}

fn handle_round_start(record: &csv::StringRecord, timestamp: NaiveTime) -> LogEvent {
    LogEvent::RoundStartEvent(RoundStartEvent {
        timestamp,
        event_name: "round_start".to_string(),
        round: record.get(3).and_then(|v| v.trim().parse::<i64>().ok()),
    })
}

fn handle_generic(record: &csv::StringRecord, timestamp: NaiveTime, kind: String) -> LogEvent {
    LogEvent::GenericEvent(GenericEvent {
        timestamp,
        event_name: kind,
        fields: record.iter().map(|v| v.trim().to_string()).collect(),
    })
}

fn parse_timestamp(value: String) -> Result<NaiveTime, ParseError> {
    let raw = value.trim().trim_start_matches('[').trim_end_matches(']');
    NaiveTime::parse_from_str(raw, "%H:%M:%S").map_err(|_| ParseError::Timestamp(value))
}

fn get(record: &csv::StringRecord, index: usize) -> String {
    record.get(index).unwrap_or("").trim().to_string()
}
