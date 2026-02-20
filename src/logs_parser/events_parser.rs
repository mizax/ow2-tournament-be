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

#[cfg(test)]
mod tests {
    use super::{ParseError, parse_csv};
    use crate::logs_parser::event_types::LogEvent;

    #[test]
    fn parse_csv_parses_player_stat_event() {
        let csv = "[12:34:56],player_stat,,1, Red Team , PlayerOne , Tracer , 10 , 20 , 30\n";
        let events = parse_csv(csv).expect("parse_csv should succeed");
        assert_eq!(events.len(), 1);

        match &events[0] {
            LogEvent::PlayerStatEvent(event) => {
                assert_eq!(event.event_name, "player_stat");
                assert_eq!(event.round, Some(1));
                assert_eq!(event.team, "Red Team");
                assert_eq!(event.player, "PlayerOne");
                assert_eq!(event.hero, "Tracer");
                assert_eq!(event.stats, vec!["10", "20", "30"]);
                assert_eq!(event.timestamp.format("%H:%M:%S").to_string(), "12:34:56");
            }
            _ => panic!("expected PlayerStatEvent"),
        }
    }

    #[test]
    fn parse_csv_parses_round_start_event() {
        let csv = "[00:00:05],round_start,,2\n";
        let events = parse_csv(csv).expect("parse_csv should succeed");
        assert_eq!(events.len(), 1);

        match &events[0] {
            LogEvent::RoundStartEvent(event) => {
                assert_eq!(event.event_name, "round_start");
                assert_eq!(event.round, Some(2));
                assert_eq!(event.timestamp.format("%H:%M:%S").to_string(), "00:00:05");
            }
            _ => panic!("expected RoundStartEvent"),
        }
    }

    #[test]
    fn parse_csv_parses_generic_event() {
        let csv = "[01:02:03],custom_event, foo , bar\n";
        let events = parse_csv(csv).expect("parse_csv should succeed");
        assert_eq!(events.len(), 1);

        match &events[0] {
            LogEvent::GenericEvent(event) => {
                assert_eq!(event.event_name, "custom_event");
                assert_eq!(
                    event.fields,
                    vec!["[01:02:03]", "custom_event", "foo", "bar"]
                );
            }
            _ => panic!("expected GenericEvent"),
        }
    }

    #[test]
    fn parse_csv_reports_invalid_timestamp() {
        let csv = "not-a-time,player_stat\n";
        let err = parse_csv(csv).expect_err("parse_csv should fail");
        match err {
            ParseError::Timestamp(value) => assert_eq!(value, "not-a-time"),
            _ => panic!("expected Timestamp error"),
        }
    }
}
